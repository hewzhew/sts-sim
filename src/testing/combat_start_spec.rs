use std::collections::{HashMap, VecDeque};

use serde::Deserialize;

use crate::content::cards::{get_card_definition, upgraded_base_cost_override, CardId};
use crate::content::monsters::factory::{self, EncounterId};
use crate::content::potions::Potion;
use crate::content::relics::RelicState;
use crate::engine::core::{
    is_smoke_escape_stable_boundary, tick_engine, with_suppressed_engine_warnings,
};
use crate::runtime::action::Action;
use crate::runtime::combat::{CardZones, CombatMeta, TurnRuntime};
use crate::runtime::combat::{CombatCard, CombatRng, CombatState, EngineRuntime, EntityState};
use crate::runtime::rng;
use crate::state::core::EngineState;
use crate::state::map::node::RoomType;
use crate::state::run::RunState;
use crate::state::selection::{EngineDiagnostic, EngineDiagnosticClass, EngineDiagnosticSeverity};

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CombatStartSpec {
    pub name: String,
    pub player_class: String,
    pub ascension_level: i32,
    pub encounter_id: String,
    pub room_type: String,
    pub seed: u64,
    pub player_current_hp: i32,
    pub player_max_hp: i32,
    #[serde(default)]
    pub relics: Vec<StartSpecRelicSpec>,
    #[serde(default)]
    pub potions: Vec<String>,
    pub master_deck: Vec<StartSpecCardSpec>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum StartSpecCardSpec {
    Simple(String),
    Detailed(StartSpecCardEntry),
}

#[derive(Debug, Clone, Deserialize)]
pub struct StartSpecCardEntry {
    pub id: String,
    #[serde(default)]
    pub upgrades: u8,
    #[serde(default)]
    pub cost: Option<i32>,
    #[serde(default)]
    pub misc: Option<i32>,
    #[serde(default = "default_count")]
    pub count: usize,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum StartSpecRelicSpec {
    Simple(String),
    Detailed(StartSpecRelicEntry),
}

#[derive(Debug, Clone, Deserialize)]
pub struct StartSpecRelicEntry {
    pub id: String,
    #[serde(default = "default_relic_counter")]
    pub counter: i32,
}

pub fn compile_combat_start_spec(
    spec: &CombatStartSpec,
) -> Result<(EngineState, CombatState), String> {
    compile_combat_start_spec_with_seed(spec, spec.seed)
}

pub fn compile_combat_start_spec_with_seed(
    spec: &CombatStartSpec,
    seed: u64,
) -> Result<(EngineState, CombatState), String> {
    let player_class = canonical_player_class(&spec.player_class)?;
    let ascension_level = u8::try_from(spec.ascension_level)
        .map_err(|_| format!("ascension_level {} out of u8 range", spec.ascension_level))?;
    let encounter_id = encounter_id_from_spec(&spec.encounter_id)?;
    let room_type = room_type_from_spec(&spec.room_type)?;

    let mut run_state = RunState::new(seed, ascension_level, false, player_class);
    run_state.current_hp = spec.player_current_hp;
    run_state.max_hp = spec.player_max_hp;
    run_state.master_deck = compile_master_deck(&spec.master_deck)?;
    run_state.relics = compile_relics(&spec.relics)?;
    run_state.potions = compile_potions(&spec.potions, ascension_level)?;

    build_natural_start_state(&mut run_state, encounter_id, room_type)
}

pub fn build_natural_start_state(
    run_state: &mut RunState,
    encounter_id: EncounterId,
    room_type: RoomType,
) -> Result<(EngineState, CombatState), String> {
    let player = run_state.build_combat_player(0);
    let monsters = factory::build_encounter(
        encounter_id,
        &mut run_state.rng_pool.misc_rng,
        &mut run_state.rng_pool.monster_hp_rng,
        run_state.ascension_level,
    );

    let mut combat = CombatState {
        meta: CombatMeta {
            ascension_level: run_state.ascension_level,
            player_class: run_state.player_class,
            is_boss_fight: room_type == RoomType::MonsterRoomBoss,
            is_elite_fight: room_type == RoomType::MonsterRoomElite,
            master_deck_snapshot: run_state.master_deck.clone(),
            meta_changes: Vec::new(),
        },
        turn: TurnRuntime::fresh_player_turn(3),
        zones: CardZones {
            draw_pile: run_state.master_deck.clone(),
            hand: Vec::new(),
            discard_pile: Vec::new(),
            exhaust_pile: Vec::new(),
            limbo: Vec::new(),
            queued_cards: VecDeque::new(),
            card_uuid_counter: 9999,
        },
        entities: EntityState {
            player,
            monsters,
            potions: run_state.potions.clone(),
            power_db: HashMap::new(),
        },
        engine: EngineRuntime::new(),
        rng: CombatRng::new(run_state.rng_pool.clone()),
        runtime: Default::default(),
    };

    let monsters_clone = combat.entities.monsters.clone();
    let player_powers = crate::content::powers::store::powers_snapshot_for(&combat, 0);
    let monster_ids = combat
        .entities
        .monsters
        .iter()
        .map(|monster| monster.id)
        .collect::<Vec<_>>();
    for monster_id in monster_ids {
        let entity_snapshot = combat
            .entities
            .monsters
            .iter()
            .find(|monster| monster.id == monster_id)
            .cloned()
            .expect("initial monster should exist while rolling intent");
        let num = combat.rng.ai_rng.random(99);
        let outcome = crate::content::monsters::roll_monster_turn_outcome(
            &mut combat.rng.ai_rng,
            &entity_snapshot,
            combat.meta.ascension_level,
            num,
            &monsters_clone,
            &player_powers,
        );
        for action in outcome.setup_actions {
            crate::engine::action_handlers::execute_action(action, &mut combat);
        }
        let plan = outcome.plan;
        let monster = combat
            .entities
            .monsters
            .iter_mut()
            .find(|monster| monster.id == monster_id)
            .expect("rolled monster should still exist");
        monster.set_planned_move_id(plan.move_id);
        monster.set_planned_steps(plan.steps);
        monster.set_planned_visible_spec(plan.visible_spec);
        monster.move_history_mut().push_back(plan.move_id);
        combat
            .runtime
            .monster_protocol
            .entry(monster_id)
            .or_default()
            .observation = Default::default();
    }

    combat.reset_turn_energy_from_player();
    rng::shuffle_with_random_long(&mut combat.zones.draw_pile, &mut combat.rng.shuffle_rng);
    combat.apply_java_initialize_deck_order_after_shuffle();
    combat.queue_action_back(Action::PreBattleTrigger);

    let mut engine_state = EngineState::CombatProcessing;
    let alive = with_suppressed_engine_warnings(|| drain_to_stable(&mut engine_state, &mut combat));
    if !alive {
        return Err("combat initialization reached terminal state before player input".to_string());
    }
    if !matches!(
        engine_state,
        EngineState::CombatPlayerTurn | EngineState::PendingChoice(_)
    ) {
        return Err(format!(
            "combat initialization did not reach stable player turn, got {engine_state:?}"
        ));
    }

    Ok((engine_state, combat))
}

fn drain_to_stable(es: &mut EngineState, cs: &mut CombatState) -> bool {
    let mut iterations = 0;
    loop {
        match es {
            EngineState::CombatPlayerTurn => break,
            EngineState::CombatProcessing if is_smoke_escape_stable_boundary(es, cs) => break,
            EngineState::CombatProcessing => {}
            EngineState::PendingChoice(_) => break,
            EngineState::GameOver(_) => return false,
            _ => break,
        }
        let alive = tick_engine(es, cs, None);
        if !alive {
            return false;
        }
        iterations += 1;
        if iterations > 1000 {
            cs.emit_diagnostic(EngineDiagnostic {
                severity: EngineDiagnosticSeverity::Warning,
                class: EngineDiagnosticClass::Suspicious,
                message: "tick loop exceeded 1000 iterations".to_string(),
            });
            break;
        }
    }
    true
}

pub fn encounter_id_from_spec(raw: &str) -> Result<EncounterId, String> {
    let normalized = normalize_identifier(raw);
    match normalized.as_str() {
        "blueslaver" => Ok(EncounterId::BlueSlaver),
        "jawworm" => Ok(EncounterId::JawWorm),
        "gremlinnob" | "nob" => Ok(EncounterId::GremlinNob),
        "lagavulin" => Ok(EncounterId::Lagavulin),
        "threesentries" | "sentries" => Ok(EncounterId::ThreeSentries),
        "hexaghost" => Ok(EncounterId::Hexaghost),
        "theguardian" | "guardian" => Ok(EncounterId::TheGuardian),
        "slimeboss" => Ok(EncounterId::SlimeBoss),
        "bookofstabbing" => Ok(EncounterId::BookOfStabbing),
        "collector" => Ok(EncounterId::Collector),
        "thechamp" | "champ" => Ok(EncounterId::TheChamp),
        "automaton" | "bronzeautomaton" => Ok(EncounterId::Automaton),
        "awakenedone" => Ok(EncounterId::AwakenedOne),
        "timeeater" => Ok(EncounterId::TimeEater),
        "donuanddeca" => Ok(EncounterId::DonuAndDeca),
        "shieldandspear" | "spearandshield" => Ok(EncounterId::ShieldAndSpear),
        "theheart" | "corruptheart" | "heart" => Ok(EncounterId::TheHeart),
        _ => Err(format!("unsupported encounter_id '{raw}'")),
    }
}

pub fn room_type_from_spec(raw: &str) -> Result<RoomType, String> {
    let normalized = normalize_identifier(raw);
    match normalized.as_str() {
        "monsterroomboss" | "boss" => Ok(RoomType::MonsterRoomBoss),
        "monsterroomelite" | "elite" => Ok(RoomType::MonsterRoomElite),
        "monsterroom" | "monster" => Ok(RoomType::MonsterRoom),
        _ => Err(format!("unsupported room_type '{raw}'")),
    }
}

fn canonical_player_class(raw: &str) -> Result<&'static str, String> {
    match normalize_identifier(raw).as_str() {
        "ironclad" => Ok("Ironclad"),
        "silent" => Ok("Silent"),
        "defect" => Ok("Defect"),
        "watcher" => Ok("Watcher"),
        _ => Err(format!("unsupported player_class '{raw}'")),
    }
}

#[cfg(test)]
mod tests {
    use super::build_natural_start_state;
    use crate::content::monsters::factory::EncounterId;
    use crate::content::relics::{RelicId, RelicState};
    use crate::runtime::combat::OrbId;
    use crate::state::map::node::RoomType;
    use crate::state::run::RunState;

    #[test]
    fn natural_combat_start_applies_ring_of_the_serpent_opening_hand_size() {
        let mut run = RunState::new(1, 0, false, "Silent");
        run.relics = vec![RelicState::new(RelicId::RingOfTheSerpent)];

        let (_engine_state, combat) =
            build_natural_start_state(&mut run, EncounterId::JawWorm, RoomType::MonsterRoom)
                .expect("combat should initialize");

        assert_eq!(
            combat.zones.hand.len(),
            6,
            "Java Ring of the Serpent increments masterHandSize, so initial draw uses 6"
        );
    }

    #[test]
    fn natural_defect_combat_start_has_java_orb_slots_before_cracked_core() {
        let mut run = RunState::new(1, 0, false, "Defect");

        let (_engine_state, combat) =
            build_natural_start_state(&mut run, EncounterId::JawWorm, RoomType::MonsterRoom)
                .expect("combat should initialize");

        assert_eq!(combat.entities.player.max_orbs, 3);
        assert_eq!(combat.entities.player.orbs.len(), 3);
        assert_eq!(
            combat.entities.player.orbs[0].id,
            OrbId::Lightning,
            "Java Defect starts combat with 3 master orb slots before Cracked Core channels Lightning"
        );
        assert!(combat
            .entities
            .player
            .orbs
            .iter()
            .skip(1)
            .all(|orb| orb.id == OrbId::Empty));
    }

    #[test]
    fn natural_non_defect_prismatic_shard_combat_start_has_one_empty_orb_slot() {
        let mut run = RunState::new(1, 0, false, "Silent");
        run.relics.push(RelicState::new(RelicId::PrismaticShard));

        let (_engine_state, combat) =
            build_natural_start_state(&mut run, EncounterId::JawWorm, RoomType::MonsterRoom)
                .expect("combat should initialize");

        assert_eq!(
            combat.entities.player.max_orbs, 1,
            "Java PrismaticShard.onEquip grants one master orb slot to non-Defect classes"
        );
        assert_eq!(combat.entities.player.orbs.len(), 1);
        assert_eq!(combat.entities.player.orbs[0].id, OrbId::Empty);
    }
}

fn compile_master_deck(specs: &[StartSpecCardSpec]) -> Result<Vec<CombatCard>, String> {
    let mut deck = Vec::new();
    let mut next_uuid = 10_000u32;
    for spec in specs {
        let entry = match spec {
            StartSpecCardSpec::Simple(id) => StartSpecCardEntry {
                id: id.clone(),
                upgrades: 0,
                cost: None,
                misc: None,
                count: 1,
            },
            StartSpecCardSpec::Detailed(entry) => entry.clone(),
        };
        let card_id = input_ids::card_id_from_start_spec(&entry.id)
            .ok_or_else(|| format!("unknown card id '{}'", entry.id))?;
        for _ in 0..entry.count.max(1) {
            let mut card = CombatCard::new(card_id, next_uuid);
            card.upgrades = entry.upgrades;
            card.misc_value = entry.misc.unwrap_or(0);
            if let Some(explicit_cost) = entry.cost {
                let upgraded_base = upgraded_or_base_cost(card.id, card.upgrades);
                if explicit_cost != upgraded_base {
                    return Err(format!(
                        "CombatStartSpec does not support per-card cost overrides yet: {} cost {} != base {}",
                        entry.id, explicit_cost, upgraded_base
                    ));
                }
            }
            deck.push(card);
            next_uuid += 1;
        }
    }
    if deck.is_empty() {
        return Err("CombatStartSpec requires a non-empty master_deck".to_string());
    }
    Ok(deck)
}

fn compile_relics(specs: &[StartSpecRelicSpec]) -> Result<Vec<RelicState>, String> {
    specs
        .iter()
        .map(|spec| match spec {
            StartSpecRelicSpec::Simple(id) => {
                let relic_id = input_ids::relic_id_from_start_spec(id)
                    .ok_or_else(|| format!("unknown relic id '{id}'"))?;
                Ok(RelicState::new(relic_id))
            }
            StartSpecRelicSpec::Detailed(entry) => {
                let relic_id = input_ids::relic_id_from_start_spec(&entry.id)
                    .ok_or_else(|| format!("unknown relic id '{}'", entry.id))?;
                let mut relic = RelicState::new(relic_id);
                relic.counter = entry.counter;
                Ok(relic)
            }
        })
        .collect()
}

fn compile_potions(specs: &[String], ascension_level: u8) -> Result<Vec<Option<Potion>>, String> {
    let slot_count = if ascension_level >= 11 {
        2usize
    } else {
        3usize
    };
    if specs.len() > slot_count {
        return Err(format!(
            "CombatStartSpec requested {} potions but only {slot_count} slots are available",
            specs.len()
        ));
    }
    let mut potions = vec![None; slot_count];
    for (index, id) in specs.iter().enumerate() {
        let potion_id = input_ids::potion_id_from_start_spec(id)
            .ok_or_else(|| format!("unknown potion id '{id}'"))?;
        potions[index] = Some(Potion::new(potion_id, 20_000 + index as u32));
    }
    Ok(potions)
}

fn upgraded_or_base_cost(card_id: CardId, upgrades: u8) -> i32 {
    let mut card = CombatCard::new(card_id, 0);
    card.upgrades = upgrades;
    upgraded_base_cost_override(&card).unwrap_or_else(|| get_card_definition(card.id).cost) as i32
}

fn normalize_identifier(raw: &str) -> String {
    raw.chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .map(|c| c.to_ascii_lowercase())
        .collect()
}

fn default_count() -> usize {
    1
}

fn default_relic_counter() -> i32 {
    -1
}

mod input_ids {
    #![allow(dead_code)]

    use crate::content::cards::{self, CardId};
    use crate::content::monsters::EnemyId;
    use crate::content::potions::PotionId;
    use crate::content::powers::PowerId;
    use crate::content::relics::RelicId;

    include!(concat!(env!("OUT_DIR"), "/generated_schema.rs"));

    pub(super) fn card_id_from_start_spec(raw: &str) -> Option<CardId> {
        let map = cards::build_java_id_map();
        if let Some(card) = map.get(raw).copied() {
            return Some(card);
        }
        let normalized = normalize_input_alias(raw);
        if normalized.is_empty() {
            return None;
        }
        map.into_iter()
            .find_map(|(java, card)| (normalize_input_alias(java) == normalized).then_some(card))
    }

    pub(super) fn relic_id_from_start_spec(raw: &str) -> Option<RelicId> {
        let normalized = normalize_input_alias(raw);
        if normalized.is_empty() {
            return None;
        }
        relic_id_from_java_raw(&normalized)
    }

    pub(super) fn potion_id_from_start_spec(raw: &str) -> Option<PotionId> {
        let normalized = normalize_input_alias(raw);
        if normalized.is_empty() {
            return None;
        }
        java_potion_id_to_rust_raw(&normalized)
    }

    fn normalize_input_alias(raw: &str) -> String {
        raw.chars()
            .filter(|c| c.is_ascii_alphanumeric())
            .map(|c| c.to_ascii_lowercase())
            .collect()
    }
}
