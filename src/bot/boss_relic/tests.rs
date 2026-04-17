use super::rank::sort_candidates;
use super::{decide, BossRelicCandidate, RelicCompatibility};
use crate::bot::Agent;
use crate::content::cards::CardId;
use crate::content::cards::CardId::{
    Anger, Barricade, Bash, BattleTrance, Bludgeon, BodySlam, BurningPact, DarkEmbrace, Defend,
    DemonForm, Entrench, FiendFire, Finesse, FlashOfSteel, Headbutt, Impervious, Inflame,
    LimitBreak, PommelStrike, PowerThrough, SecondWind, ShrugItOff, Strike, TrueGrit, TwinStrike,
    Uppercut, Whirlwind, WildStrike,
};
use crate::content::potions::{Potion, PotionId};
use crate::content::relics::{RelicId, RelicState};
use crate::map::node::{MapEdge, MapRoomNode, RoomType};
use crate::map::state::MapState;
use crate::rewards::state::BossRelicChoiceState;
use crate::runtime::combat::CombatCard;
use crate::state::run::RunState;

struct Fixture {
    run_state: RunState,
    next_uuid: u32,
}

impl Fixture {
    fn new() -> Self {
        let mut run_state = RunState::new(1, 0, false, "Ironclad");
        run_state.floor_num = 17;
        run_state.act_num = 1;
        run_state.map = linear_map(&[
            RoomType::MonsterRoom,
            RoomType::RestRoom,
            RoomType::ShopRoom,
            RoomType::MonsterRoomElite,
        ]);
        Self {
            run_state,
            next_uuid: 1000,
        }
    }

    fn act(mut self, act_num: u8) -> Self {
        self.run_state.act_num = act_num;
        self
    }

    fn hp(mut self, current_hp: i32, max_hp: i32) -> Self {
        self.run_state.current_hp = current_hp;
        self.run_state.max_hp = max_hp;
        self
    }

    fn deck(mut self, cards: &[(CardId, u8)]) -> Self {
        let mut next_uuid = self.next_uuid;
        self.run_state.master_deck = cards
            .iter()
            .map(|(id, upgrades)| {
                let mut card = CombatCard::new(*id, next_uuid);
                card.upgrades = *upgrades;
                next_uuid += 1;
                card
            })
            .collect();
        self.next_uuid = next_uuid;
        self
    }

    fn add_relics(mut self, relics: &[RelicId]) -> Self {
        for relic_id in relics {
            self.run_state.relics.push(RelicState::new(*relic_id));
        }
        self
    }

    fn potions(mut self, potions: &[Option<PotionId>]) -> Self {
        self.run_state.potions = potions
            .iter()
            .enumerate()
            .map(|(idx, potion_id)| potion_id.map(|id| Potion::new(id, idx as u32 + 1)))
            .collect();
        self
    }

    fn route(mut self, route: &[RoomType]) -> Self {
        self.run_state.map = linear_map(route);
        self
    }

    fn final_act(mut self, enabled: bool, keys: [bool; 3]) -> Self {
        self.run_state.is_final_act_available = enabled;
        self.run_state.keys = keys;
        self
    }

    fn build(self) -> RunState {
        self.run_state
    }
}

fn linear_map(route: &[RoomType]) -> MapState {
    let mut graph = route
        .iter()
        .enumerate()
        .map(|(y, room_type)| {
            let mut node = MapRoomNode::new(0, y as i32);
            node.class = Some(*room_type);
            vec![node]
        })
        .collect::<Vec<_>>();

    for y in 0..graph.len().saturating_sub(1) {
        graph[y][0]
            .edges
            .insert(MapEdge::new(0, y as i32, 0, y as i32 + 1));
    }

    MapState::new(graph)
}

fn candidate_for<'a>(
    diagnostics: &'a super::BossRelicDecisionDiagnostics,
    relic_id: RelicId,
) -> &'a BossRelicCandidate {
    diagnostics
        .top_candidates
        .iter()
        .find(|candidate| candidate.relic_id == format!("{relic_id:?}"))
        .unwrap()
}

