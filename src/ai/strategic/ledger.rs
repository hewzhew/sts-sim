use super::StrategicSnapshot;
use crate::ai::deck_startup_profile_v1::DeckStartupProfileV1;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum StrategicJob {
    Frontload,
    Block,
    Scaling,
    DrawEnergy,
    Consistency,
    EnemyStrengthDown,
    StatusControl,
    ExhaustAccess,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum StrategicDebt {
    CycleTime,
    SetupDebt,
    UpgradeDebt,
    PayoffWithoutEnabler,
    CurseOrStarterDensity,
    CombatShapeRisk,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum StrategicBossTax {
    AwakenedPowerTax,
    AwakenedPhaseTwoBlock,
    AutomatonHyperbeamPlan,
    AutomatonOrbControl,
    TimeEaterCardCount,
    ChampExecutePlan,
    CollectorMinionPlan,
    CollectorTurnFourDebuffPlan,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PressureHorizon {
    Immediate,
    NextCombat,
    VisibleRoute,
    ActBoss,
    LongTerm,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PressureKind {
    MissingJob(StrategicJob),
    DeckDebt(StrategicDebt),
    BossTax(StrategicBossTax),
    RouteRisk,
    EconomyNeed,
    UpgradeNeed,
    BranchDiversityNeed,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct PressureItem {
    pub id: String,
    pub kind: PressureKind,
    pub horizon: PressureHorizon,
    pub severity: f32,
    pub confidence: f32,
    pub evidence: Vec<String>,
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct PressureLedger {
    pub items: Vec<PressureItem>,
}

impl PressureLedger {
    pub fn push(
        &mut self,
        id: impl Into<String>,
        kind: PressureKind,
        horizon: PressureHorizon,
        severity: f32,
        confidence: f32,
        evidence: Vec<String>,
    ) {
        self.items.push(PressureItem {
            id: id.into(),
            kind,
            horizon,
            severity: severity.clamp(0.0, 1.0),
            confidence: confidence.clamp(0.0, 1.0),
            evidence,
        });
    }

    pub fn strongest(&self) -> Option<&PressureItem> {
        self.items.iter().max_by(|left, right| {
            left.severity
                .partial_cmp(&right.severity)
                .unwrap_or(std::cmp::Ordering::Equal)
        })
    }
}

pub fn ledger_from_snapshot(snapshot: &StrategicSnapshot) -> PressureLedger {
    let mut ledger = PressureLedger::default();

    for need in &snapshot.formation_needs {
        ledger.push(
            format!("missing_job:{need:?}"),
            PressureKind::MissingJob(*need),
            PressureHorizon::VisibleRoute,
            0.55,
            0.65,
            vec!["formation summary reports this current need".to_string()],
        );
    }

    let effective_cycle_pressure = if snapshot.deck.deck_size >= 40 {
        Some(0.85)
    } else if snapshot.deck.deck_size >= 34 {
        Some(0.70)
    } else if snapshot.deck.deck_size >= 28 {
        Some(0.50)
    } else {
        None
    };
    if let Some(severity) = effective_cycle_pressure {
        ledger.push(
            "deck_debt:cycle_time",
            PressureKind::DeckDebt(StrategicDebt::CycleTime),
            PressureHorizon::LongTerm,
            severity,
            0.65,
            vec![format!("deck_size={}", snapshot.deck.deck_size)],
        );
    }

    if snapshot.deck.curses > 0
        || snapshot.deck.starter_strikes + snapshot.deck.starter_defends >= 7
    {
        ledger.push(
            "deck_debt:curse_or_starter_density",
            PressureKind::DeckDebt(StrategicDebt::CurseOrStarterDensity),
            PressureHorizon::VisibleRoute,
            0.55,
            0.70,
            vec![format!(
                "curses={} starter_cards={}",
                snapshot.deck.curses,
                snapshot.deck.starter_strikes + snapshot.deck.starter_defends
            )],
        );
    }

    if snapshot.deck.draw_sources == 0 && snapshot.deck.deck_size >= 18 {
        ledger.push(
            "missing_job:draw_energy",
            PressureKind::MissingJob(StrategicJob::DrawEnergy),
            PressureHorizon::VisibleRoute,
            0.65,
            0.70,
            vec!["deck has no explicit draw source at this abstraction level".to_string()],
        );
    }
    if snapshot.deck.deck_size >= 24 && snapshot.deck.draw_sources <= 1 {
        ledger.push(
            "deck_debt:low_access_large_deck",
            PressureKind::DeckDebt(StrategicDebt::CycleTime),
            PressureHorizon::VisibleRoute,
            0.60,
            0.70,
            vec![format!(
                "deck_size={} draw_sources={}",
                snapshot.deck.deck_size, snapshot.deck.draw_sources
            )],
        );
    }
    if snapshot.deck.status_generators > 0 && snapshot.deck.status_payoffs == 0 {
        ledger.push(
            "deck_debt:status_without_digest",
            PressureKind::DeckDebt(StrategicDebt::CombatShapeRisk),
            PressureHorizon::VisibleRoute,
            (snapshot.deck.status_generators as f32 / 3.0).clamp(0.35, 0.80),
            0.70,
            vec![format!(
                "status_generators={} status_payoffs={}",
                snapshot.deck.status_generators, snapshot.deck.status_payoffs
            )],
        );
    }
    if snapshot.deck.exhaust_payoffs > 0 && snapshot.deck.exhaust_generators == 0 {
        ledger.push(
            "deck_debt:exhaust_payoff_without_enabler",
            PressureKind::DeckDebt(StrategicDebt::PayoffWithoutEnabler),
            PressureHorizon::VisibleRoute,
            (snapshot.deck.exhaust_payoffs as f32 / 3.0).clamp(0.35, 0.75),
            0.70,
            vec![format!(
                "exhaust_payoffs={} exhaust_generators={}",
                snapshot.deck.exhaust_payoffs, snapshot.deck.exhaust_generators
            )],
        );
    }
    if snapshot.deck.strength_payoffs > 0
        && snapshot.deck.strength_sources == 0
        && snapshot.deck.convertible_strength_sources > 0
    {
        ledger.push(
            "deck_debt:strength_payoff_without_stable_source",
            PressureKind::DeckDebt(StrategicDebt::PayoffWithoutEnabler),
            PressureHorizon::VisibleRoute,
            (snapshot.deck.strength_payoffs as f32 / 4.0).clamp(0.25, 0.60),
            0.75,
            vec![format!(
                "strength_payoffs={} stable_sources={} temporary_bursts={} converters={} convertible_sources={}",
                snapshot.deck.strength_payoffs,
                snapshot.deck.strength_sources,
                snapshot.deck.temporary_strength_bursts,
                snapshot.deck.strength_converters,
                snapshot.deck.convertible_strength_sources
            )],
        );
    } else if snapshot.deck.strength_payoffs > 0 && snapshot.deck.strength_sources == 0 {
        ledger.push(
            "deck_debt:strength_payoff_without_source",
            PressureKind::DeckDebt(StrategicDebt::PayoffWithoutEnabler),
            PressureHorizon::VisibleRoute,
            (snapshot.deck.strength_payoffs as f32 / 3.0).clamp(0.35, 0.75),
            0.70,
            vec![format!(
                "strength_payoffs={} strength_sources={}",
                snapshot.deck.strength_payoffs, snapshot.deck.strength_sources
            )],
        );
    }

    if let Some(route) = &snapshot.route {
        let route_pressure = (route.avoid_damage + (1.0 - route.can_take_elite)).clamp(0.0, 1.0);
        if route_pressure >= 0.45 {
            ledger.push(
                "route_risk:visible_pressure",
                PressureKind::RouteRisk,
                PressureHorizon::VisibleRoute,
                route_pressure,
                0.55,
                vec![format!(
                    "avoid_damage={:.2} can_take_elite={:.2}",
                    route.avoid_damage, route.can_take_elite
                )],
            );
        }
        if route.need_upgrade >= 0.55 {
            ledger.push(
                "upgrade_need:visible_route",
                PressureKind::UpgradeNeed,
                PressureHorizon::VisibleRoute,
                route.need_upgrade,
                0.55,
                vec![format!("need_upgrade={:.2}", route.need_upgrade)],
            );
        }
    }

    match snapshot.boss.as_deref() {
        Some("AwakenedOne") => {
            if snapshot.deck.powers > 0 {
                ledger.push(
                    "boss_tax:awakened_power_tax",
                    PressureKind::BossTax(StrategicBossTax::AwakenedPowerTax),
                    PressureHorizon::ActBoss,
                    (snapshot.deck.powers as f32 / 4.0).clamp(0.35, 0.90),
                    0.75,
                    vec![format!("power_count={}", snapshot.deck.powers)],
                );
            }
            ledger.push(
                "boss_tax:awakened_phase_two_block",
                PressureKind::BossTax(StrategicBossTax::AwakenedPhaseTwoBlock),
                PressureHorizon::ActBoss,
                0.65,
                0.70,
                vec!["Awakened One phase two asks for a real block plan".to_string()],
            );
        }
        Some("Automaton") => {
            ledger.push(
                "boss_tax:automaton_hyperbeam_plan",
                PressureKind::BossTax(StrategicBossTax::AutomatonHyperbeamPlan),
                PressureHorizon::ActBoss,
                0.75,
                0.70,
                vec!["Bronze Automaton asks for a hyperbeam mitigation plan".to_string()],
            );
            ledger.push(
                "boss_tax:automaton_orb_control",
                PressureKind::BossTax(StrategicBossTax::AutomatonOrbControl),
                PressureHorizon::ActBoss,
                0.60,
                0.65,
                vec!["Bronze Automaton asks for orb control and stasis recovery".to_string()],
            );
        }
        Some("TimeEater") => {
            ledger.push(
                "boss_tax:time_eater_card_count",
                PressureKind::BossTax(StrategicBossTax::TimeEaterCardCount),
                PressureHorizon::ActBoss,
                0.65,
                0.70,
                vec!["Time Eater taxes low-impact card spam".to_string()],
            );
        }
        Some("TheChamp") => {
            ledger.push(
                "boss_tax:champ_execute_plan",
                PressureKind::BossTax(StrategicBossTax::ChampExecutePlan),
                PressureHorizon::ActBoss,
                0.60,
                0.65,
                vec!["The Champ asks for execute-phase mitigation or scaling".to_string()],
            );
        }
        Some("Collector") => {
            ledger.push(
                "boss_tax:collector_minion_plan",
                PressureKind::BossTax(StrategicBossTax::CollectorMinionPlan),
                PressureHorizon::ActBoss,
                0.70,
                0.70,
                vec!["Collector asks for minion control or efficient AOE".to_string()],
            );
            ledger.push(
                "boss_tax:collector_turn4_debuff_plan",
                PressureKind::BossTax(StrategicBossTax::CollectorTurnFourDebuffPlan),
                PressureHorizon::ActBoss,
                0.55,
                0.65,
                vec!["Collector turn four debuff asks for mitigation or tempo".to_string()],
            );
        }
        _ => {}
    }

    ledger
}

pub fn add_startup_profile_pressure_to_ledger(
    ledger: &mut PressureLedger,
    startup: &DeckStartupProfileV1,
) {
    if startup.has_snecko_eye && startup.has_snecko_low_cost_volatility {
        ledger.push(
            "deck_debt:snecko_low_cost_volatility",
            PressureKind::DeckDebt(StrategicDebt::SetupDebt),
            PressureHorizon::VisibleRoute,
            (0.40 + startup.snecko_random_cost_debt as f32 * 0.15).clamp(0.40, 0.75),
            0.70,
            vec![
                "Snecko Eye randomizes card costs; low-cost decks lose startup reliability"
                    .to_string(),
                format!(
                    "low_cost_cards={} high_cost_cards={} random_cost_debt={}",
                    startup.low_cost_card_count,
                    startup.high_cost_card_count,
                    startup.snecko_random_cost_debt
                ),
            ],
        );
    }

    if startup.has_snecko_offering_reliability_debt {
        ledger.push(
            "deck_debt:snecko_offering_reliability",
            PressureKind::DeckDebt(StrategicDebt::SetupDebt),
            PressureHorizon::VisibleRoute,
            0.55,
            0.70,
            vec![
                "Offering draw/energy is less reliable when Snecko randomizes follow-up costs"
                    .to_string(),
                format!(
                    "raw_setup_payment={} effective_setup_payment={} raw_strong_draw={} effective_strong_draw={}",
                    startup.setup_payment,
                    startup.effective_setup_payment,
                    startup.strong_draw_count,
                    startup.effective_strong_draw_count
                ),
            ],
        );
    }
}
