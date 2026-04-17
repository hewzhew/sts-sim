use clap::{Parser, ValueEnum};
use std::collections::{BTreeMap, HashMap};

use std::collections::{HashMap as StdHashMap, VecDeque};
use sts_simulator::content::cards::CardId;
use sts_simulator::content::monsters::EnemyId;
use sts_simulator::content::potions::{
    get_potion_definition, random_potion, PotionClass, PotionId, PotionRarity,
};
use sts_simulator::engine::action_handlers::cards::handle_obtain_potion;
use sts_simulator::runtime::combat::{
    CardZones, CombatMeta, CombatPhase, CombatRng, CombatState, EngineRuntime, EntityState, Intent,
    MonsterEntity, PlayerEntity, RelicBuses, StanceId, TurnRuntime,
};
use sts_simulator::runtime::rng::RngPool;
use sts_simulator::EntityId;

#[derive(Clone, Copy, Debug, ValueEnum)]
enum PotionAuditClassArg {
    Ironclad,
    Silent,
    Defect,
    Watcher,
}

impl PotionAuditClassArg {
    fn as_player_class(self) -> &'static str {
        match self {
            Self::Ironclad => "Ironclad",
            Self::Silent => "Silent",
            Self::Defect => "Defect",
            Self::Watcher => "Watcher",
        }
    }

    fn as_potion_class(self) -> PotionClass {
        match self {
            Self::Ironclad => PotionClass::Ironclad,
            Self::Silent => PotionClass::Silent,
            Self::Defect => PotionClass::Defect,
            Self::Watcher => PotionClass::Watcher,
        }
    }
}

#[derive(Clone, Copy, Debug, ValueEnum)]
enum PotionAuditSourceArg {
    Direct,
    Run,
    Combat,
}

#[derive(Parser, Debug)]
struct Args {
    #[arg(long, value_enum, default_value_t = PotionAuditClassArg::Ironclad)]
    class: PotionAuditClassArg,
    #[arg(long, value_enum, default_value_t = PotionAuditSourceArg::Run)]
    source: PotionAuditSourceArg,
    #[arg(long, default_value_t = 2048)]
    trials: usize,
    #[arg(long, default_value_t = 1)]
    seed: u64,
    #[arg(long, default_value_t = false)]
    limited: bool,
}

fn main() {
    let args = Args::parse();
    let counts = match args.source {
        PotionAuditSourceArg::Direct => {
            sample_direct(args.class, args.trials, args.seed, args.limited)
        }
        PotionAuditSourceArg::Run => sample_run(args.class, args.trials, args.seed),
        PotionAuditSourceArg::Combat => sample_combat(args.class, args.trials, args.seed),
    };

    let mut rarity_counts: BTreeMap<&'static str, usize> = BTreeMap::new();
    let mut class_counts: BTreeMap<&'static str, usize> = BTreeMap::new();
    for (&potion_id, &count) in &counts {
        let def = get_potion_definition(potion_id);
        *rarity_counts.entry(rarity_label(def.rarity)).or_default() += count;
        *class_counts.entry(class_label(def.class)).or_default() += count;
    }

    println!(
        "potion audit | class={} source={:?} trials={} seed={} limited={}",
        args.class.as_player_class(),
        args.source,
        args.trials,
        args.seed,
        args.limited
    );
    println!("rarity:");
    for (label, count) in rarity_counts {
        println!(
            "  {}: {} ({:.2}%)",
            label,
            count,
            count as f64 * 100.0 / args.trials.max(1) as f64
        );
    }
    println!("class:");
    for (label, count) in class_counts {
        println!(
            "  {}: {} ({:.2}%)",
            label,
            count,
            count as f64 * 100.0 / args.trials.max(1) as f64
        );
    }
    println!("potions:");
    let mut potion_rows = counts.into_iter().collect::<Vec<_>>();
    potion_rows.sort_by_key(|(potion_id, _)| get_potion_definition(*potion_id).name);
    for (potion_id, count) in potion_rows {
        let def = get_potion_definition(potion_id);
        println!(
            "  {:<24} {:>4} ({:.2}%) [{}/{}]",
            def.name,
            count,
            count as f64 * 100.0 / args.trials.max(1) as f64,
            rarity_label(def.rarity),
            class_label(def.class)
        );
    }
}