fn manual_candidate(
    index: usize,
    compatibility: RelicCompatibility,
    rank_score: i32,
    confidence: i32,
    volatility: i32,
    primary_reason: &'static str,
) -> BossRelicCandidate {
    BossRelicCandidate {
        index,
        relic_id: format!("Test{index}"),
        compatibility,
        rank_score,
        upside: rank_score.max(0),
        downside: (-rank_score).max(0),
        volatility,
        confidence,
        primary_reason,
        positive_tags: Vec::new(),
        negative_tags: Vec::new(),
    }
}

fn choose(run_state: RunState, relics: &[RelicId]) -> (usize, super::BossRelicDecisionDiagnostics) {
    decide(&run_state, relics)
}

#[test]
fn strong_fit_sorts_above_higher_scoring_neutral() {
    let mut candidates = vec![
        manual_candidate(0, RelicCompatibility::Neutral, 90, 70, 10, "neutral"),
        manual_candidate(1, RelicCompatibility::StrongFit, 10, 20, 30, "fit"),
    ];
    sort_candidates(&mut candidates);
    assert_eq!(candidates[0].compatibility, RelicCompatibility::StrongFit);
}

#[test]
fn neutral_sorts_above_high_risk_even_if_score_is_slightly_lower() {
    let mut candidates = vec![
        manual_candidate(0, RelicCompatibility::HighRisk, 40, 70, 10, "risk"),
        manual_candidate(1, RelicCompatibility::Neutral, 35, 60, 15, "neutral"),
    ];
    sort_candidates(&mut candidates);
    assert_eq!(candidates[0].compatibility, RelicCompatibility::Neutral);
}

#[test]
fn hard_reject_never_wins_if_non_reject_exists() {
    let mut candidates = vec![
        manual_candidate(0, RelicCompatibility::HardReject, 500, 99, 0, "reject"),
        manual_candidate(1, RelicCompatibility::HighRisk, -10, 1, 99, "risk"),
    ];
    sort_candidates(&mut candidates);
    assert_ne!(candidates[0].compatibility, RelicCompatibility::HardReject);
}

#[test]
fn unmodeled_relic_stays_below_explicit_neutral_option() {
    let run_state = Fixture::new()
        .deck(&[
            (Strike, 0),
            (Strike, 0),
            (Strike, 0),
            (Strike, 0),
            (Defend, 0),
            (Defend, 0),
            (Defend, 0),
            (Defend, 0),
            (Bash, 0),
            (Anger, 0),
        ])
        .build();
    let (chosen_index, diagnostics) = choose(run_state, &[RelicId::SacredBark, RelicId::TinyHouse]);
    assert_eq!(chosen_index, 1);
    assert_eq!(
        candidate_for(&diagnostics, RelicId::SacredBark).compatibility,
        RelicCompatibility::HighRisk
    );
}

#[test]
fn runic_pyramid_flips_between_fit_and_hand_clog_shells() {
    let fit_run = Fixture::new()
        .act(2)
        .deck(&[
            (Impervious, 1),
            (Entrench, 1),
            (BodySlam, 1),
            (ShrugItOff, 1),
            (BattleTrance, 1),
            (SecondWind, 1),
            (BurningPact, 1),
            (DarkEmbrace, 1),
            (FiendFire, 1),
            (Uppercut, 1),
            (DemonForm, 1),
        ])
        .build();
    let (fit_choice, fit_diag) = choose(fit_run, &[RelicId::TinyHouse, RelicId::RunicPyramid]);
    assert_eq!(fit_choice, 1);
    assert_eq!(
        candidate_for(&fit_diag, RelicId::RunicPyramid).compatibility,
        RelicCompatibility::StrongFit
    );

    let clog_run = Fixture::new()
        .deck(&[
            (WildStrike, 0),
            (WildStrike, 0),
            (PowerThrough, 0),
            (PowerThrough, 0),
            (Bludgeon, 0),
            (DemonForm, 0),
            (Impervious, 0),
            (Bash, 0),
            (Strike, 0),
            (Defend, 0),
            (Defend, 0),
        ])
        .build();
    let (clog_choice, clog_diag) = choose(clog_run, &[RelicId::RunicPyramid, RelicId::TinyHouse]);
    assert_eq!(clog_choice, 1);
    assert!(matches!(
        candidate_for(&clog_diag, RelicId::RunicPyramid).compatibility,
        RelicCompatibility::HighRisk | RelicCompatibility::HardReject
    ));
}

