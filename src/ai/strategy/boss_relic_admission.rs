use crate::content::cards::{get_card_definition, is_starter_basic, CardType};
use crate::content::relics::{energy_master_delta, RelicId};
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
    Skip,
    Unknown,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BossRelicAdmission {
    pub relic: Option<RelicId>,
    pub class: BossRelicAdmissionClass,
    pub lane: BossRelicAdmissionLane,
    pub reasons: Vec<BossRelicAdmissionReason>,
}

impl BossRelicAdmissionClass {
    fn order_rank(self) -> u8 {
        match self {
            BossRelicAdmissionClass::StarterUpgrade => 0,
            BossRelicAdmissionClass::LowDownsideValue => 1,
            BossRelicAdmissionClass::DeckCleanup => 2,
            BossRelicAdmissionClass::EnergyWithConstraint => 3,
            BossRelicAdmissionClass::RouteValue => 4,
            BossRelicAdmissionClass::StrategicPower => 5,
            BossRelicAdmissionClass::CurseDebt => 6,
            BossRelicAdmissionClass::TransformAgency => 7,
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

pub fn boss_relic_admission_order_rank(admission: &BossRelicAdmission) -> u8 {
    admission.lane.order_rank() * 16 + admission.class.order_rank()
}

pub fn skip_boss_relic_admission() -> BossRelicAdmission {
    BossRelicAdmission {
        relic: None,
        class: BossRelicAdmissionClass::Skip,
        lane: BossRelicAdmissionLane::Skip,
        reasons: vec![BossRelicAdmissionReason::Skip],
    }
}

pub fn assess_boss_relic_admission(run_state: &RunState, relic: RelicId) -> BossRelicAdmission {
    let starter_basics = run_state
        .master_deck
        .iter()
        .filter(|card| is_starter_basic(card.id))
        .count();
    let curses = run_state
        .master_deck
        .iter()
        .filter(|card| get_card_definition(card.id).card_type == CardType::Curse)
        .count();
    let context = BossRelicAdmissionContext::from_run_state(run_state);
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
                starter_basics,
                curses,
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
    let lane = lane_for_relic(&context, relic, class, &mut reasons);

    BossRelicAdmission {
        relic: Some(relic),
        class,
        lane,
        reasons,
    }
}

struct BossRelicAdmissionContext {
    entering_act: u8,
    has_energy_relic: bool,
}

impl BossRelicAdmissionContext {
    fn from_run_state(run_state: &RunState) -> Self {
        Self {
            entering_act: run_state.act_num.saturating_add(1),
            has_energy_relic: run_state
                .relics
                .iter()
                .any(|relic| energy_master_delta(relic.id) > 0),
        }
    }

    fn has_act2_energy_gap(&self) -> bool {
        self.entering_act == 2 && !self.has_energy_relic
    }
}

fn lane_for_relic(
    context: &BossRelicAdmissionContext,
    relic: RelicId,
    class: BossRelicAdmissionClass,
    reasons: &mut Vec<BossRelicAdmissionReason>,
) -> BossRelicAdmissionLane {
    if context.has_act2_energy_gap() {
        if is_act2_default_energy_relic(relic) {
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

fn is_act2_default_energy_relic(relic: RelicId) -> bool {
    matches!(
        relic,
        RelicId::CursedKey | RelicId::FusionHammer | RelicId::PhilosopherStone
    )
}

fn default_lane(class: BossRelicAdmissionClass) -> BossRelicAdmissionLane {
    match class {
        BossRelicAdmissionClass::StarterUpgrade
        | BossRelicAdmissionClass::LowDownsideValue
        | BossRelicAdmissionClass::DeckCleanup
        | BossRelicAdmissionClass::StrategicPower => BossRelicAdmissionLane::Mainline,
        BossRelicAdmissionClass::RouteValue
        | BossRelicAdmissionClass::EnergyWithConstraint
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
        BossRelicAdmissionReason::Skip => "skip".to_string(),
        BossRelicAdmissionReason::Unknown => "no-model".to_string(),
    }
}