fn sample_direct(
    class: PotionAuditClassArg,
    trials: usize,
    seed: u64,
    limited: bool,
) -> HashMap<PotionId, usize> {
    let mut rng = sts_simulator::runtime::rng::StsRng::new(seed);
    let mut counts = HashMap::new();
    for _ in 0..trials {
        let potion_id = random_potion(&mut rng, class.as_potion_class(), limited);
        *counts.entry(potion_id).or_default() += 1;
    }
    counts
}

fn sample_run(class: PotionAuditClassArg, trials: usize, seed: u64) -> HashMap<PotionId, usize> {
    let mut run = sts_simulator::state::run::RunState::new(seed, 0, false, class.as_player_class());
    let mut counts = HashMap::new();
    for _ in 0..trials {
        let potion_id = run.random_potion();
        *counts.entry(potion_id).or_default() += 1;
    }
    counts
}

fn sample_combat(class: PotionAuditClassArg, trials: usize, seed: u64) -> HashMap<PotionId, usize> {
    let mut combat = audit_combat(seed, class.as_player_class());
    let mut counts = HashMap::new();
    for _ in 0..trials {
        combat.entities.potions[0] = None;
        handle_obtain_potion(&mut combat);
        let potion_id = combat.entities.potions[0]
            .take()
            .expect("combat audit expected potion slot to be filled")
            .id;
        *counts.entry(potion_id).or_default() += 1;
    }
    counts
}

fn rarity_label(rarity: PotionRarity) -> &'static str {
    match rarity {
        PotionRarity::Common => "common",
        PotionRarity::Uncommon => "uncommon",
        PotionRarity::Rare => "rare",
    }
}

fn class_label(class: PotionClass) -> &'static str {
    match class {
        PotionClass::Any => "any",
        PotionClass::Ironclad => "ironclad",
        PotionClass::Silent => "silent",
        PotionClass::Defect => "defect",
        PotionClass::Watcher => "watcher",
    }
}

fn audit_combat(seed: u64, player_class: &'static str) -> CombatState {
    CombatState {
        meta: CombatMeta {
            ascension_level: 0,
            player_class,
            is_boss_fight: false,
            is_elite_fight: false,
            meta_changes: Vec::new(),
        },
        turn: TurnRuntime {
            turn_count: 1,
            current_phase: CombatPhase::PlayerTurn,
            energy: 3,
            turn_start_draw_modifier: 0,
            counters: Default::default(),
        },
        zones: CardZones {
            draw_pile: Vec::new(),
            hand: vec![sts_simulator::runtime::combat::CombatCard::new(
                CardId::Strike,
                1,
            )],
            discard_pile: Vec::new(),
            exhaust_pile: Vec::new(),
            limbo: Vec::new(),
            queued_cards: VecDeque::new(),
            card_uuid_counter: 10,
        },
        entities: EntityState {
            player: PlayerEntity {
                id: 0 as EntityId,
                current_hp: 80,
                max_hp: 80,
                block: 0,
                gold_delta_this_combat: 0,
                gold: 99,
                max_orbs: 0,
                orbs: Vec::new(),
                stance: StanceId::Neutral,
                relics: Vec::new(),
                relic_buses: RelicBuses::default(),
                energy_master: 3,
            },
            monsters: vec![MonsterEntity {
                id: 1,
                monster_type: EnemyId::JawWorm as usize,
                current_hp: 40,
                max_hp: 40,
                block: 0,
                slot: 0,
                is_dying: false,
                is_escaped: false,
                half_dead: false,
                next_move_byte: 0,
                current_intent: Intent::Buff,
                move_history: VecDeque::new(),
                intent_preview_damage: 0,
                logical_position: 0,
                protocol_identity: Default::default(),
                hexaghost: Default::default(),
                chosen: Default::default(),
                darkling: Default::default(),
                lagavulin: Default::default(),
            }],
            potions: vec![None, None, None],
            power_db: StdHashMap::new(),
        },
        engine: EngineRuntime {
            action_queue: VecDeque::new(),
        },
        rng: CombatRng::new(RngPool::new(seed)),
        runtime: Default::default(),
    }
}