#[test]
fn snecko_eye_flips_between_high_cost_shell_and_low_cost_precision_shell() {
    let fit_run = Fixture::new()
        .act(2)
        .deck(&[
            (Bludgeon, 1),
            (DemonForm, 1),
            (Impervious, 1),
            (FiendFire, 1),
            (Uppercut, 1),
            (Barricade, 1),
            (Whirlwind, 1),
            (BodySlam, 1),
            (Headbutt, 1),
        ])
        .build();
    let (fit_choice, fit_diag) = choose(fit_run, &[RelicId::TinyHouse, RelicId::SneckoEye]);
    assert_eq!(fit_choice, 1);
    assert_eq!(
        candidate_for(&fit_diag, RelicId::SneckoEye).compatibility,
        RelicCompatibility::StrongFit
    );

    let low_cost_run = Fixture::new()
        .deck(&[
            (Anger, 0),
            (Anger, 0),
            (Finesse, 0),
            (FlashOfSteel, 0),
            (BattleTrance, 0),
            (ShrugItOff, 0),
            (PommelStrike, 0),
            (TwinStrike, 0),
            (Headbutt, 0),
            (TrueGrit, 0),
        ])
        .build();
    let (low_cost_choice, low_cost_diag) =
        choose(low_cost_run, &[RelicId::SneckoEye, RelicId::TinyHouse]);
    assert_eq!(low_cost_choice, 1);
    assert!(matches!(
        candidate_for(&low_cost_diag, RelicId::SneckoEye).compatibility,
        RelicCompatibility::HighRisk | RelicCompatibility::HardReject
    ));
}

#[test]
fn busted_crown_flips_between_immature_and_mature_decks() {
    let immature_run = Fixture::new()
        .deck(&[
            (Strike, 0),
            (Strike, 0),
            (Strike, 0),
            (Strike, 0),
            (Strike, 0),
            (Defend, 0),
            (Defend, 0),
            (Defend, 0),
            (Defend, 0),
            (Bash, 0),
        ])
        .build();
    let (immature_choice, immature_diag) =
        choose(immature_run, &[RelicId::BustedCrown, RelicId::TinyHouse]);
    assert_eq!(immature_choice, 1);
    assert_eq!(
        candidate_for(&immature_diag, RelicId::BustedCrown).compatibility,
        RelicCompatibility::HardReject
    );

    let mature_run = Fixture::new()
        .act(2)
        .deck(&[
            (ShrugItOff, 1),
            (ShrugItOff, 1),
            (BattleTrance, 1),
            (Impervious, 1),
            (Entrench, 1),
            (BodySlam, 1),
            (SecondWind, 1),
            (BurningPact, 1),
            (DarkEmbrace, 1),
            (Barricade, 1),
            (Whirlwind, 1),
            (Inflame, 1),
            (Headbutt, 1),
            (Uppercut, 1),
            (PommelStrike, 1),
            (LimitBreak, 1),
            (TrueGrit, 1),
        ])
        .build();
    let (mature_choice, mature_diag) =
        choose(mature_run, &[RelicId::TinyHouse, RelicId::BustedCrown]);
    assert_eq!(mature_choice, 1);
    assert_eq!(
        candidate_for(&mature_diag, RelicId::BustedCrown).compatibility,
        RelicCompatibility::StrongFit
    );
}

