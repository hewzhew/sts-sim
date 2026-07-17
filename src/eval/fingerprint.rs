use blake2::{Blake2b512, Digest};
use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::ai::combat_policy_v1::{combat_public_observation_v1, CombatPublicObservationV1};
use crate::ai::combat_state_key::{
    combat_exact_state_hash_v1, stable_dominance_bucket_key, stable_outcome_key,
};
use crate::content::cards::java_id;
use crate::content::monsters::EnemyId;
use crate::runtime::combat::CombatState;
use crate::runtime::rng::{RngPool, StsRng};
use crate::sim::combat::{combat_terminal, stable_boundary, CombatPosition, CombatTerminal};
use crate::sim::combat_action_surface::{
    combat_legal_action_surface_v2, CombatSelectionActionFamilyV2,
    CombatSelectionDomainCandidateV2, CombatSelectionInputEncodingV2,
    CombatSelectionPayloadLanguageV2, CombatSelectionStatusV2,
};
use crate::state::core::{ClientInput, EngineState};

pub const FINGERPRINT_SCHEMA_NAME: &str = "StateFingerprintV2";
pub const FINGERPRINT_SCHEMA_VERSION: u32 = 2;
pub const FINGERPRINT_ALGORITHM_JSON: &str = "blake2b_256_canonical_json_v1";
pub const FINGERPRINT_ALGORITHM_DEBUG: &str = "blake2b_256_of_typed_key_debug_v1";

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct StateFingerprintV2 {
    pub schema_name: String,
    pub schema_version: u32,
    pub fingerprint_algorithm: String,
    pub boundary: DecisionBoundaryFingerprintV2,
    pub public_observation_hash: String,
    pub legal_input_language_hash: String,
    pub action_enumeration_domain_hash: String,
    pub exact_state_hash: String,
    pub stable_outcome_hash: Option<String>,
    pub rng_boundary: RngBoundaryFingerprintV2,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DecisionBoundaryFingerprintV2 {
    pub engine_state: String,
    pub decision_kind: String,
    pub terminal: CombatTerminal,
    pub stable_boundary: bool,
    pub turn_count: u32,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RngBoundaryFingerprintV2 {
    pub status: RngFingerprintStatus,
    pub stream_count: usize,
    pub digest: String,
    pub streams: Vec<RngStreamFingerprintV2>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RngFingerprintStatus {
    Complete,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RngStreamFingerprintV2 {
    pub name: String,
    pub counter: u32,
    pub state_hash: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CombatLegalActionSurfaceFingerprintV2 {
    pub fingerprint_algorithm: String,
    pub legal_input_language_digest: String,
    pub enumeration_domain_digest: String,
    pub atomic_action_count: u64,
    pub action_family_count: u64,
    pub atomic_actions: Vec<CombatActionFingerprintDescriptorV2>,
    pub selection_families: Vec<crate::sim::combat_action_surface::CombatSelectionActionFamilyV2>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CombatActionFingerprintDescriptorV2 {
    pub kind: String,
    pub stable_key: String,
    pub input: ClientInput,
    pub subject: Option<ActionSubjectFingerprintV2>,
    pub target: Option<ActionTargetFingerprintV2>,
    pub indices: Vec<usize>,
    pub uuids: Vec<u32>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ActionSubjectFingerprintV2 {
    pub kind: String,
    pub index: Option<usize>,
    pub uuid: Option<u32>,
    pub id: Option<String>,
    pub upgrades: Option<u8>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ActionTargetFingerprintV2 {
    pub kind: String,
    pub entity_id: usize,
    pub slot: Option<u8>,
    pub id: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
struct CombatPublicObservationFingerprintInputV2 {
    boundary: DecisionBoundaryFingerprintV2,
    public: CombatPublicObservationV1,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CombatFingerprintBundleV2 {
    pub state: StateFingerprintV2,
    pub legal_action_surface: CombatLegalActionSurfaceFingerprintV2,
}

pub fn combat_fingerprint_bundle_v2(position: &CombatPosition) -> CombatFingerprintBundleV2 {
    let legal_action_surface =
        combat_legal_action_surface_fingerprint_v2(&position.engine, &position.combat);
    let stable = stable_dominance_bucket_key(&position.engine, &position.combat)
        .map(|_| stable_outcome_key(&position.engine, &position.combat));
    let state = StateFingerprintV2 {
        schema_name: FINGERPRINT_SCHEMA_NAME.to_string(),
        schema_version: FINGERPRINT_SCHEMA_VERSION,
        fingerprint_algorithm: FINGERPRINT_ALGORITHM_JSON.to_string(),
        boundary: boundary_fingerprint(&position.engine, &position.combat),
        public_observation_hash: hash_serializable(&public_observation_input(position)),
        legal_input_language_hash: legal_action_surface.legal_input_language_digest.clone(),
        action_enumeration_domain_hash: legal_action_surface.enumeration_domain_digest.clone(),
        exact_state_hash: combat_exact_state_hash_v1(&position.engine, &position.combat),
        stable_outcome_hash: stable.as_ref().map(hash_debug),
        rng_boundary: rng_boundary_fingerprint_v2(&position.combat.rng.pool),
    };
    CombatFingerprintBundleV2 {
        state,
        legal_action_surface,
    }
}

pub fn combat_state_fingerprint_v2(position: &CombatPosition) -> StateFingerprintV2 {
    combat_fingerprint_bundle_v2(position).state
}

pub fn combat_legal_action_surface_fingerprint_v2(
    engine: &EngineState,
    combat: &CombatState,
) -> CombatLegalActionSurfaceFingerprintV2 {
    let legal_surface = combat_legal_action_surface_v2(engine, combat);
    let atomic_actions = legal_surface
        .atomic_actions
        .iter()
        .cloned()
        .into_iter()
        .map(|input| combat_action_descriptor_v2(combat, input))
        .collect::<Vec<_>>();
    let legal_input_language_digest = legal_input_language_digest(&legal_surface);
    let enumeration_domain_digest =
        enumeration_domain_digest(&atomic_actions, &legal_surface.selection_families);
    CombatLegalActionSurfaceFingerprintV2 {
        fingerprint_algorithm: FINGERPRINT_ALGORITHM_JSON.to_string(),
        legal_input_language_digest,
        enumeration_domain_digest,
        atomic_action_count: u64::try_from(atomic_actions.len()).unwrap_or(u64::MAX),
        action_family_count: u64::try_from(legal_surface.selection_families.len())
            .unwrap_or(u64::MAX),
        atomic_actions,
        selection_families: legal_surface.selection_families,
    }
}

#[derive(Serialize)]
struct LegalInputLanguageDigestInputV2 {
    atomic_inputs: Vec<ClientInput>,
    selection_languages: Vec<LegalSelectionLanguageProjectionV2>,
}

#[derive(Serialize)]
#[serde(tag = "input_language", rename_all = "snake_case")]
enum LegalSelectionLanguageProjectionV2 {
    CardUuidSequence {
        input_encoding: CombatSelectionInputEncodingV2,
        eligible_uuids: Vec<u32>,
        min_selected: u64,
        max_selected: u64,
        payload_language: CombatSelectionPayloadLanguageV2,
    },
    ScryIndexSequence {
        eligible_indices: Vec<u64>,
        uuid_equivalence_classes: Vec<Vec<u64>>,
        min_selected: u64,
        max_selected: u64,
        payload_language: CombatSelectionPayloadLanguageV2,
    },
}

#[derive(Serialize)]
struct EnumerationDomainDigestInputV2<'a> {
    contract: &'static str,
    atomic_actions: &'a [CombatActionFingerprintDescriptorV2],
    selection_families: &'a [CombatSelectionActionFamilyV2],
}

fn legal_input_language_digest(
    surface: &crate::sim::combat_action_surface::CombatLegalActionSurfaceV2,
) -> String {
    let mut atomic_inputs = surface.atomic_actions.clone();
    atomic_inputs.sort_by_cached_key(serialized_sort_key);
    let mut selection_languages = surface
        .selection_families
        .iter()
        .filter_map(legal_selection_language_projection)
        .collect::<Vec<_>>();
    selection_languages.sort_by_cached_key(serialized_sort_key);
    hash_serializable(&LegalInputLanguageDigestInputV2 {
        atomic_inputs,
        selection_languages,
    })
}

fn legal_selection_language_projection(
    family: &CombatSelectionActionFamilyV2,
) -> Option<LegalSelectionLanguageProjectionV2> {
    if family.selection_status != CombatSelectionStatusV2::Enabled {
        return None;
    }
    match family.input_encoding {
        CombatSelectionInputEncodingV2::SubmitSelectionHandCardUuids
        | CombatSelectionInputEncodingV2::SubmitSelectionGridCardUuids => {
            let mut eligible_uuids = family
                .raw_domain
                .iter()
                .filter_map(|candidate| match candidate {
                    CombatSelectionDomainCandidateV2::CardUuid {
                        uuid,
                        eligible: true,
                        ..
                    } => Some(*uuid),
                    _ => None,
                })
                .collect::<Vec<_>>();
            eligible_uuids.sort_unstable();
            eligible_uuids.dedup();
            Some(LegalSelectionLanguageProjectionV2::CardUuidSequence {
                input_encoding: family.input_encoding,
                eligible_uuids,
                min_selected: family.declared_min,
                max_selected: family.effective_max,
                payload_language: family.payload_language,
            })
        }
        CombatSelectionInputEncodingV2::SubmitScryDiscardIndices => {
            let mut eligible_indices = Vec::new();
            let mut equivalence_classes = BTreeMap::<u32, Vec<u64>>::new();
            for candidate in &family.raw_domain {
                let CombatSelectionDomainCandidateV2::ScryIndex {
                    index,
                    card_uuid: Some(card_uuid),
                    currently_present: true,
                    ..
                } = candidate
                else {
                    continue;
                };
                eligible_indices.push(*index);
                equivalence_classes
                    .entry(*card_uuid)
                    .or_default()
                    .push(*index);
            }
            eligible_indices.sort_unstable();
            let mut uuid_equivalence_classes =
                equivalence_classes.into_values().collect::<Vec<_>>();
            uuid_equivalence_classes.sort();
            Some(LegalSelectionLanguageProjectionV2::ScryIndexSequence {
                eligible_indices,
                uuid_equivalence_classes,
                min_selected: family.declared_min,
                max_selected: family.effective_max,
                payload_language: family.payload_language,
            })
        }
    }
}

fn enumeration_domain_digest(
    atomic_actions: &[CombatActionFingerprintDescriptorV2],
    selection_families: &[CombatSelectionActionFamilyV2],
) -> String {
    hash_serializable(&EnumerationDomainDigestInputV2 {
        contract: "ordered_semantic_atomic_actions_plus_frozen_selection_domain_v1",
        atomic_actions,
        selection_families,
    })
}

fn serialized_sort_key<T: Serialize>(value: &T) -> Vec<u8> {
    serde_json::to_vec(value).expect("fingerprint sort input should serialize deterministically")
}

pub fn hash_debug<T: std::fmt::Debug>(value: &T) -> String {
    hash_bytes(format!("{value:?}").as_bytes())
}

pub(crate) fn hash_serializable<T: Serialize>(value: &T) -> String {
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
) -> DecisionBoundaryFingerprintV2 {
    DecisionBoundaryFingerprintV2 {
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
) -> CombatPublicObservationFingerprintInputV2 {
    CombatPublicObservationFingerprintInputV2 {
        boundary: boundary_fingerprint(&position.engine, &position.combat),
        public: combat_public_observation_v1(&position.combat),
    }
}

fn combat_action_descriptor_v2(
    combat: &CombatState,
    input: ClientInput,
) -> CombatActionFingerprintDescriptorV2 {
    match &input {
        ClientInput::PlayCard { card_index, target } => {
            let subject =
                combat
                    .zones
                    .hand
                    .get(*card_index)
                    .map(|card| ActionSubjectFingerprintV2 {
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
                .map(|potion| ActionSubjectFingerprintV2 {
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
                .map(|potion| ActionSubjectFingerprintV2 {
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
        ClientInput::SubmitSelection(resolution) => selection_descriptor(
            match resolution.scope {
                crate::state::selection::SelectionScope::Hand => "submit_hand_select",
                crate::state::selection::SelectionScope::Grid => "submit_grid_select",
                crate::state::selection::SelectionScope::Deck => "submit_deck_select",
            },
            input.clone(),
            Vec::new(),
            resolution.selected_card_uuids(),
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
) -> CombatActionFingerprintDescriptorV2 {
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
    subject: Option<ActionSubjectFingerprintV2>,
    target: Option<ActionTargetFingerprintV2>,
    indices: Vec<usize>,
    uuids: Vec<u32>,
) -> CombatActionFingerprintDescriptorV2 {
    CombatActionFingerprintDescriptorV2 {
        kind: kind.to_string(),
        stable_key,
        input,
        subject,
        target,
        indices,
        uuids,
    }
}

fn monster_target(combat: &CombatState, entity_id: usize) -> Option<ActionTargetFingerprintV2> {
    combat
        .entities
        .monsters
        .iter()
        .find(|monster| monster.id == entity_id)
        .map(|monster| ActionTargetFingerprintV2 {
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
    subject: Option<&ActionSubjectFingerprintV2>,
    target: Option<&ActionTargetFingerprintV2>,
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

fn rng_boundary_fingerprint_v2(pool: &RngPool) -> RngBoundaryFingerprintV2 {
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
    RngBoundaryFingerprintV2 {
        status: RngFingerprintStatus::Complete,
        stream_count: streams.len(),
        digest: hash_serializable(&streams),
        streams,
    }
}

fn rng_stream(name: &str, rng: &StsRng) -> RngStreamFingerprintV2 {
    RngStreamFingerprintV2 {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::cards::CardId;
    use crate::content::monsters::EnemyId;
    use crate::content::relics::{RelicId, RelicState};
    use crate::runtime::combat::{CombatCard, Intent};
    use crate::sim::combat_action_surface::{
        CombatSelectionDistinctByV2, CombatSelectionPayloadLanguageV2,
    };
    use crate::state::core::{EngineState, PendingChoice};

    #[test]
    fn large_scry_fingerprint_is_a_single_linear_action_family() {
        let mut combat = combat_with_single_monster();
        let cards = vec![CardId::Strike; 64];
        let card_uuids = (1..=64).collect::<Vec<_>>();
        combat.zones.draw_pile = card_uuids
            .iter()
            .map(|uuid| CombatCard::new(CardId::Strike, *uuid))
            .collect();

        let surface = combat_legal_action_surface_fingerprint_v2(
            &EngineState::PendingChoice(PendingChoice::ScrySelect { cards, card_uuids }),
            &combat,
        );

        assert_eq!(surface.atomic_action_count, 0);
        assert_eq!(surface.action_family_count, 1);
        assert!(surface.atomic_actions.is_empty());
        let family = &surface.selection_families[0];
        assert_eq!(family.raw_domain_count, 64);
        assert_eq!(family.raw_domain.len(), 64);
        assert_eq!(
            family.payload_language,
            CombatSelectionPayloadLanguageV2::OrderedDistinctSequence(
                CombatSelectionDistinctByV2::ScryIndexAndCardUuid
            )
        );
    }

    #[test]
    fn legal_language_and_frozen_enumeration_domain_are_separate() {
        let mut combat = combat_with_single_monster();
        combat.zones.draw_pile = vec![
            CombatCard::new(CardId::Strike, 1),
            CombatCard::new(CardId::Defend, 2),
        ];
        let first = combat_legal_action_surface_fingerprint_v2(
            &EngineState::PendingChoice(PendingChoice::ScrySelect {
                cards: vec![CardId::Strike, CardId::Defend],
                card_uuids: vec![1, 2],
            }),
            &combat,
        );
        let reversed = combat_legal_action_surface_fingerprint_v2(
            &EngineState::PendingChoice(PendingChoice::ScrySelect {
                cards: vec![CardId::Defend, CardId::Strike],
                card_uuids: vec![2, 1],
            }),
            &combat,
        );

        assert_eq!(
            first.legal_input_language_digest,
            reversed.legal_input_language_digest
        );
        assert_ne!(
            first.enumeration_domain_digest,
            reversed.enumeration_domain_digest
        );
    }

    #[test]
    fn atomic_language_is_stable_but_semantic_domain_tracks_hand_identity() {
        let mut combat = combat_with_single_monster();
        combat.turn.energy = 3;
        combat.zones.hand = vec![
            CombatCard::new(CardId::Strike, 1),
            CombatCard::new(CardId::Bash, 2),
        ];
        let first =
            combat_legal_action_surface_fingerprint_v2(&EngineState::CombatPlayerTurn, &combat);
        combat.zones.hand.swap(0, 1);
        let swapped =
            combat_legal_action_surface_fingerprint_v2(&EngineState::CombatPlayerTurn, &combat);

        assert_eq!(
            first.legal_input_language_digest,
            swapped.legal_input_language_digest
        );
        assert_ne!(
            first.enumeration_domain_digest,
            swapped.enumeration_domain_digest
        );
    }

    #[test]
    fn symbolic_action_surface_digest_tracks_hand_candidates_beyond_legacy_caps() {
        let mut combat = combat_with_single_monster();
        let candidate_uuids = (100..124).collect::<Vec<_>>();
        combat.zones.hand = candidate_uuids
            .iter()
            .map(|uuid| CombatCard::new(CardId::Strike, *uuid))
            .collect();
        let first = combat_legal_action_surface_fingerprint_v2(
            &EngineState::PendingChoice(PendingChoice::HandSelect {
                candidate_uuids: candidate_uuids.clone(),
                min_cards: 1,
                max_cards: 1,
                can_cancel: false,
                reason: crate::state::core::HandSelectReason::Discard,
            }),
            &combat,
        );

        let replacement_uuid = 10_000;
        let mut changed_candidates = candidate_uuids;
        changed_candidates[17] = replacement_uuid;
        combat.zones.hand[17] = CombatCard::new(CardId::Defend, replacement_uuid);
        let changed = combat_legal_action_surface_fingerprint_v2(
            &EngineState::PendingChoice(PendingChoice::HandSelect {
                candidate_uuids: changed_candidates,
                min_cards: 1,
                max_cards: 1,
                can_cancel: false,
                reason: crate::state::core::HandSelectReason::Discard,
            }),
            &combat,
        );

        assert_eq!(first.selection_families[0].raw_domain_count, 24);
        assert_ne!(
            first.legal_input_language_digest,
            changed.legal_input_language_digest
        );
        assert_ne!(
            first.enumeration_domain_digest,
            changed.enumeration_domain_digest
        );
    }

    #[test]
    fn public_observation_hash_does_not_change_for_hidden_runic_dome_intent() {
        let mut attack = combat_with_single_monster();
        attack
            .entities
            .player
            .add_relic(RelicState::new(RelicId::RunicDome));
        attack.set_monster_protocol_visible_intent(
            7,
            Intent::Attack {
                damage: 11,
                hits: 1,
            },
        );
        let mut defend = attack.clone();
        defend.set_monster_protocol_visible_intent(7, Intent::Defend);

        assert_eq!(
            public_hash(attack),
            public_hash(defend),
            "Runic Dome hides intent, so changing privileged monster intent must not change public hash"
        );
    }

    #[test]
    fn public_observation_hash_changes_for_visible_intent() {
        let mut attack = combat_with_single_monster();
        attack.set_monster_protocol_visible_intent(
            7,
            Intent::Attack {
                damage: 11,
                hits: 1,
            },
        );
        let mut defend = attack.clone();
        defend.set_monster_protocol_visible_intent(7, Intent::Defend);

        assert_ne!(
            public_hash(attack),
            public_hash(defend),
            "visible monster intent is part of public observation"
        );
    }

    #[test]
    fn public_observation_hash_ignores_draw_order_without_frozen_eye() {
        let mut first = combat_with_single_monster();
        first.zones.draw_pile = vec![
            CombatCard::new(CardId::Bash, 1),
            CombatCard::new(CardId::Strike, 2),
            CombatCard::new(CardId::Defend, 3),
        ];
        let mut reordered = first.clone();
        reordered.zones.draw_pile.swap(0, 2);

        assert_eq!(
            public_hash(first),
            public_hash(reordered),
            "without Frozen Eye, draw pile contents are public but exact order is hidden"
        );
    }

    #[test]
    fn public_observation_hash_tracks_draw_order_with_frozen_eye() {
        let mut first = combat_with_single_monster();
        first
            .entities
            .player
            .add_relic(RelicState::new(RelicId::FrozenEye));
        first.zones.draw_pile = vec![
            CombatCard::new(CardId::Bash, 1),
            CombatCard::new(CardId::Strike, 2),
            CombatCard::new(CardId::Defend, 3),
        ];
        let mut reordered = first.clone();
        reordered.zones.draw_pile.swap(0, 2);

        assert_ne!(
            public_hash(first),
            public_hash(reordered),
            "Frozen Eye makes exact draw pile order public"
        );
    }

    fn public_hash(combat: CombatState) -> String {
        combat_state_fingerprint_v2(&CombatPosition::new(EngineState::CombatPlayerTurn, combat))
            .public_observation_hash
    }

    fn combat_with_single_monster() -> CombatState {
        let mut combat = crate::test_support::blank_test_combat();
        let mut monster = crate::test_support::test_monster(EnemyId::JawWorm);
        monster.id = 7;
        monster.slot = 0;
        combat.entities.monsters.push(monster);
        combat
    }
}
