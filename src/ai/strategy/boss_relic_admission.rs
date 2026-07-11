use crate::ai::action_supply_v1::action_supply_profile_v1;
use crate::ai::card_semantics_v1::card_mechanics_profile_v1;
use crate::ai::deck_startup_profile_v1::{deck_startup_profile_v1, PyramidApparitionCoverageV1};
use crate::ai::pandora_offer_profile_v1::{pandora_offer_profile_v1, PandoraOfferHorizonV1};
use crate::ai::strategic::run_debt_projection_for_relic_v1;
use crate::ai::strategy::deck_plan::DeckPlanSnapshot;
use crate::ai::strategy::run_strategic_facts::RunStrategicFacts;
use crate::content::cards::CardId;
use crate::content::relics::{RelicId, RelicState};
use crate::state::run::RunState;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BossRelicAdmissionClass {
    StarterUpgrade,
    LowDownsideValue,
    DeckCleanup,
    RouteValue,
    EnergyWithConstraint,
    StrategicPower,
    CurseDebt,
    TransformAgency,
    Unknown,
    Skip,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BossRelicAdmissionLane {
    Mainline,
    Probe,
    Skip,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BossRelicAdmissionBurden {
    None,
    AddedRunDebt,
    IntroducedStartupLiability,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BossRelicAdmissionReason {
    StarterUpgrade,
    BroadValue,
    RemovesCards {
        starter_basics: usize,
        curses: usize,
    },
    EnergyGain,
    RouteDependent,
    RestLocked,
    SmithLocked,
    GoldLocked,
    PotionLocked,
    RewardWidthLoss,
    EnemyIntentHidden,
    PlayLimit,
    AddsCurse,
    FutureChestCurses,
    TransformsDeck,
    HandRetention,
    SneckoConfusion,
    Act2EnergyGap,
    DoesNotSolveAct2EnergyGap,
    NoRestDebt,
    AddsRunDebt {
        contracts: usize,
    },
    IntroducesStartupLiability,
    PyramidApparitionCoverage(PyramidApparitionCoverageV1),
    ActionSupplyFacts {
        opening_once_options: u8,
        delayed_per_turn_sources: u8,
        same_turn_burst_sources: u8,
        triggered_repeatable_sources: u8,
        additional_play_sources: u8,
        cost_or_resource_compression_sources: u8,
        potentially_recursive_sources: u8,
    },
    PandoraOfferFacts {
        starter_strikes: usize,
        starter_defends: usize,
        transform_targets: usize,
        transform_share_percent: u8,
        horizon: PandoraOfferHorizonV1,
        high_variance: bool,
    },
    Skip,
    Unknown,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BossRelicAdmission {
    pub relic: Option<RelicId>,
    pub class: BossRelicAdmissionClass,
    pub lane: BossRelicAdmissionLane,
    pub burden: BossRelicAdmissionBurden,
    pub reasons: Vec<BossRelicAdmissionReason>,
}

impl BossRelicAdmissionClass {
    fn order_rank(self) -> u8 {
        match self {
            BossRelicAdmissionClass::EnergyWithConstraint => 0,
            BossRelicAdmissionClass::StrategicPower => 1,
            BossRelicAdmissionClass::TransformAgency => 2,
            BossRelicAdmissionClass::DeckCleanup => 3,
            BossRelicAdmissionClass::StarterUpgrade => 4,
            BossRelicAdmissionClass::LowDownsideValue => 5,
            BossRelicAdmissionClass::RouteValue => 6,
            BossRelicAdmissionClass::CurseDebt => 7,
            BossRelicAdmissionClass::Unknown => 8,
            BossRelicAdmissionClass::Skip => 9,
        }
    }
}

impl BossRelicAdmissionLane {
    fn order_rank(self) -> u8 {
        match self {
            BossRelicAdmissionLane::Mainline => 0,
            BossRelicAdmissionLane::Probe => 1,
            BossRelicAdmissionLane::Skip => 2,
        }
    }
}

impl BossRelicAdmissionBurden {
    fn order_rank(self) -> u8 {
        match self {
            BossRelicAdmissionBurden::None => 0,
            BossRelicAdmissionBurden::AddedRunDebt => 1,
            BossRelicAdmissionBurden::IntroducedStartupLiability => 2,
        }
    }
}

pub fn boss_relic_admission_order_rank(admission: &BossRelicAdmission) -> u8 {
    admission.lane.order_rank() * 80
        + explicit_priority_rank(admission) * 40
        + admission.burden.order_rank() * 10
        + admission.class.order_rank()
}

fn explicit_priority_rank(admission: &BossRelicAdmission) -> u8 {
    if admission
        .reasons
        .contains(&BossRelicAdmissionReason::Act2EnergyGap)
    {
        0
    } else {
        1
    }
}

pub fn skip_boss_relic_admission() -> BossRelicAdmission {
    BossRelicAdmission {
        relic: None,
        class: BossRelicAdmissionClass::Skip,
        lane: BossRelicAdmissionLane::Skip,
        burden: BossRelicAdmissionBurden::None,
        reasons: vec![BossRelicAdmissionReason::Skip],
    }
}

pub fn assess_boss_relic_admission(run_state: &RunState, relic: RelicId) -> BossRelicAdmission {
    let plan = DeckPlanSnapshot::from_run_state(run_state);
    let facts = plan.run_facts;
    let mut reasons = Vec::new();

    let class = match relic {
        RelicId::BlackBlood
        | RelicId::RingOfTheSerpent
        | RelicId::FrozenCore
        | RelicId::HolyWater => {
            reasons.push(BossRelicAdmissionReason::StarterUpgrade);
            BossRelicAdmissionClass::StarterUpgrade
        }
        RelicId::TinyHouse => {
            reasons.push(BossRelicAdmissionReason::BroadValue);
            BossRelicAdmissionClass::LowDownsideValue
        }
        RelicId::EmptyCage => {
            reasons.push(BossRelicAdmissionReason::RemovesCards {
                starter_basics: facts.starter_basic_count,
                curses: facts.curse_count,
            });
            BossRelicAdmissionClass::DeckCleanup
        }
        RelicId::CoffeeDripper => energy_class(&mut reasons, BossRelicAdmissionReason::RestLocked),
        RelicId::FusionHammer => energy_class(&mut reasons, BossRelicAdmissionReason::SmithLocked),
        RelicId::Ectoplasm => energy_class(&mut reasons, BossRelicAdmissionReason::GoldLocked),
        RelicId::Sozu => energy_class(&mut reasons, BossRelicAdmissionReason::PotionLocked),
        RelicId::BustedCrown => {
            energy_class(&mut reasons, BossRelicAdmissionReason::RewardWidthLoss)
        }
        RelicId::RunicDome => {
            energy_class(&mut reasons, BossRelicAdmissionReason::EnemyIntentHidden)
        }
        RelicId::VelvetChoker => energy_class(&mut reasons, BossRelicAdmissionReason::PlayLimit),
        RelicId::MarkOfPain => energy_class(&mut reasons, BossRelicAdmissionReason::AddsCurse),
        RelicId::PhilosopherStone => {
            reasons.push(BossRelicAdmissionReason::EnergyGain);
            BossRelicAdmissionClass::EnergyWithConstraint
        }
        RelicId::BlackStar | RelicId::SacredBark | RelicId::SlaversCollar => {
            reasons.push(BossRelicAdmissionReason::RouteDependent);
            BossRelicAdmissionClass::RouteValue
        }
        RelicId::CursedKey => {
            reasons.push(BossRelicAdmissionReason::EnergyGain);
            reasons.push(BossRelicAdmissionReason::FutureChestCurses);
            BossRelicAdmissionClass::CurseDebt
        }
        RelicId::CallingBell => {
            reasons.push(BossRelicAdmissionReason::AddsCurse);
            BossRelicAdmissionClass::CurseDebt
        }
        RelicId::Astrolabe | RelicId::PandorasBox => {
            reasons.push(BossRelicAdmissionReason::TransformsDeck);
            BossRelicAdmissionClass::TransformAgency
        }
        RelicId::RunicPyramid => {
            reasons.push(BossRelicAdmissionReason::HandRetention);
            BossRelicAdmissionClass::StrategicPower
        }
        RelicId::SneckoEye => {
            reasons.push(BossRelicAdmissionReason::SneckoConfusion);
            BossRelicAdmissionClass::StrategicPower
        }
        _ => {
            reasons.push(BossRelicAdmissionReason::Unknown);
            BossRelicAdmissionClass::Unknown
        }
    };
    let lane = lane_for_relic(run_state, &facts, relic, class, &mut reasons);
    let debt_projection = run_debt_projection_for_relic_v1(run_state, relic);
    let projected_startup = projected_startup_profile(run_state, relic);
    if relic == RelicId::PandorasBox {
        let profile = pandora_offer_profile_v1(run_state);
        reasons.push(BossRelicAdmissionReason::PandoraOfferFacts {
            starter_strikes: profile.starter_strikes,
            starter_defends: profile.starter_defends,
            transform_targets: profile.transform_targets,
            transform_share_percent: profile.transform_share_percent,
            horizon: profile.horizon,
            high_variance: profile.high_variance,
        });
    }
    if relic == RelicId::RunicPyramid {
        reasons.push(BossRelicAdmissionReason::PyramidApparitionCoverage(
            projected_startup.pyramid_apparition_coverage,
        ));
    }
    if relic == RelicId::VelvetChoker {
        let supply = action_supply_profile_v1(run_state);
        if !supply.is_empty() {
            reasons.push(BossRelicAdmissionReason::ActionSupplyFacts {
                opening_once_options: supply.opening_once_options,
                delayed_per_turn_sources: supply.delayed_per_turn_sources,
                same_turn_burst_sources: supply.same_turn_burst_sources,
                triggered_repeatable_sources: supply.triggered_repeatable_sources,
                additional_play_sources: supply.additional_play_sources,
                cost_or_resource_compression_sources: supply.cost_or_resource_compression_sources,
                potentially_recursive_sources: supply.potentially_recursive_sources,
            });
        }
    }
    let burden = if debt_projection.added_contracts.is_empty() {
        BossRelicAdmissionBurden::None
    } else {
        reasons.push(BossRelicAdmissionReason::AddsRunDebt {
            contracts: debt_projection.added_contracts.len(),
        });
        BossRelicAdmissionBurden::AddedRunDebt
    };

    BossRelicAdmission {
        relic: Some(relic),
        class,
        lane,
        burden,
        reasons,
    }
}

fn projected_startup_profile(
    run_state: &RunState,
    relic: RelicId,
) -> crate::ai::deck_startup_profile_v1::DeckStartupProfileV1 {
    let mut projected_run = run_state.clone();
    if !projected_run.relics.iter().any(|state| state.id == relic) {
        projected_run.relics.push(RelicState::new(relic));
    }
    deck_startup_profile_v1(&projected_run)
}

fn lane_for_relic(
    run_state: &RunState,
    facts: &RunStrategicFacts,
    relic: RelicId,
    class: BossRelicAdmissionClass,
    reasons: &mut Vec<BossRelicAdmissionReason>,
) -> BossRelicAdmissionLane {
    if facts.has_act2_energy_gap() {
        if relic == RelicId::CoffeeDripper && coffee_dripper_no_rest_debt_high(run_state) {
            reasons.push(BossRelicAdmissionReason::NoRestDebt);
            return BossRelicAdmissionLane::Probe;
        }
        if is_act2_default_energy_relic(relic) {
            reasons.push(BossRelicAdmissionReason::Act2EnergyGap);
            return BossRelicAdmissionLane::Mainline;
        }
        if relic == RelicId::Sozu && !has_live_potion_synergy(run_state) {
            reasons.push(BossRelicAdmissionReason::Act2EnergyGap);
            return BossRelicAdmissionLane::Mainline;
        }
        if relic == RelicId::EmptyCage {
            reasons.push(BossRelicAdmissionReason::DoesNotSolveAct2EnergyGap);
            return BossRelicAdmissionLane::Probe;
        }
    }
    default_lane(class)
}

fn has_live_potion_synergy(run_state: &RunState) -> bool {
    run_state.relics.iter().any(|relic| {
        !relic.used_up
            && matches!(
                relic.id,
                RelicId::WhiteBeastStatue | RelicId::SacredBark | RelicId::PotionBelt
            )
    })
}

fn coffee_dripper_no_rest_debt_high(run_state: &RunState) -> bool {
    let hp_percent = hp_percent(run_state);
    let self_damage = self_damage_source_count(run_state);
    hp_percent < 45
        || (self_damage >= 2 && !has_combat_healing_plan(run_state))
        || (self_damage >= 1 && survival_support_count(run_state) <= 1 && hp_percent < 70)
}

fn hp_percent(run_state: &RunState) -> i32 {
    if run_state.max_hp <= 0 {
        return 0;
    }
    run_state.current_hp.max(0).saturating_mul(100) / run_state.max_hp
}

fn self_damage_source_count(run_state: &RunState) -> usize {
    run_state
        .master_deck
        .iter()
        .filter(|card| card_mechanics_profile_v1(card.id).self_damage_source)
        .count()
}

fn has_combat_healing_plan(run_state: &RunState) -> bool {
    run_state
        .master_deck
        .iter()
        .any(|card| card.id == CardId::Reaper)
}

fn survival_support_count(run_state: &RunState) -> usize {
    run_state
        .master_deck
        .iter()
        .filter(|card| {
            matches!(
                card.id,
                CardId::ShrugItOff
                    | CardId::FlameBarrier
                    | CardId::PowerThrough
                    | CardId::Impervious
                    | CardId::SecondWind
                    | CardId::TrueGrit
                    | CardId::Disarm
                    | CardId::Shockwave
                    | CardId::Uppercut
                    | CardId::Clothesline
                    | CardId::Intimidate
                    | CardId::GhostlyArmor
            )
        })
        .count()
}

fn is_act2_default_energy_relic(relic: RelicId) -> bool {
    matches!(
        relic,
        RelicId::CoffeeDripper
            | RelicId::CursedKey
            | RelicId::FusionHammer
            | RelicId::PhilosopherStone
    )
}

fn default_lane(class: BossRelicAdmissionClass) -> BossRelicAdmissionLane {
    match class {
        BossRelicAdmissionClass::StarterUpgrade
        | BossRelicAdmissionClass::LowDownsideValue
        | BossRelicAdmissionClass::DeckCleanup => BossRelicAdmissionLane::Mainline,
        BossRelicAdmissionClass::RouteValue
        | BossRelicAdmissionClass::EnergyWithConstraint
        | BossRelicAdmissionClass::StrategicPower
        | BossRelicAdmissionClass::CurseDebt
        | BossRelicAdmissionClass::TransformAgency
        | BossRelicAdmissionClass::Unknown => BossRelicAdmissionLane::Probe,
        BossRelicAdmissionClass::Skip => BossRelicAdmissionLane::Skip,
    }
}

fn energy_class(
    reasons: &mut Vec<BossRelicAdmissionReason>,
    constraint: BossRelicAdmissionReason,
) -> BossRelicAdmissionClass {
    reasons.push(BossRelicAdmissionReason::EnergyGain);
    reasons.push(constraint);
    BossRelicAdmissionClass::EnergyWithConstraint
}

pub fn render_boss_relic_admission_compact(admission: &BossRelicAdmission) -> String {
    let reasons = admission
        .reasons
        .iter()
        .take(3)
        .map(reason_tag)
        .collect::<Vec<_>>();
    let header = format!(
        "{} {}",
        lane_tag(admission.lane),
        class_tag(admission.class)
    );
    if reasons.is_empty() {
        header
    } else {
        format!("{header} | {}", reasons.join(" "))
    }
}

fn lane_tag(lane: BossRelicAdmissionLane) -> &'static str {
    match lane {
        BossRelicAdmissionLane::Mainline => "Mainline",
        BossRelicAdmissionLane::Probe => "Probe",
        BossRelicAdmissionLane::Skip => "Skip",
    }
}

fn class_tag(class: BossRelicAdmissionClass) -> &'static str {
    match class {
        BossRelicAdmissionClass::StarterUpgrade => "StarterUpgrade",
        BossRelicAdmissionClass::LowDownsideValue => "LowDownside",
        BossRelicAdmissionClass::DeckCleanup => "DeckCleanup",
        BossRelicAdmissionClass::RouteValue => "RouteValue",
        BossRelicAdmissionClass::EnergyWithConstraint => "EnergyRisk",
        BossRelicAdmissionClass::StrategicPower => "Strategic",
        BossRelicAdmissionClass::CurseDebt => "CurseDebt",
        BossRelicAdmissionClass::TransformAgency => "Transform",
        BossRelicAdmissionClass::Unknown => "Unknown",
        BossRelicAdmissionClass::Skip => "Skip",
    }
}

fn reason_tag(reason: &BossRelicAdmissionReason) -> String {
    match reason {
        BossRelicAdmissionReason::StarterUpgrade => "starter-upgrade".to_string(),
        BossRelicAdmissionReason::BroadValue => "broad-value".to_string(),
        BossRelicAdmissionReason::RemovesCards {
            starter_basics,
            curses,
        } => format!("remove-targets:starter={starter_basics},curse={curses}"),
        BossRelicAdmissionReason::EnergyGain => "+energy".to_string(),
        BossRelicAdmissionReason::RouteDependent => "route-dependent".to_string(),
        BossRelicAdmissionReason::RestLocked => "no-rest".to_string(),
        BossRelicAdmissionReason::SmithLocked => "no-smith".to_string(),
        BossRelicAdmissionReason::GoldLocked => "no-gold".to_string(),
        BossRelicAdmissionReason::PotionLocked => "no-potions".to_string(),
        BossRelicAdmissionReason::RewardWidthLoss => "less-card-choice".to_string(),
        BossRelicAdmissionReason::EnemyIntentHidden => "hide-intents".to_string(),
        BossRelicAdmissionReason::PlayLimit => "play-limit".to_string(),
        BossRelicAdmissionReason::AddsCurse => "adds-curse".to_string(),
        BossRelicAdmissionReason::FutureChestCurses => "chest-curses".to_string(),
        BossRelicAdmissionReason::TransformsDeck => "transforms-deck".to_string(),
        BossRelicAdmissionReason::HandRetention => "retain-hand".to_string(),
        BossRelicAdmissionReason::SneckoConfusion => "confusion".to_string(),
        BossRelicAdmissionReason::Act2EnergyGap => "act2-energy-gap".to_string(),
        BossRelicAdmissionReason::DoesNotSolveAct2EnergyGap => "misses-act2-energy-gap".to_string(),
        BossRelicAdmissionReason::NoRestDebt => "no-rest-debt".to_string(),
        BossRelicAdmissionReason::AddsRunDebt { contracts } => {
            format!("adds-run-debt:{contracts}")
        }
        BossRelicAdmissionReason::IntroducesStartupLiability => "startup-liability".to_string(),
        BossRelicAdmissionReason::PyramidApparitionCoverage(coverage) => {
            format!("apparition-coverage:{}", coverage.label())
        }
        BossRelicAdmissionReason::ActionSupplyFacts {
            opening_once_options,
            delayed_per_turn_sources,
            same_turn_burst_sources,
            triggered_repeatable_sources,
            additional_play_sources,
            cost_or_resource_compression_sources,
            potentially_recursive_sources,
        } => format!(
            "action-supply:opening={opening_once_options},delayed={delayed_per_turn_sources},burst={same_turn_burst_sources},triggered={triggered_repeatable_sources},extra={additional_play_sources},compression={cost_or_resource_compression_sources},recursive={potentially_recursive_sources}"
        ),
        BossRelicAdmissionReason::PandoraOfferFacts {
            starter_strikes,
            starter_defends,
            transform_targets,
            transform_share_percent,
            horizon,
            high_variance,
        } => format!(
            "pandora-offer:targets={transform_targets},strikes={starter_strikes},defends={starter_defends},share={transform_share_percent}%,horizon={horizon:?},high-variance={high_variance}"
        ),
        BossRelicAdmissionReason::Skip => "skip".to_string(),
        BossRelicAdmissionReason::Unknown => "no-model".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::relics::RelicState;
    use crate::runtime::combat::CombatCard;
    use crate::state::run::RunState;

    #[test]
    fn coffee_dripper_solves_act2_energy_gap_before_strategic_power() {
        let run = RunState::new(1552225673, 0, false, "Ironclad");

        let coffee = assess_boss_relic_admission(&run, RelicId::CoffeeDripper);
        let pyramid = assess_boss_relic_admission(&run, RelicId::RunicPyramid);

        assert_eq!(coffee.lane, BossRelicAdmissionLane::Mainline);
        assert!(coffee
            .reasons
            .contains(&BossRelicAdmissionReason::Act2EnergyGap));
        assert!(
            boss_relic_admission_order_rank(&coffee) < boss_relic_admission_order_rank(&pyramid),
            "Act2 energy gap should let Coffee Dripper outrank generic strategic power"
        );
    }

    #[test]
    fn coffee_dripper_rest_lock_debt_yields_to_strategic_power() {
        let mut run = RunState::new(1552225673, 0, false, "Ironclad");
        run.current_hp = 9;
        run.max_hp = 80;
        run.master_deck
            .push(CombatCard::new(CardId::Bloodletting, 1001));

        let coffee = assess_boss_relic_admission(&run, RelicId::CoffeeDripper);
        let pyramid = assess_boss_relic_admission(&run, RelicId::RunicPyramid);

        assert_eq!(coffee.lane, BossRelicAdmissionLane::Probe);
        assert!(coffee
            .reasons
            .contains(&BossRelicAdmissionReason::NoRestDebt));
        assert!(
            boss_relic_admission_order_rank(&pyramid) < boss_relic_admission_order_rank(&coffee),
            "low HP plus self-damage should prevent Coffee Dripper's energy gap shortcut from outranking Pyramid"
        );
    }

    #[test]
    fn sozu_solves_unconstrained_act2_energy_gap_before_black_blood() {
        let run = RunState::new(1552225673, 0, false, "Ironclad");

        let sozu = assess_boss_relic_admission(&run, RelicId::Sozu);
        let black_blood = assess_boss_relic_admission(&run, RelicId::BlackBlood);

        assert_eq!(sozu.lane, BossRelicAdmissionLane::Mainline);
        assert!(sozu
            .reasons
            .contains(&BossRelicAdmissionReason::Act2EnergyGap));
        assert!(
            boss_relic_admission_order_rank(&sozu) < boss_relic_admission_order_rank(&black_blood),
            "an unconstrained energy solution should outrank a starter upgrade"
        );
    }

    #[test]
    fn potion_synergy_relics_keep_sozu_out_of_act2_energy_shortcut() {
        for synergy in [
            RelicId::WhiteBeastStatue,
            RelicId::SacredBark,
            RelicId::PotionBelt,
        ] {
            let mut run = RunState::new(1552225673, 0, false, "Ironclad");
            run.relics.push(RelicState::new(synergy));

            let sozu = assess_boss_relic_admission(&run, RelicId::Sozu);
            let black_blood = assess_boss_relic_admission(&run, RelicId::BlackBlood);

            assert_eq!(sozu.lane, BossRelicAdmissionLane::Probe, "{synergy:?}");
            assert!(
                !sozu
                    .reasons
                    .contains(&BossRelicAdmissionReason::Act2EnergyGap),
                "{synergy:?}"
            );
            assert!(
                boss_relic_admission_order_rank(&black_blood)
                    < boss_relic_admission_order_rank(&sozu),
                "{synergy:?} should keep Sozu behind the unconstrained starter upgrade"
            );
        }
    }

    #[test]
    fn strategic_power_defaults_to_probe() {
        let mut run = RunState::new(1552225673, 0, false, "Ironclad");
        run.act_num = 2;

        let pyramid = assess_boss_relic_admission(&run, RelicId::RunicPyramid);

        assert_eq!(pyramid.lane, BossRelicAdmissionLane::Probe);
    }

    #[test]
    fn pyramid_reports_repairable_coverage_without_mutating_run_or_adding_burden() {
        let mut run = RunState::new(1552225673, 0, false, "Ironclad");
        run.act_num = 2;
        run.master_deck
            .push(CombatCard::new(CardId::Apparition, 1001));
        let mut armaments = CombatCard::new(CardId::Armaments, 1002);
        armaments.upgrades = 1;
        run.master_deck.push(armaments);
        let relic_count = run.relics.len();

        let pyramid = assess_boss_relic_admission(&run, RelicId::RunicPyramid);

        assert_eq!(pyramid.lane, BossRelicAdmissionLane::Probe);
        assert_eq!(pyramid.burden, BossRelicAdmissionBurden::None);
        assert!(pyramid
            .reasons
            .contains(&BossRelicAdmissionReason::PyramidApparitionCoverage(
                PyramidApparitionCoverageV1::CombatRepairAvailable,
            )));
        assert!(!pyramid
            .reasons
            .contains(&BossRelicAdmissionReason::IntroducesStartupLiability));
        assert!(render_boss_relic_admission_compact(&pyramid)
            .contains("apparition-coverage:combat-repair"));
        assert_eq!(run.relics.len(), relic_count);
    }

    #[test]
    fn repairable_pyramid_competes_normally_inside_probe_lane() {
        let mut run = RunState::new(1552225673, 0, false, "Ironclad");
        run.act_num = 2;
        run.master_deck
            .push(CombatCard::new(CardId::Apparition, 1001));
        let mut armaments = CombatCard::new(CardId::Armaments, 1002);
        armaments.upgrades = 1;
        run.master_deck.push(armaments);

        let pyramid = assess_boss_relic_admission(&run, RelicId::RunicPyramid);
        let bark = assess_boss_relic_admission(&run, RelicId::SacredBark);
        let sozu = assess_boss_relic_admission(&run, RelicId::Sozu);

        assert!(boss_relic_admission_order_rank(&pyramid) < boss_relic_admission_order_rank(&bark));
        assert!(boss_relic_admission_order_rank(&bark) < boss_relic_admission_order_rank(&sozu));
    }

    #[test]
    fn choker_reports_opening_once_supply_without_repeatable_exposure() {
        let baseline = RunState::new(20260711002, 0, false, "Ironclad");
        let mut with_enchiridion = baseline.clone();
        with_enchiridion
            .relics
            .push(RelicState::new(RelicId::Enchiridion));

        let baseline_choker = assess_boss_relic_admission(&baseline, RelicId::VelvetChoker);
        let choker = assess_boss_relic_admission(&with_enchiridion, RelicId::VelvetChoker);

        assert!(choker
            .reasons
            .contains(&BossRelicAdmissionReason::ActionSupplyFacts {
                opening_once_options: 1,
                delayed_per_turn_sources: 0,
                same_turn_burst_sources: 0,
                triggered_repeatable_sources: 0,
                additional_play_sources: 0,
                cost_or_resource_compression_sources: 0,
                potentially_recursive_sources: 0,
            }));
        assert_eq!(choker.burden, BossRelicAdmissionBurden::AddedRunDebt);
        assert_eq!(
            boss_relic_admission_order_rank(&choker),
            boss_relic_admission_order_rank(&baseline_choker),
            "opening-once supply is evidence, not a new ordering penalty"
        );
    }

    #[test]
    fn choker_reports_repeatable_and_compressed_action_supply_as_facts() {
        let mut run = RunState::new(20260711002, 0, false, "Ironclad");
        run.relics.push(RelicState::new(RelicId::DeadBranch));
        run.add_card_to_deck(CardId::Corruption);

        let choker = assess_boss_relic_admission(&run, RelicId::VelvetChoker);

        assert!(choker
            .reasons
            .contains(&BossRelicAdmissionReason::ActionSupplyFacts {
                opening_once_options: 0,
                delayed_per_turn_sources: 0,
                same_turn_burst_sources: 0,
                triggered_repeatable_sources: 1,
                additional_play_sources: 0,
                cost_or_resource_compression_sources: 1,
                potentially_recursive_sources: 1,
            }));
    }

    #[test]
    fn pandora_offer_facts_do_not_change_admission_order_rank() {
        let mut many = RunState::new(1, 0, false, "Ironclad");
        many.act_num = 1;
        many.master_deck = (0..4)
            .map(|index| CombatCard::new(CardId::Strike, index + 1))
            .chain((0..4).map(|index| CombatCard::new(CardId::Defend, index + 10)))
            .collect();
        let mut few = many.clone();
        few.master_deck = vec![
            CombatCard::new(CardId::Strike, 1),
            CombatCard::new(CardId::Defend, 2),
        ];

        let many_admission = assess_boss_relic_admission(&many, RelicId::PandorasBox);
        let few_admission = assess_boss_relic_admission(&few, RelicId::PandorasBox);

        assert!(many_admission.reasons.iter().any(|reason| matches!(
            reason,
            BossRelicAdmissionReason::PandoraOfferFacts {
                transform_targets: 8,
                ..
            }
        )));
        assert_eq!(
            boss_relic_admission_order_rank(&many_admission),
            boss_relic_admission_order_rank(&few_admission),
        );
    }

    #[test]
    fn pyramid_does_not_inherit_existing_choker_enchiridion_supply() {
        let mut run = RunState::new(20260711002, 0, false, "Ironclad");
        run.relics = vec![
            RelicState::new(RelicId::VelvetChoker),
            RelicState::new(RelicId::Enchiridion),
        ];

        let pyramid = assess_boss_relic_admission(&run, RelicId::RunicPyramid);

        assert!(!pyramid
            .reasons
            .iter()
            .any(|reason| matches!(reason, BossRelicAdmissionReason::ActionSupplyFacts { .. })));
        assert!(!render_boss_relic_admission_compact(&pyramid).contains("opening-action-budget"));
        assert_eq!(pyramid.burden, BossRelicAdmissionBurden::None);
    }

    #[test]
    fn energy_gap_mainline_stays_ahead_of_burden_free_probe() {
        let run = RunState::new(1552225673, 0, false, "Ironclad");

        let sozu = assess_boss_relic_admission(&run, RelicId::Sozu);
        let bark = assess_boss_relic_admission(&run, RelicId::SacredBark);

        assert_eq!(sozu.lane, BossRelicAdmissionLane::Mainline);
        assert_eq!(bark.lane, BossRelicAdmissionLane::Probe);
        assert!(boss_relic_admission_order_rank(&sozu) < boss_relic_admission_order_rank(&bark));
    }
}