#[test]
fn sozu_flips_with_potion_dependence() {
    let potion_dependent_run = Fixture::new()
        .deck(&[
            (Strike, 0),
            (Strike, 0),
            (Strike, 0),
            (Strike, 0),
            (Defend, 0),
            (Defend, 0),
            (Defend, 0),
            (Defend, 0),
            (Bash, 0),
            (Anger, 0),
        ])
        .potions(&[
            Some(PotionId::AncientPotion),
            Some(PotionId::PowerPotion),
            Some(PotionId::FairyPotion),
        ])
        .build();
    let (dependent_choice, dependent_diag) =
        choose(potion_dependent_run, &[RelicId::Sozu, RelicId::TinyHouse]);
    assert_eq!(dependent_choice, 1);
    assert!(matches!(
        candidate_for(&dependent_diag, RelicId::Sozu).compatibility,
        RelicCompatibility::HighRisk | RelicCompatibility::HardReject
    ));

    let low_dependence_run = Fixture::new()
        .act(2)
        .deck(&[
            (ShrugItOff, 1),
            (BattleTrance, 1),
            (Impervious, 1),
            (Entrench, 1),
            (BodySlam, 1),
            (SecondWind, 1),
            (BurningPact, 1),
            (Whirlwind, 1),
            (Uppercut, 1),
            (DemonForm, 1),
        ])
        .potions(&[
            Some(PotionId::SmokeBomb),
            Some(PotionId::CunningPotion),
            Some(PotionId::PotionOfCapacity),
        ])
        .build();
    let (low_dep_choice, low_dep_diag) =
        choose(low_dependence_run, &[RelicId::CallingBell, RelicId::Sozu]);
    assert!(low_dep_choice == 0 || low_dep_choice == 1);
    assert!(matches!(
        candidate_for(&low_dep_diag, RelicId::Sozu).compatibility,
        RelicCompatibility::Neutral | RelicCompatibility::StrongFit
    ));
}

#[test]
fn coffee_dripper_flips_with_hp_and_sustain() {
    let low_hp_run = Fixture::new()
        .hp(18, 80)
        .route(&[
            RoomType::RestRoom,
            RoomType::MonsterRoom,
            RoomType::ShopRoom,
        ])
        .deck(&[
            (Strike, 0),
            (Strike, 0),
            (Strike, 0),
            (Strike, 0),
            (Defend, 0),
            (Defend, 0),
            (Defend, 0),
            (Defend, 0),
            (Bash, 0),
            (Anger, 0),
        ])
        .build();
    let (low_hp_choice, low_hp_diag) =
        choose(low_hp_run, &[RelicId::CoffeeDripper, RelicId::TinyHouse]);
    assert_eq!(low_hp_choice, 1);
    assert_eq!(
        candidate_for(&low_hp_diag, RelicId::CoffeeDripper).compatibility,
        RelicCompatibility::HardReject
    );

    let high_hp_run = Fixture::new()
        .act(2)
        .hp(70, 80)
        .route(&[
            RoomType::MonsterRoom,
            RoomType::MonsterRoomElite,
            RoomType::ShopRoom,
        ])
        .add_relics(&[RelicId::BloodVial, RelicId::MeatOnTheBone])
        .deck(&[
            (ShrugItOff, 1),
            (BattleTrance, 1),
            (Impervious, 1),
            (SecondWind, 1),
            (BurningPact, 1),
            (Whirlwind, 1),
            (Uppercut, 1),
            (DemonForm, 1),
            (Barricade, 1),
        ])
        .build();
    let (high_hp_choice, high_hp_diag) =
        choose(high_hp_run, &[RelicId::TinyHouse, RelicId::CoffeeDripper]);
    assert_eq!(high_hp_choice, 1);
    assert_eq!(
        candidate_for(&high_hp_diag, RelicId::CoffeeDripper).compatibility,
        RelicCompatibility::StrongFit
    );
}

