use crate::content::cards::{get_card_definition, is_starter_basic, CardType};
use crate::content::relics::RelicId;
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
    Skip,
    Unknown,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BossRelicAdmission {
    pub relic: Option<RelicId>,
    pub class: BossRelicAdmissionClass,
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

pub fn boss_relic_admission_order_rank(admission: &BossRelicAdmission) -> u8 {
    admission.class.order_rank()
}

pub fn skip_boss_relic_admission() -> BossRelicAdmission {
    BossRelicAdmission {
        relic: None,
        class: BossRelicAdmissionClass::Skip,
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

    BossRelicAdmission {
        relic: Some(relic),
        class,
        reasons,
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
    if reasons.is_empty() {
        class_tag(admission.class).to_string()
    } else {
        format!("{} | {}", class_tag(admission.class), reasons.join(" "))
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
        BossRelicAdmissionReason::Skip => "skip".to_string(),
        BossRelicAdmissionReason::Unknown => "no-model".to_string(),
    }
}
