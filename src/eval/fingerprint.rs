use blake2::{Blake2b512, Digest};
use serde::{Deserialize, Serialize};

use crate::ai::combat_state_key::{
    combat_exact_state_key, stable_dominance_bucket_key, stable_outcome_key,
};
use crate::content::cards::java_id;
use crate::content::monsters::EnemyId;
use crate::runtime::combat::CombatState;
use crate::runtime::rng::{RngPool, StsRng};
use crate::sim::combat::{combat_terminal, stable_boundary, CombatPosition, CombatTerminal};
use crate::sim::combat_legal_actions::get_legal_moves;
use crate::state::core::{ClientInput, EngineState};

pub const FINGERPRINT_SCHEMA_NAME: &str = "StateFingerprintV1";
pub const FINGERPRINT_SCHEMA_VERSION: u32 = 1;
pub const FINGERPRINT_ALGORITHM_JSON: &str = "blake2b_256_canonical_json_v1";
pub const FINGERPRINT_ALGORITHM_DEBUG: &str = "blake2b_256_of_typed_key_debug_v1";

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct StateFingerprintV1 {
    pub schema_name: String,
    pub schema_version: u32,
    pub fingerprint_algorithm: String,
    pub boundary: DecisionBoundaryFingerprintV1,
    pub public_observation_hash: String,
    pub legal_candidate_set_hash: String,
    pub legal_candidate_order_hash: String,
    pub exact_state_hash: String,
    pub stable_outcome_hash: Option<String>,
    pub rng_boundary: RngBoundaryFingerprintV1,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DecisionBoundaryFingerprintV1 {
    pub engine_state: String,
    pub decision_kind: String,
    pub terminal: CombatTerminal,
    pub stable_boundary: bool,
    pub turn_count: u32,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RngBoundaryFingerprintV1 {
    pub status: RngFingerprintStatus,
    pub stream_count: usize,
    pub digest: String,
    pub streams: Vec<RngStreamFingerprintV1>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RngFingerprintStatus {
    Complete,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RngStreamFingerprintV1 {
    pub name: String,
    pub counter: u32,
    pub state_hash: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CombatLegalActionSetFingerprintV1 {
    pub fingerprint_algorithm: String,
    pub count: usize,
    pub candidate_set_hash: String,
    pub candidate_order_hash: String,
    pub descriptors: Vec<CombatActionFingerprintDescriptorV1>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CombatActionFingerprintDescriptorV1 {
    pub kind: String,
    pub stable_key: String,
    pub input: ClientInput,
    pub subject: Option<ActionSubjectFingerprintV1>,
    pub target: Option<ActionTargetFingerprintV1>,
    pub indices: Vec<usize>,
    pub uuids: Vec<u32>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ActionSubjectFingerprintV1 {
    pub kind: String,
    pub index: Option<usize>,
    pub uuid: Option<u32>,
    pub id: Option<String>,
    pub upgrades: Option<u8>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ActionTargetFingerprintV1 {
    pub kind: String,
    pub entity_id: usize,
    pub slot: Option<u8>,
    pub id: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
struct CombatPublicObservationFingerprintInputV1 {
    boundary: DecisionBoundaryFingerprintV1,
    player: PlayerObservationFingerprintV1,
    hand: Vec<CardObservationFingerprintV1>,
    piles: PileObservationFingerprintV1,
    potions: Vec<Option<PotionObservationFingerprintV1>>,
    monsters: Vec<MonsterObservationFingerprintV1>,
}

#[derive(Clone, Debug, Serialize)]
struct PlayerObservationFingerprintV1 {
    player_class: String,
    ascension_level: u8,
    hp: i32,
    max_hp: i32,
    block: i32,
    energy: u8,
}

#[derive(Clone, Debug, Serialize)]
struct CardObservationFingerprintV1 {
    uuid: u32,
    card_id: String,
    upgrades: u8,
    cost_for_turn: i32,
}

#[derive(Clone, Debug, Serialize)]
struct PileObservationFingerprintV1 {
    draw_count: usize,
    discard_count: usize,
    exhaust_count: usize,
    limbo_count: usize,
    queued_cards_count: usize,
}

#[derive(Clone, Debug, Serialize)]
struct PotionObservationFingerprintV1 {
    uuid: u32,
    potion_id: String,
    can_use: bool,
    can_discard: bool,
    requires_target: bool,
}

#[derive(Clone, Debug, Serialize)]
struct MonsterObservationFingerprintV1 {
    slot: u8,
    entity_id: usize,
    enemy_id: String,
    hp: i32,
    max_hp: i32,
    block: i32,
    alive: bool,
    escaped: bool,
    dying: bool,
    half_dead: bool,
    planned_move_id: u8,
    visible_intent: String,
    preview_damage_per_hit: i32,
}

pub fn combat_state_fingerprint_v1(position: &CombatPosition) -> StateFingerprintV1 {
    let legal_actions = combat_legal_action_set_fingerprint_v1(&position.engine, &position.combat);
    let exact = combat_exact_state_key(&position.engine, &position.combat);
    let stable = stable_dominance_bucket_key(&position.engine, &position.combat)
        .map(|_| stable_outcome_key(&position.engine, &position.combat));
    StateFingerprintV1 {
        schema_name: FINGERPRINT_SCHEMA_NAME.to_string(),
        schema_version: FINGERPRINT_SCHEMA_VERSION,
        fingerprint_algorithm: FINGERPRINT_ALGORITHM_JSON.to_string(),
        boundary: boundary_fingerprint(&position.engine, &position.combat),
        public_observation_hash: hash_serializable(&public_observation_input(position)),
        legal_candidate_set_hash: legal_actions.candidate_set_hash,
        legal_candidate_order_hash: legal_actions.candidate_order_hash,
        exact_state_hash: hash_debug(&exact),
        stable_outcome_hash: stable.as_ref().map(hash_debug),
        rng_boundary: rng_boundary_fingerprint_v1(&position.combat.rng.pool),
    }
}

pub fn combat_legal_action_set_fingerprint_v1(
    engine: &EngineState,
    combat: &CombatState,
) -> CombatLegalActionSetFingerprintV1 {
    let descriptors = get_legal_moves(engine, combat)
        .into_iter()
        .map(|input| combat_action_descriptor_v1(combat, input))
        .collect::<Vec<_>>();
    let mut sorted = descriptors.clone();
    sorted.sort_by(|a, b| {
        a.stable_key
            .cmp(&b.stable_key)
            .then_with(|| hash_serializable(a).cmp(&hash_serializable(b)))
    });
    CombatLegalActionSetFingerprintV1 {
        fingerprint_algorithm: FINGERPRINT_ALGORITHM_JSON.to_string(),
        count: descriptors.len(),
        candidate_set_hash: hash_serializable(&sorted),
        candidate_order_hash: hash_serializable(&descriptors),
        descriptors,
    }
}

pub fn hash_debug<T: std::fmt::Debug>(value: &T) -> String {
    hash_bytes(format!("{value:?}").as_bytes())
}

fn hash_serializable<T: Serialize>(value: &T) -> String {
    let payload =
        serde_json::to_vec(value).expect("fingerprint input should serialize deterministically");
    hash_bytes(&payload)
}

fn hash_bytes(bytes: &[u8]) -> String {
    let mut hasher = Blake2b512::new();
    hasher.update(bytes);
    let digest = hasher.finalize();
    hex_lower(&digest[..32])
}

fn boundary_fingerprint(
    engine: &EngineState,
    combat: &CombatState,
) -> DecisionBoundaryFingerprintV1 {
    DecisionBoundaryFingerprintV1 {
        engine_state: format!("{engine:?}"),
        decision_kind: decision_kind(engine),
        terminal: combat_terminal(engine, combat),
        stable_boundary: stable_boundary(engine, combat),
        turn_count: combat.turn.turn_count,
    }
}

fn decision_kind(engine: &EngineState) -> String {
    match engine {
        EngineState::CombatPlayerTurn => "combat_player_action".to_string(),
        EngineState::PendingChoice(choice) => format!("combat_pending_choice:{choice:?}"),
        EngineState::CombatProcessing => "combat_processing".to_string(),
        EngineState::CombatStart(request) => format!("combat_start:{:?}", request.encounter_id),
        EngineState::RewardScreen(_) => "reward_screen".to_string(),
        EngineState::RewardOverlay { .. } => "reward_overlay".to_string(),
        EngineState::TreasureRoom(_) => "treasure_room".to_string(),
        EngineState::Campfire => "campfire".to_string(),
        EngineState::Shop(_) => "shop".to_string(),
        EngineState::MapNavigation => "map_choice".to_string(),
        EngineState::MapOverlay { .. } => "map_overlay_choice".to_string(),
        EngineState::EventRoom => "event_choice".to_string(),
        EngineState::RunPendingChoice(choice) => format!("run_pending_choice:{:?}", choice.reason),
        EngineState::BossRelicSelect(_) => "boss_relic_choice".to_string(),
        EngineState::GameOver(result) => format!("game_over:{result:?}"),
    }
}

fn public_observation_input(
    position: &CombatPosition,
) -> CombatPublicObservationFingerprintInputV1 {
    let combat = &position.combat;
    CombatPublicObservationFingerprintInputV1 {
        boundary: boundary_fingerprint(&position.engine, combat),
        player: PlayerObservationFingerprintV1 {
            player_class: combat.meta.player_class.clone(),
            ascension_level: combat.meta.ascension_level,
            hp: combat.entities.player.current_hp,
            max_hp: combat.entities.player.max_hp,
            block: combat.entities.player.block,
            energy: combat.turn.energy,
        },
        hand: combat
            .zones
            .hand
            .iter()
            .map(|card| CardObservationFingerprintV1 {
                uuid: card.uuid,
                card_id: java_id(card.id).to_string(),
                upgrades: card.upgrades,
                cost_for_turn: card.cost_for_turn_java(),
            })
            .collect(),
        piles: PileObservationFingerprintV1 {
            draw_count: combat.zones.draw_pile.len(),
            discard_count: combat.zones.discard_pile.len(),
            exhaust_count: combat.zones.exhaust_pile.len(),
            limbo_count: combat.zones.limbo.len(),
            queued_cards_count: combat.zones.queued_cards.len(),
        },
        potions: combat
            .entities
            .potions
            .iter()
            .map(|slot| {
                slot.as_ref().map(|potion| PotionObservationFingerprintV1 {
                    uuid: potion.uuid,
                    potion_id: format!("{:?}", potion.id),
                    can_use: potion.can_use,
                    can_discard: potion.can_discard,
                    requires_target: potion.requires_target,
                })
            })
            .collect(),
        monsters: combat
            .entities
            .monsters
            .iter()
            .map(|monster| {
                let observation = combat
                    .runtime
                    .monster_protocol
                    .get(&monster.id)
                    .map(|protocol| &protocol.observation);
                let turn_plan = monster.turn_plan();
                MonsterObservationFingerprintV1 {
                    slot: monster.slot,
                    entity_id: monster.id,
                    enemy_id: EnemyId::from_id(monster.monster_type)
                        .map(|enemy| format!("{enemy:?}"))
                        .unwrap_or_else(|| format!("monster_type:{}", monster.monster_type)),
                    hp: monster.current_hp,
                    max_hp: monster.max_hp,
                    block: monster.block,
                    alive: monster.is_alive_for_action(),
                    escaped: monster.is_escaped,
                    dying: monster.is_dying,
                    half_dead: monster.half_dead,
                    planned_move_id: monster.planned_move_id(),
                    visible_intent: observation
                        .filter(|obs| obs.visible_intent != crate::runtime::combat::Intent::Unknown)
                        .map(|obs| format!("{:?}", obs.visible_intent))
                        .unwrap_or_else(|| format!("{:?}", turn_plan.summary_spec())),
                    preview_damage_per_hit: observation
                        .filter(|obs| obs.preview_damage_per_hit > 0)
                        .map(|obs| obs.preview_damage_per_hit)
                        .or_else(|| turn_plan.attack().map(|attack| attack.base_damage))
                        .unwrap_or(0),
                }
            })
            .collect(),
    }
}

fn combat_action_descriptor_v1(
    combat: &CombatState,
    input: ClientInput,
) -> CombatActionFingerprintDescriptorV1 {
    match &input {
        ClientInput::PlayCard { card_index, target } => {
            let subject =
                combat
                    .zones
                    .hand
                    .get(*card_index)
                    .map(|card| ActionSubjectFingerprintV1 {
                        kind: "hand_card".to_string(),
                        index: Some(*card_index),
                        uuid: Some(card.uuid),
                        id: Some(java_id(card.id).to_string()),
                        upgrades: Some(card.upgrades),
                    });
            let target = target.and_then(|id| monster_target(combat, id));
            descriptor(
                "play_card",
                stable_key("play_card", subject.as_ref(), target.as_ref(), &[], &[]),
                input.clone(),
                subject,
                target,
                Vec::new(),
                Vec::new(),
            )
        }
        ClientInput::UsePotion {
            potion_index,
            target,
        } => {
            let subject = combat
                .entities
                .potions
                .get(*potion_index)
                .and_then(|slot| slot.as_ref())
                .map(|potion| ActionSubjectFingerprintV1 {
                    kind: "potion".to_string(),
                    index: Some(*potion_index),
                    uuid: Some(potion.uuid),
                    id: Some(format!("{:?}", potion.id)),
                    upgrades: None,
                });
            let target = target.and_then(|id| monster_target(combat, id));
            descriptor(
                "use_potion",
                stable_key("use_potion", subject.as_ref(), target.as_ref(), &[], &[]),
                input.clone(),
                subject,
                target,
                Vec::new(),
                Vec::new(),
            )
        }
        ClientInput::DiscardPotion(slot) => {
            let subject = combat
                .entities
                .potions
                .get(*slot)
                .and_then(|slot| slot.as_ref())
                .map(|potion| ActionSubjectFingerprintV1 {
                    kind: "potion".to_string(),
                    index: Some(*slot),
                    uuid: Some(potion.uuid),
                    id: Some(format!("{:?}", potion.id)),
                    upgrades: None,
                });
            descriptor(
                "discard_potion",
                stable_key("discard_potion", subject.as_ref(), None, &[], &[]),
                input.clone(),
                subject,
                None,
                Vec::new(),
                Vec::new(),
            )
        }
        ClientInput::EndTurn => descriptor(
            "end_turn",
            "end_turn".to_string(),
            input.clone(),
            None,
            None,
            Vec::new(),
            Vec::new(),
        ),
        ClientInput::SubmitCardChoice(indices) => selection_descriptor(
            "submit_card_choice",
            input.clone(),
            indices.clone(),
            Vec::new(),
        ),
        ClientInput::SubmitDiscoverChoice(index) => selection_descriptor(
            "submit_discover_choice",
            input.clone(),
            vec![*index],
            Vec::new(),
        ),
        ClientInput::SubmitScryDiscard(indices) => selection_descriptor(
            "submit_scry_discard",
            input.clone(),
            indices.clone(),
            Vec::new(),
        ),
        ClientInput::SubmitHandSelect(uuids) => selection_descriptor(
            "submit_hand_select",
            input.clone(),
            Vec::new(),
            uuids.clone(),
        ),
        ClientInput::SubmitGridSelect(uuids) => selection_descriptor(
            "submit_grid_select",
            input.clone(),
            Vec::new(),
            uuids.clone(),
        ),
        ClientInput::Proceed => descriptor(
            "proceed",
            "proceed".to_string(),
            input.clone(),
            None,
            None,
            Vec::new(),
            Vec::new(),
        ),
        ClientInput::Cancel => descriptor(
            "cancel",
            "cancel".to_string(),
            input.clone(),
            None,
            None,
            Vec::new(),
            Vec::new(),
        ),
        other => descriptor(
            "run_or_noncombat_input",
            format!("run_or_noncombat_input:{other:?}"),
            input.clone(),
            None,
            None,
            Vec::new(),
            Vec::new(),
        ),
    }
}

fn selection_descriptor(
    kind: &str,
    input: ClientInput,
    indices: Vec<usize>,
    uuids: Vec<u32>,
) -> CombatActionFingerprintDescriptorV1 {
    descriptor(
        kind,
        stable_key(kind, None, None, &indices, &uuids),
        input,
        None,
        None,
        indices,
        uuids,
    )
}

fn descriptor(
    kind: &str,
    stable_key: String,
    input: ClientInput,
    subject: Option<ActionSubjectFingerprintV1>,
    target: Option<ActionTargetFingerprintV1>,
    indices: Vec<usize>,
    uuids: Vec<u32>,
) -> CombatActionFingerprintDescriptorV1 {
    CombatActionFingerprintDescriptorV1 {
        kind: kind.to_string(),
        stable_key,
        input,
        subject,
        target,
        indices,
        uuids,
    }
}

fn monster_target(combat: &CombatState, entity_id: usize) -> Option<ActionTargetFingerprintV1> {
    combat
        .entities
        .monsters
        .iter()
        .find(|monster| monster.id == entity_id)
        .map(|monster| ActionTargetFingerprintV1 {
            kind: "monster".to_string(),
            entity_id,
            slot: Some(monster.slot),
            id: EnemyId::from_id(monster.monster_type)
                .map(|enemy| format!("{enemy:?}"))
                .or_else(|| Some(format!("monster_type:{}", monster.monster_type))),
        })
}

fn stable_key(
    kind: &str,
    subject: Option<&ActionSubjectFingerprintV1>,
    target: Option<&ActionTargetFingerprintV1>,
    indices: &[usize],
    uuids: &[u32],
) -> String {
    let subject = subject
        .map(|subject| {
            format!(
                "{}:{}:{}:{}",
                subject.kind,
                subject
                    .uuid
                    .map(|uuid| uuid.to_string())
                    .unwrap_or_else(|| "-".to_string()),
                subject.id.as_deref().unwrap_or("-"),
                subject
                    .index
                    .map(|index| index.to_string())
                    .unwrap_or_else(|| "-".to_string())
            )
        })
        .unwrap_or_else(|| "subject:none".to_string());
    let target = target
        .map(|target| {
            format!(
                "{}:{}:{}:{}",
                target.kind,
                target.entity_id,
                target
                    .slot
                    .map(|slot| slot.to_string())
                    .unwrap_or_else(|| "-".to_string()),
                target.id.as_deref().unwrap_or("-")
            )
        })
        .unwrap_or_else(|| "target:none".to_string());
    format!("{kind}/{subject}/{target}/indices:{indices:?}/uuids:{uuids:?}")
}

fn rng_boundary_fingerprint_v1(pool: &RngPool) -> RngBoundaryFingerprintV1 {
    let streams = vec![
        rng_stream("monster_rng", &pool.monster_rng),
        rng_stream("event_rng", &pool.event_rng),
        rng_stream("merchant_rng", &pool.merchant_rng),
        rng_stream("card_rng", &pool.card_rng),
        rng_stream("treasure_rng", &pool.treasure_rng),
        rng_stream("relic_rng", &pool.relic_rng),
        rng_stream("potion_rng", &pool.potion_rng),
        rng_stream("monster_hp_rng", &pool.monster_hp_rng),
        rng_stream("ai_rng", &pool.ai_rng),
        rng_stream("shuffle_rng", &pool.shuffle_rng),
        rng_stream("card_random_rng", &pool.card_random_rng),
        rng_stream("misc_rng", &pool.misc_rng),
        rng_stream("math_rng", &pool.math_rng),
    ];
    RngBoundaryFingerprintV1 {
        status: RngFingerprintStatus::Complete,
        stream_count: streams.len(),
        digest: hash_serializable(&streams),
        streams,
    }
}

fn rng_stream(name: &str, rng: &StsRng) -> RngStreamFingerprintV1 {
    RngStreamFingerprintV1 {
        name: name.to_string(),
        counter: rng.counter,
        state_hash: hash_serializable(&(rng.seed0, rng.seed1, rng.counter)),
    }
}

fn hex_lower(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        use std::fmt::Write;
        let _ = write!(&mut out, "{byte:02x}");
    }
    out
}