#[test]
fn fusion_hammer_flips_with_upgrade_backlog() {
    let backlog_run = Fixture::new()
        .deck(&[
            (ShrugItOff, 0),
            (ShrugItOff, 0),
            (BattleTrance, 0),
            (Impervious, 0),
            (Entrench, 0),
            (BodySlam, 0),
            (SecondWind, 0),
            (BurningPact, 0),
            (DarkEmbrace, 0),
            (Barricade, 0),
            (Whirlwind, 0),
        ])
        .build();
    let (backlog_choice, backlog_diag) =
        choose(backlog_run, &[RelicId::FusionHammer, RelicId::TinyHouse]);
    assert_eq!(backlog_choice, 1);
    assert!(matches!(
        candidate_for(&backlog_diag, RelicId::FusionHammer).compatibility,
        RelicCompatibility::HighRisk | RelicCompatibility::HardReject
    ));

    let finished_run = Fixture::new()
        .act(2)
        .deck(&[
            (ShrugItOff, 1),
            (ShrugItOff, 1),
            (BattleTrance, 1),
            (Impervious, 1),
            (Entrench, 1),
            (BodySlam, 1),
            (SecondWind, 1),
            (BurningPact, 1),
            (DarkEmbrace, 1),
            (Barricade, 1),
            (Whirlwind, 1),
        ])
        .build();
    let (finished_choice, finished_diag) =
        choose(finished_run, &[RelicId::TinyHouse, RelicId::FusionHammer]);
    assert_eq!(finished_choice, 1);
    assert_eq!(
        candidate_for(&finished_diag, RelicId::FusionHammer).compatibility,
        RelicCompatibility::StrongFit
    );
}

#[test]
fn velvet_choker_hard_rejects_high_action_shells() {
    let run_state = Fixture::new()
        .deck(&[
            (Anger, 0),
            (Anger, 0),
            (Finesse, 0),
            (FlashOfSteel, 0),
            (BattleTrance, 0),
            (ShrugItOff, 0),
            (PommelStrike, 0),
            (PommelStrike, 0),
            (TwinStrike, 0),
            (Headbutt, 0),
            (TrueGrit, 0),
        ])
        .build();
    let (chosen_index, diagnostics) =
        choose(run_state, &[RelicId::VelvetChoker, RelicId::TinyHouse]);
    assert_eq!(chosen_index, 1);
    assert_eq!(
        candidate_for(&diagnostics, RelicId::VelvetChoker).compatibility,
        RelicCompatibility::HardReject
    );
}

#[test]
fn mark_of_pain_gets_better_with_exhaust_outlets() {
    let outlet_run = Fixture::new()
        .act(2)
        .deck(&[
            (SecondWind, 1),
            (BurningPact, 1),
            (FiendFire, 1),
            (TrueGrit, 1),
            (DarkEmbrace, 1),
            (Whirlwind, 1),
            (Uppercut, 1),
            (ShrugItOff, 1),
            (Impervious, 1),
        ])
        .build();
    let (outlet_choice, outlet_diag) =
        choose(outlet_run, &[RelicId::TinyHouse, RelicId::MarkOfPain]);
    assert_eq!(outlet_choice, 1);
    assert_eq!(
        candidate_for(&outlet_diag, RelicId::MarkOfPain).compatibility,
        RelicCompatibility::StrongFit
    );

    let no_outlet_run = Fixture::new()
        .deck(&[
            (Strike, 0),
            (Strike, 0),
            (Strike, 0),
            (Strike, 0),
            (Defend, 0),
            (Defend, 0),
            (Defend, 0),
            (Defend, 0),
            (Bash, 0),
            (Bludgeon, 0),
        ])
        .build();
    let (no_outlet_choice, no_outlet_diag) =
        choose(no_outlet_run, &[RelicId::MarkOfPain, RelicId::TinyHouse]);
    assert_eq!(no_outlet_choice, 1);
    assert_eq!(
        candidate_for(&no_outlet_diag, RelicId::MarkOfPain).compatibility,
        RelicCompatibility::HighRisk
    );
}

