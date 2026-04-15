use std::collections::{HashMap, VecDeque};

use serde::Deserialize;

use crate::action::Action;
use crate::combat::{CardZones, CombatMeta, TurnRuntime};
use crate::combat::{CombatCard, CombatPhase, CombatRng, CombatState, EngineRuntime, EntityState};
use crate::content::cards::{get_card_definition, upgraded_base_cost_override, CardId};
use crate::content::monsters::factory::{self, EncounterId};
use crate::content::potions::Potion;
use crate::content::relics::RelicState;
use crate::diff::protocol::mapper::{
    card_id_from_java, java_potion_id_to_rust, relic_id_from_java,
};
use crate::diff::replay::replay_support::drain_to_stable;
use crate::engine::core::with_suppressed_engine_warnings;
use crate::map::node::RoomType;
use crate::rng;
use crate::state::core::EngineState;
use crate::state::run::RunState;

use crate::testing::fixtures::author_spec::{AuthorCardEntry, AuthorCardSpec, AuthorRelicSpec};

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
    pub relics: Vec<AuthorRelicSpec>,
    #[serde(default)]
    pub potions: Vec<String>,
    pub master_deck: Vec<AuthorCardSpec>,
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
            meta_changes: Vec::new(),
        },
        turn: TurnRuntime {
            turn_count: 0,
            current_phase: CombatPhase::PlayerTurn,
            energy: 3,
            turn_start_draw_modifier: 0,
            counters: Default::default(),
        },
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
        engine: EngineRuntime {
            action_queue: VecDeque::new(),
        },
        rng: CombatRng::new(run_state.rng_pool.clone()),
        runtime: Default::default(),
    };

    let monsters_clone = combat.entities.monsters.clone();
    for monster in &mut combat.entities.monsters {
        let num = combat.rng.ai_rng.random(99);
        let (move_byte, intent) = crate::content::monsters::roll_monster_move(
            &mut combat.rng.ai_rng,
            monster,
            combat.meta.ascension_level,
            num,
            &monsters_clone,
        );
        monster.next_move_byte = move_byte;
        monster.current_intent = intent;
        monster.move_history.push_back(move_byte);
    }

    combat.turn.energy = combat.entities.player.energy_master;
    rng::shuffle_with_random_long(&mut combat.zones.draw_pile, &mut combat.rng.shuffle_rng);
    let mut innate_cards = Vec::new();
    let mut normal_cards = Vec::new();
    for card in std::mem::take(&mut combat.zones.draw_pile) {
        if crate::content::cards::is_innate_card(&card) {
            innate_cards.push(card);
        } else {
            normal_cards.push(card);
        }
    }
    innate_cards.extend(normal_cards);
    combat.zones.draw_pile = innate_cards;
    combat
        .engine
        .action_queue
        .push_back(Action::PreBattleTrigger);

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

pub fn encounter_id_from_spec(raw: &str) -> Result<EncounterId, String> {
    let normalized = normalize_identifier(raw);
    match normalized.as_str() {
        "jawworm" => Ok(EncounterId::JawWorm),
        "hexaghost" => Ok(EncounterId::Hexaghost),
        "theguardian" | "guardian" => Ok(EncounterId::TheGuardian),
        "slimeboss" => Ok(EncounterId::SlimeBoss),
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

fn compile_master_deck(specs: &[AuthorCardSpec]) -> Result<Vec<CombatCard>, String> {
    let mut deck = Vec::new();
    let mut next_uuid = 10_000u32;
    for spec in specs {
        let entry = match spec {
            AuthorCardSpec::Simple(id) => AuthorCardEntry {
                id: id.clone(),
                upgrades: 0,
                cost: None,
                misc: None,
                count: 1,
            },
            AuthorCardSpec::Detailed(entry) => entry.clone(),
        };
        let card_id = card_id_from_java(&entry.id)
            .ok_or_else(|| format!("unknown Java card id '{}'", entry.id))?;
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

fn compile_relics(specs: &[AuthorRelicSpec]) -> Result<Vec<RelicState>, String> {
    specs
        .iter()
        .map(|spec| match spec {
            AuthorRelicSpec::Simple(id) => {
                let relic_id = relic_id_from_java(id)
                    .ok_or_else(|| format!("unknown Java relic id '{id}'"))?;
                Ok(RelicState::new(relic_id))
            }
            AuthorRelicSpec::Detailed(entry) => {
                let relic_id = relic_id_from_java(&entry.id)
                    .ok_or_else(|| format!("unknown Java relic id '{}'", entry.id))?;
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
        let potion_id =
            java_potion_id_to_rust(id).ok_or_else(|| format!("unknown Java potion id '{id}'"))?;
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