#[test]
fn ectoplasm_flips_with_shop_dependence() {
    let early_shop_run = Fixture::new()
        .route(&[
            RoomType::ShopRoom,
            RoomType::RestRoom,
            RoomType::MonsterRoomElite,
        ])
        .deck(&[
            (Strike, 0),
            (Strike, 0),
            (Strike, 0),
            (Strike, 0),
            (Defend, 0),
            (Defend, 0),
            (Defend, 0),
            (Defend, 0),
            (Bash, 0),
            (Anger, 0),
        ])
        .build();
    let (early_choice, early_diag) =
        choose(early_shop_run, &[RelicId::Ectoplasm, RelicId::TinyHouse]);
    assert_eq!(early_choice, 1);
    assert_eq!(
        candidate_for(&early_diag, RelicId::Ectoplasm).compatibility,
        RelicCompatibility::HardReject
    );

    let late_run = Fixture::new()
        .act(2)
        .route(&[
            RoomType::MonsterRoom,
            RoomType::RestRoom,
            RoomType::MonsterRoomElite,
        ])
        .deck(&[
            (ShrugItOff, 1),
            (BattleTrance, 1),
            (Impervious, 1),
            (Entrench, 1),
            (BodySlam, 1),
            (SecondWind, 1),
            (BurningPact, 1),
            (DarkEmbrace, 1),
            (Barricade, 1),
            (Whirlwind, 1),
            (Uppercut, 1),
        ])
        .build();
    let (late_choice, late_diag) = choose(late_run, &[RelicId::TinyHouse, RelicId::Ectoplasm]);
    assert_eq!(late_choice, 1);
    assert_eq!(
        candidate_for(&late_diag, RelicId::Ectoplasm).compatibility,
        RelicCompatibility::StrongFit
    );
}

#[test]
fn cursed_key_flips_with_curse_tolerance() {
    let tolerant_run = Fixture::new()
        .act(2)
        .add_relics(&[RelicId::Omamori])
        .deck(&[
            (SecondWind, 1),
            (BurningPact, 1),
            (DarkEmbrace, 1),
            (Whirlwind, 1),
            (Uppercut, 1),
            (ShrugItOff, 1),
            (Impervious, 1),
            (Barricade, 1),
            (TrueGrit, 1),
        ])
        .build();
    let (tolerant_choice, tolerant_diag) =
        choose(tolerant_run, &[RelicId::TinyHouse, RelicId::CursedKey]);
    assert_eq!(tolerant_choice, 1);
    assert_eq!(
        candidate_for(&tolerant_diag, RelicId::CursedKey).compatibility,
        RelicCompatibility::StrongFit
    );

    let fragile_run = Fixture::new()
        .deck(&[
            (Strike, 0),
            (Strike, 0),
            (Strike, 0),
            (Strike, 0),
            (Defend, 0),
            (Defend, 0),
            (Defend, 0),
            (Defend, 0),
            (Bash, 0),
            (WildStrike, 0),
        ])
        .build();
    let (fragile_choice, fragile_diag) =
        choose(fragile_run, &[RelicId::CursedKey, RelicId::TinyHouse]);
    assert_eq!(fragile_choice, 1);
    assert!(matches!(
        candidate_for(&fragile_diag, RelicId::CursedKey).compatibility,
        RelicCompatibility::HighRisk | RelicCompatibility::HardReject
    ));
}

#[test]
fn philosophers_stone_flips_with_kill_speed_and_fragility() {
    let aggressive_run = Fixture::new()
        .act(2)
        .hp(70, 80)
        .deck(&[
            (Whirlwind, 1),
            (Whirlwind, 1),
            (Inflame, 1),
            (Uppercut, 1),
            (Headbutt, 1),
            (Bludgeon, 1),
            (TwinStrike, 1),
            (PommelStrike, 1),
            (ShrugItOff, 1),
        ])
        .build();
    let (aggressive_choice, aggressive_diag) = choose(
        aggressive_run,
        &[RelicId::TinyHouse, RelicId::PhilosopherStone],
    );
    assert_eq!(aggressive_choice, 1);
    assert_eq!(
        candidate_for(&aggressive_diag, RelicId::PhilosopherStone).compatibility,
        RelicCompatibility::StrongFit
    );

    let fragile_run = Fixture::new()
        .hp(22, 80)
        .deck(&[
            (Strike, 0),
            (Strike, 0),
            (Strike, 0),
            (Strike, 0),
            (Defend, 0),
            (Defend, 0),
            (Defend, 0),
            (BattleTrance, 0),
            (DemonForm, 0),
            (Bludgeon, 0),
        ])
        .build();
    let (fragile_choice, fragile_diag) = choose(
        fragile_run,
        &[RelicId::PhilosopherStone, RelicId::TinyHouse],
    );
    assert_eq!(fragile_choice, 1);
    assert_eq!(
        candidate_for(&fragile_diag, RelicId::PhilosopherStone).compatibility,
        RelicCompatibility::HighRisk
    );
}

#[test]
fn pandoras_box_flips_with_starter_density_and_volatility_tolerance() {
    let starter_heavy_run = Fixture::new()
        .final_act(true, [true, true, false])
        .deck(&[
            (Strike, 0),
            (Strike, 0),
            (Strike, 0),
            (Strike, 0),
            (Strike, 0),
            (Defend, 0),
            (Defend, 0),
            (Defend, 0),
            (Defend, 0),
            (Bash, 0),
        ])
        .build();
    let (starter_choice, starter_diag) = choose(
        starter_heavy_run,
        &[RelicId::TinyHouse, RelicId::PandorasBox],
    );
    assert_eq!(starter_choice, 1);
    assert_eq!(
        candidate_for(&starter_diag, RelicId::PandorasBox).compatibility,
        RelicCompatibility::StrongFit
    );

    let refined_run = Fixture::new()
        .act(2)
        .deck(&[
            (ShrugItOff, 1),
            (BattleTrance, 1),
            (Impervious, 1),
            (Entrench, 1),
            (BodySlam, 1),
            (SecondWind, 1),
            (BurningPact, 1),
            (DarkEmbrace, 1),
            (Barricade, 1),
            (Whirlwind, 1),
            (Uppercut, 1),
        ])
        .build();
    let (refined_choice, refined_diag) =
        choose(refined_run, &[RelicId::PandorasBox, RelicId::TinyHouse]);
    assert_eq!(refined_choice, 1);
    assert!(matches!(
        candidate_for(&refined_diag, RelicId::PandorasBox).compatibility,
        RelicCompatibility::HighRisk | RelicCompatibility::HardReject
    ));
}

#[test]
fn policy_spine_boss_relic_decision_uses_new_payload() {
    let run_state = Fixture::new()
        .act(2)
        .deck(&[
            (ShrugItOff, 1),
            (BattleTrance, 1),
            (Impervious, 1),
            (Entrench, 1),
            (BodySlam, 1),
            (SecondWind, 1),
            (BurningPact, 1),
            (DarkEmbrace, 1),
            (Barricade, 1),
            (Whirlwind, 1),
            (Uppercut, 1),
        ])
        .build();
    let state = BossRelicChoiceState::new(vec![RelicId::TinyHouse, RelicId::BustedCrown]);
    let agent = Agent::new();
    let decision = agent.decide_boss_relic_policy(&run_state, &state);
    assert_eq!(decision.chosen_index, 1);
    assert_eq!(decision.meta.rationale_key, Some("boss_relic_reward_lock"));
    assert_eq!(
        decision.diagnostics.top_candidates[0].compatibility,
        RelicCompatibility::StrongFit
    );
}
