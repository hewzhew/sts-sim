use serde::Serialize;

use crate::content::cards::{self, CardType};
use crate::runtime::combat::CombatState;
use crate::state::core::ClientInput;
use crate::state::EngineState;

use super::audit::TrajectoryOutcomeKind;
use super::dominance::TurnResourceSummary;
use super::exact_turn_solver::{ExactTurnSolution, TurnEndState};
use super::frontier_eval::{eval_frontier_state, FrontierEval};
use super::posture::posture_features;
use super::pressure::StatePressureFeatures;
use super::types::CombatCandidate;
use super::SearchExperimentFlags;

#[derive(Clone, Copy, Debug, Serialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub(crate) enum SurvivalJudgement {
    ForcedLoss,
    SevereRisk,
    RiskyButPlayable,
    Stable,
    Safe,
}

#[derive(Clone, Copy, Debug, Serialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub(crate) enum PositionClass {
    Collapsing,
    DefensiveBind,
    TempoNeutral,
    Stabilizing,
    WinningLine,
}

#[derive(Clone, Copy, Debug, Serialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub(crate) enum TerminalForecast {
    DiesInWindow,
    TimeoutUnknown,
    SurvivesWindow,
    LethalWin,
}

#[derive(Clone, Copy, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum CombatRegime {
    Crisis,
    Fragile,
    Contested,
    Advantage,
}

#[derive(Clone, Copy, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum DominanceClaim {
    StrictlyBetterInWindow,
    StrictlyWorseInWindow,
    Incomparable,
}

#[derive(Clone, Copy, Debug, Serialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ExactnessLevel {
    Exact,
    Bounded,
    Unavailable,
}

#[derive(Clone, Copy, Debug, Serialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ProposalClass {
    EndTurn,
    Attack,
    Block,
    SkillUtility,
    Power,
    Potion,
    Choice,
    Other,
}

impl ProposalClass {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::EndTurn => "end_turn",
            Self::Attack => "attack",
            Self::Block => "block",
            Self::SkillUtility => "skill_utility",
            Self::Power => "power",
            Self::Potion => "potion",
            Self::Choice => "choice",
            Self::Other => "other",
        }
    }
}

#[derive(Clone, Copy, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ProposalDisposition {
    FrontierChosen,
    Considered,
    ScreenedOut,
}

#[derive(Clone, Copy, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ScreenRejectionKind {
    UnsurvivableWhileSurvivorExists,
    ImmediateLethalWhenSaferExists,
    EndTurnWorseThanPlayableAlternative,
    DominatedFrontierSurvival,
    FragileRiskOutlier,
    TrimmedByScreeningWidth,
}

#[derive(Clone, Debug, Serialize, PartialEq)]
pub(crate) struct ScreenRejection {
    pub input: String,
    pub proposal_class: ProposalClass,
    pub frontier_outcome: DecisionOutcome,
    pub reason: ScreenRejectionKind,
}

impl ScreenRejectionKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::UnsurvivableWhileSurvivorExists => "unsurvivable_while_survivor_exists",
            Self::ImmediateLethalWhenSaferExists => "immediate_lethal_when_safer_exists",
            Self::EndTurnWorseThanPlayableAlternative => "end_turn_worse_than_playable_alternative",
            Self::DominatedFrontierSurvival => "dominated_frontier_survival",
            Self::FragileRiskOutlier => "fragile_risk_outlier",
            Self::TrimmedByScreeningWidth => "trimmed_by_screening_width",
        }
    }
}

#[derive(Clone, Debug, Serialize, PartialEq)]
pub(crate) struct ProposalTrace {
    pub input: String,
    pub proposal_class: ProposalClass,
    pub disposition: ProposalDisposition,
    pub frontier_outcome: DecisionOutcome,
    pub exact_outcome: Option<DecisionOutcome>,
    pub exact_confidence: ExactnessLevel,
    pub reasons: Vec<String>,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub(crate) struct ResourceDeltaSummary {
    pub spent_potions: u8,
    pub hp_lost: i32,
    pub exhausted_cards: u16,
    pub final_hp: i32,
    pub final_block: i32,
}

#[derive(Clone, Debug, Serialize, PartialEq)]
pub(crate) struct DecisionOutcome {
    pub survival: SurvivalJudgement,
    pub position: PositionClass,
    pub terminality: TerminalForecast,
    pub resource_delta: ResourceDeltaSummary,
    pub efficiency_score: f32,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub(crate) struct LethalWindow {
    pub incoming: i32,
    pub unblocked: i32,
    pub player_hp: i32,
    pub player_block: i32,
}

#[derive(Clone, Debug, Serialize, PartialEq)]
pub(crate) struct ExactTurnVerdict {
    pub best_first_input: Option<String>,
    pub best_outcome: Option<DecisionOutcome>,
    pub survival: SurvivalJudgement,
    pub dominance: DominanceClaim,
    pub lethal_window: Option<LethalWindow>,
    pub confidence: ExactnessLevel,
    pub truncated: bool,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum DecisionAuthority {
    Frontier,
    ExactTurnTakeover,
}

#[derive(Clone, Debug, Serialize, PartialEq)]
pub(crate) struct DecisionOutcomeBundle {
    pub frontier: DecisionOutcome,
    pub exact_best: Option<DecisionOutcome>,
}

#[derive(Clone, Debug, Serialize, PartialEq)]
pub(crate) struct DecisionTrace {
    pub regime: CombatRegime,
    pub frontier_choice: String,
    pub frontier_proposal_class: ProposalClass,
    pub exact_turn_verdict: ExactTurnVerdict,
    pub decision_outcomes: DecisionOutcomeBundle,
    pub chosen_action: String,
    pub chosen_by: DecisionAuthority,
    pub rejection_reasons: Vec<String>,
    pub screened_out: Vec<ScreenRejection>,
    pub why_not_others: Vec<ProposalTrace>,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub(crate) struct TakeoverPolicy {
    pub regime: CombatRegime,
    pub takeover_eligible: bool,
    pub takeover_reason: String,
    pub takeover_applied: bool,
    pub confidence: ExactnessLevel,
    pub dominance: DominanceClaim,
    pub frontier_survival: SurvivalJudgement,
    pub exact_survival: SurvivalJudgement,
}

pub(crate) fn classify_regime(combat: &CombatState) -> CombatRegime {
    let pressure = StatePressureFeatures::from_combat(combat);
    let posture = posture_features(combat);
    let living_monsters = living_monster_count(combat);

    if pressure.lethal_pressure
        || pressure.max_unblocked >= pressure.player_hp
        || pressure.unblocked >= pressure.player_hp.saturating_sub(3).max(1)
    {
        CombatRegime::Crisis
    } else if (pressure.urgent_pressure
        && (pressure.encounter_risk || posture.future_pollution_risk >= 6))
        || posture.future_pollution_risk >= 9
        || (living_monsters >= 2 && posture.immediate_survival_pressure >= 8)
    {
        CombatRegime::Fragile
    } else if pressure.urgent_pressure
        || posture.future_pollution_risk >= 4
        || posture.immediate_survival_pressure >= 4
        || living_monsters >= 2
    {
        CombatRegime::Contested
    } else {
        CombatRegime::Advantage
    }
}

pub(crate) fn frontier_outcome_from_candidate(
    combat_before: &CombatState,
    candidate: &CombatCandidate,
) -> DecisionOutcome {
    let exhausted_delta = candidate
        .next_combat
        .zones
        .exhaust_pile
        .len()
        .saturating_sub(combat_before.zones.exhaust_pile.len());
    let resources = TurnResourceSummary::at_frontier(
        candidate.frontier_combat.entities.player.current_hp,
        candidate.frontier_combat.entities.player.block,
    )
    .with_transition(
        &candidate.input,
        combat_before.entities.player.current_hp,
        candidate.next_combat.entities.player.current_hp,
        exhausted_delta,
    );
    outcome_from_frontier_state(
        &candidate.frontier_engine,
        &candidate.frontier_combat,
        resources,
        candidate.projection_truncated,
    )
}

pub(crate) fn outcome_from_end_state(end_state: &TurnEndState) -> DecisionOutcome {
    outcome_from_frontier_state(
        &end_state.frontier_engine,
        &end_state.frontier_combat,
        end_state.resources,
        false,
    )
}

pub(crate) fn compare_decision_outcomes(
    left: &DecisionOutcome,
    right: &DecisionOutcome,
) -> std::cmp::Ordering {
    left.survival
        .cmp(&right.survival)
        .then_with(|| left.position.cmp(&right.position))
        .then_with(|| left.terminality.cmp(&right.terminality))
        .then_with(|| compare_resource_delta(&left.resource_delta, &right.resource_delta))
        .then_with(|| {
            left.efficiency_score
                .partial_cmp(&right.efficiency_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        })
}

pub(crate) fn classify_proposal_class(combat: &CombatState, input: &ClientInput) -> ProposalClass {
    match input {
        ClientInput::EndTurn => ProposalClass::EndTurn,
        ClientInput::PlayCard { card_index, .. } => combat
            .zones
            .hand
            .get(*card_index)
            .map(|card| {
                let def = cards::get_card_definition(card.id);
                match def.card_type {
                    CardType::Attack => ProposalClass::Attack,
                    CardType::Power => ProposalClass::Power,
                    CardType::Skill => {
                        if def.base_block > 0 {
                            ProposalClass::Block
                        } else {
                            ProposalClass::SkillUtility
                        }
                    }
                    CardType::Status | CardType::Curse => ProposalClass::Other,
                }
            })
            .unwrap_or(ProposalClass::Other),
        ClientInput::UsePotion { .. } | ClientInput::DiscardPotion(_) => ProposalClass::Potion,
        ClientInput::SubmitCardChoice(_)
        | ClientInput::SubmitDiscoverChoice(_)
        | ClientInput::SubmitScryDiscard(_)
        | ClientInput::SubmitSelection(_)
        | ClientInput::SubmitHandSelect(_)
        | ClientInput::SubmitGridSelect(_)
        | ClientInput::SubmitDeckSelect(_)
        | ClientInput::SelectCard(_)
        | ClientInput::SubmitRelicChoice(_)
        | ClientInput::ClaimReward(_)
        | ClientInput::Proceed
        | ClientInput::Cancel => ProposalClass::Choice,
        _ => ProposalClass::Other,
    }
}

pub(crate) fn build_exact_turn_verdict(
    chosen_move: &ClientInput,
    frontier_outcome: &DecisionOutcome,
    solution: &ExactTurnSolution,
) -> ExactTurnVerdict {
    let best_outcome = solution
        .nondominated_end_states
        .first()
        .map(outcome_from_end_state);
    let confidence = if solution.best_first_input.is_none() {
        ExactnessLevel::Unavailable
    } else if solution.truncated || solution.cycle_cuts > 0 {
        ExactnessLevel::Bounded
    } else {
        ExactnessLevel::Exact
    };
    let survival = best_outcome
        .as_ref()
        .map(|outcome| outcome.survival)
        .unwrap_or(SurvivalJudgement::RiskyButPlayable);
    let dominance = match best_outcome.as_ref() {
        Some(best_outcome) => match compare_decision_outcomes(best_outcome, frontier_outcome) {
            std::cmp::Ordering::Greater => DominanceClaim::StrictlyBetterInWindow,
            std::cmp::Ordering::Less => DominanceClaim::StrictlyWorseInWindow,
            std::cmp::Ordering::Equal => DominanceClaim::Incomparable,
        },
        None => DominanceClaim::Incomparable,
    };
    let lethal_window = best_outcome
        .as_ref()
        .and_then(|outcome| match outcome.survival {
            SurvivalJudgement::ForcedLoss
            | SurvivalJudgement::SevereRisk
            | SurvivalJudgement::RiskyButPlayable => solution
                .nondominated_end_states
                .first()
                .and_then(|state| lethal_window_for_state(&state.frontier_combat)),
            SurvivalJudgement::Stable | SurvivalJudgement::Safe => None,
        });

    ExactTurnVerdict {
        best_first_input: solution
            .best_first_input
            .as_ref()
            .map(|input| format!("{input:?}")),
        best_outcome,
        survival,
        dominance: if solution.best_first_input.as_ref() == Some(chosen_move) {
            DominanceClaim::Incomparable
        } else {
            dominance
        },
        lethal_window,
        confidence,
        truncated: solution.truncated,
    }
}

pub(crate) fn exact_turn_takeover_policy(
    engine: &EngineState,
    chosen_move: &ClientInput,
    regime: CombatRegime,
    frontier_outcome: &DecisionOutcome,
    verdict: &ExactTurnVerdict,
    solution: &ExactTurnSolution,
    flags: SearchExperimentFlags,
) -> (
    Option<ClientInput>,
    TakeoverPolicy,
    DecisionAuthority,
    Vec<String>,
) {
    let takeover_eligible = solution.best_first_input.is_some()
        && verdict.confidence != ExactnessLevel::Unavailable
        && !matches!(verdict.dominance, DominanceClaim::StrictlyWorseInWindow);
    let exact_disagrees = solution
        .best_first_input
        .as_ref()
        .is_some_and(|input| input != chosen_move);
    let allow_pending_choice_takeover = matches!(engine, EngineState::PendingChoice(_));

    let mut reasons = Vec::new();
    let mut takeover = None;
    let takeover_reason = if solution.best_first_input.is_none() {
        reasons.push("exact_turn_unavailable".to_string());
        "no_best_first_input"
    } else if verdict.confidence == ExactnessLevel::Unavailable {
        reasons.push("exact_turn_unavailable".to_string());
        "confidence_unavailable"
    } else if !exact_disagrees {
        reasons.push("frontier_agrees".to_string());
        "frontier_agrees"
    } else if flags.forbid_idle_end_turn_when_exact_prefers_play
        && matches!(chosen_move, ClientInput::EndTurn)
        && solution
            .best_first_input
            .as_ref()
            .is_some_and(|input| !matches!(input, ClientInput::EndTurn))
        && matches!(verdict.dominance, DominanceClaim::StrictlyBetterInWindow)
    {
        takeover = solution.best_first_input.clone();
        reasons.push("idle_end_turn_guardrail".to_string());
        "idle_end_turn_strict_dominance"
    } else if allow_pending_choice_takeover {
        takeover = solution.best_first_input.clone();
        "override_pending_choice"
    } else {
        match regime {
            CombatRegime::Crisis
                if matches!(verdict.dominance, DominanceClaim::StrictlyBetterInWindow) =>
            {
                takeover = solution.best_first_input.clone();
                reasons.push("exact_turn_strictly_better".to_string());
                "crisis_strict_dominance"
            }
            CombatRegime::Fragile if verdict.survival > frontier_outcome.survival => {
                takeover = solution.best_first_input.clone();
                reasons.push("survival_upgrade".to_string());
                "fragile_survival_upgrade"
            }
            CombatRegime::Crisis => {
                reasons.push("crisis_without_strict_dominance".to_string());
                "crisis_not_strictly_better"
            }
            CombatRegime::Fragile => {
                reasons.push("fragile_without_survival_upgrade".to_string());
                "fragile_not_better_survival"
            }
            CombatRegime::Contested
                if flags.contested_strict_dominance_takeover
                    && matches!(verdict.dominance, DominanceClaim::StrictlyBetterInWindow) =>
            {
                takeover = solution.best_first_input.clone();
                reasons.push("contested_strict_dominance".to_string());
                "contested_strict_dominance"
            }
            CombatRegime::Advantage
                if flags.advantage_strict_dominance_takeover
                    && matches!(verdict.dominance, DominanceClaim::StrictlyBetterInWindow) =>
            {
                takeover = solution.best_first_input.clone();
                reasons.push("advantage_strict_dominance".to_string());
                "advantage_strict_dominance"
            }
            CombatRegime::Contested | CombatRegime::Advantage => {
                reasons.push("regime_not_takeover".to_string());
                "regime_not_takeover"
            }
        }
    };

    if exact_disagrees && regime != CombatRegime::Advantage {
        reasons.push("high_threat_disagreement".to_string());
    }

    let takeover_applied = takeover.is_some();
    let authority = if takeover_applied {
        DecisionAuthority::ExactTurnTakeover
    } else {
        DecisionAuthority::Frontier
    };
    let policy = TakeoverPolicy {
        regime,
        takeover_eligible,
        takeover_reason: takeover_reason.to_string(),
        takeover_applied,
        confidence: verdict.confidence,
        dominance: verdict.dominance,
        frontier_survival: frontier_outcome.survival,
        exact_survival: verdict.survival,
    };

    (takeover, policy, authority, reasons)
}

pub(crate) fn build_decision_trace(
    chosen_move: &ClientInput,
    chosen_by: DecisionAuthority,
    regime: CombatRegime,
    frontier_proposal_class: ProposalClass,
    frontier_outcome: DecisionOutcome,
    verdict: ExactTurnVerdict,
    rejection_reasons: Vec<String>,
    screened_out: Vec<ScreenRejection>,
    why_not_others: Vec<ProposalTrace>,
) -> DecisionTrace {
    let chosen_action = if matches!(chosen_by, DecisionAuthority::ExactTurnTakeover) {
        verdict
            .best_first_input
            .clone()
            .unwrap_or_else(|| format!("{chosen_move:?}"))
    } else {
        format!("{chosen_move:?}")
    };
    DecisionTrace {
        regime,
        frontier_choice: format!("{chosen_move:?}"),
        frontier_proposal_class,
        exact_turn_verdict: verdict.clone(),
        decision_outcomes: DecisionOutcomeBundle {
            frontier: frontier_outcome,
            exact_best: verdict.best_outcome.clone(),
        },
        chosen_action,
        chosen_by,
        rejection_reasons,
        screened_out,
        why_not_others,
    }
}

#[allow(dead_code)]
pub(crate) fn trajectory_terminality(outcome: TrajectoryOutcomeKind) -> TerminalForecast {
    match outcome {
        TrajectoryOutcomeKind::LethalWin => TerminalForecast::LethalWin,
        TrajectoryOutcomeKind::Survives => TerminalForecast::SurvivesWindow,
        TrajectoryOutcomeKind::Timeout => TerminalForecast::TimeoutUnknown,
        TrajectoryOutcomeKind::Dies => TerminalForecast::DiesInWindow,
    }
}

fn outcome_from_frontier_state(
    engine: &EngineState,
    combat: &CombatState,
    resources: TurnResourceSummary,
    timed_out: bool,
) -> DecisionOutcome {
    let frontier_eval = eval_frontier_state(engine, combat);
    let terminality = classify_terminality(&frontier_eval, timed_out);
    let survival = classify_survival(combat, terminality);
    let position = classify_position(combat, terminality, survival);
    DecisionOutcome {
        survival,
        position,
        terminality,
        resource_delta: ResourceDeltaSummary {
            spent_potions: resources.spent_potions,
            hp_lost: resources.hp_lost,
            exhausted_cards: resources.exhausted_cards,
            final_hp: resources.final_hp,
            final_block: resources.final_block,
        },
        efficiency_score: efficiency_score(combat, &resources, terminality),
    }
}

fn classify_terminality(frontier_eval: &FrontierEval, timed_out: bool) -> TerminalForecast {
    if timed_out {
        return TerminalForecast::TimeoutUnknown;
    }
    match frontier_eval {
        FrontierEval::Terminal(outcome) => match outcome.kind {
            crate::bot::combat::terminal::TerminalKind::Victory
            | crate::bot::combat::terminal::TerminalKind::CombatCleared => {
                TerminalForecast::LethalWin
            }
            crate::bot::combat::terminal::TerminalKind::Defeat => TerminalForecast::DiesInWindow,
            crate::bot::combat::terminal::TerminalKind::Ongoing => TerminalForecast::SurvivesWindow,
        },
        FrontierEval::NonTerminal(_) => TerminalForecast::SurvivesWindow,
    }
}

fn classify_survival(combat: &CombatState, terminality: TerminalForecast) -> SurvivalJudgement {
    match terminality {
        TerminalForecast::DiesInWindow => return SurvivalJudgement::ForcedLoss,
        TerminalForecast::LethalWin => return SurvivalJudgement::Safe,
        TerminalForecast::TimeoutUnknown => return SurvivalJudgement::RiskyButPlayable,
        TerminalForecast::SurvivesWindow => {}
    }

    let pressure = StatePressureFeatures::from_combat(combat);
    let hp = combat.entities.player.current_hp.max(1);
    if pressure.max_unblocked >= hp || pressure.lethal_pressure {
        SurvivalJudgement::SevereRisk
    } else if pressure.urgent_pressure
        || pressure.unblocked >= hp / 2
        || (hp <= 10 && pressure.unblocked > 0)
    {
        SurvivalJudgement::RiskyButPlayable
    } else if pressure.unblocked == 0 && pressure.incoming == 0 && hp >= 20 {
        SurvivalJudgement::Safe
    } else {
        SurvivalJudgement::Stable
    }
}

fn classify_position(
    combat: &CombatState,
    terminality: TerminalForecast,
    survival: SurvivalJudgement,
) -> PositionClass {
    if matches!(terminality, TerminalForecast::LethalWin) {
        return PositionClass::WinningLine;
    }

    let pressure = StatePressureFeatures::from_combat(combat);
    let posture = posture_features(combat);
    if matches!(
        survival,
        SurvivalJudgement::ForcedLoss | SurvivalJudgement::SevereRisk
    ) {
        PositionClass::Collapsing
    } else if pressure.urgent_pressure || posture.future_pollution_risk >= 8 {
        PositionClass::DefensiveBind
    } else if pressure.unblocked == 0
        && posture.future_pollution_risk <= 3
        && combat.entities.player.current_hp >= 20
    {
        PositionClass::Stabilizing
    } else if living_monster_count(combat) == 0 {
        PositionClass::WinningLine
    } else {
        PositionClass::TempoNeutral
    }
}

fn efficiency_score(
    combat: &CombatState,
    resources: &TurnResourceSummary,
    terminality: TerminalForecast,
) -> f32 {
    let pressure = StatePressureFeatures::from_combat(combat);
    let posture = posture_features(combat);
    let enemy_total = total_enemy_hp(combat) as f32;
    let terminal_bonus = match terminality {
        TerminalForecast::LethalWin => 50.0,
        TerminalForecast::SurvivesWindow => 10.0,
        TerminalForecast::TimeoutUnknown => 0.0,
        TerminalForecast::DiesInWindow => -50.0,
    };
    terminal_bonus + resources.final_hp as f32 * 0.6 + resources.final_block as f32 * 0.2
        - pressure.unblocked as f32 * 2.0
        - enemy_total * 0.12
        - posture.future_pollution_risk as f32 * 0.8
        - resources.spent_potions as f32 * 4.0
        - resources.hp_lost as f32 * 0.5
        - resources.exhausted_cards as f32 * 0.25
}

fn compare_resource_delta(
    left: &ResourceDeltaSummary,
    right: &ResourceDeltaSummary,
) -> std::cmp::Ordering {
    left.final_hp
        .cmp(&right.final_hp)
        .then_with(|| left.final_block.cmp(&right.final_block))
        .then_with(|| right.spent_potions.cmp(&left.spent_potions))
        .then_with(|| right.hp_lost.cmp(&left.hp_lost))
        .then_with(|| right.exhausted_cards.cmp(&left.exhausted_cards))
}

fn lethal_window_for_state(combat: &CombatState) -> Option<LethalWindow> {
    let pressure = StatePressureFeatures::from_combat(combat);
    if pressure.unblocked <= 0 && pressure.max_unblocked <= 0 {
        return None;
    }
    Some(LethalWindow {
        incoming: pressure.incoming,
        unblocked: pressure.unblocked.max(pressure.max_unblocked),
        player_hp: combat.entities.player.current_hp,
        player_block: combat.entities.player.block,
    })
}

fn total_enemy_hp(combat: &CombatState) -> i32 {
    combat
        .entities
        .monsters
        .iter()
        .filter(|monster| !monster.is_dying && !monster.is_escaped && monster.current_hp > 0)
        .map(|monster| monster.current_hp + monster.block)
        .sum()
}

fn living_monster_count(combat: &CombatState) -> usize {
    combat
        .entities
        .monsters
        .iter()
        .filter(|monster| {
            !monster.is_dying && !monster.half_dead && !monster.is_escaped && monster.current_hp > 0
        })
        .count()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bot::combat::exact_turn_solver::{solve_exact_turn_with_config, ExactTurnConfig};
    use crate::content::cards::CardId;
    use crate::content::monsters::EnemyId;
    use crate::runtime::combat::CombatCard;
    use crate::state::core::ClientInput;
    use crate::test_support::{blank_test_combat, planned_monster};

    fn card(id: CardId, uuid: u32) -> CombatCard {
        CombatCard::new(id, uuid)
    }

    #[test]
    fn classify_regime_marks_low_hp_high_incoming_as_crisis() {
        let mut combat = blank_test_combat();
        combat.entities.player.current_hp = 6;
        combat
            .entities
            .monsters
            .push(planned_monster(EnemyId::Cultist, 1));

        assert_eq!(classify_regime(&combat), CombatRegime::Crisis);
    }

    #[test]
    fn classify_regime_marks_boss_pollution_as_fragile() {
        let mut combat = blank_test_combat();
        combat.meta.is_boss_fight = true;
        combat
            .entities
            .monsters
            .push(planned_monster(EnemyId::SlimeBoss, 1));
        for uuid in 0..6 {
            combat.zones.draw_pile.push(card(CardId::Burn, uuid + 1));
        }

        assert_eq!(classify_regime(&combat), CombatRegime::Fragile);
    }

    #[test]
    fn compare_decision_outcomes_prefers_survival_over_efficiency() {
        let risky = DecisionOutcome {
            survival: SurvivalJudgement::SevereRisk,
            position: PositionClass::WinningLine,
            terminality: TerminalForecast::SurvivesWindow,
            resource_delta: ResourceDeltaSummary {
                spent_potions: 0,
                hp_lost: 0,
                exhausted_cards: 0,
                final_hp: 20,
                final_block: 12,
            },
            efficiency_score: 99.0,
        };
        let stable = DecisionOutcome {
            survival: SurvivalJudgement::Stable,
            position: PositionClass::TempoNeutral,
            terminality: TerminalForecast::SurvivesWindow,
            resource_delta: ResourceDeltaSummary {
                spent_potions: 0,
                hp_lost: 1,
                exhausted_cards: 0,
                final_hp: 14,
                final_block: 4,
            },
            efficiency_score: 1.0,
        };

        assert_eq!(
            compare_decision_outcomes(&stable, &risky),
            std::cmp::Ordering::Greater
        );
    }

    #[test]
    fn build_exact_turn_verdict_reports_bounded_for_truncated_search() {
        let mut combat = blank_test_combat();
        combat.turn.energy = 3;
        combat
            .entities
            .monsters
            .push(planned_monster(EnemyId::Cultist, 1));
        combat
            .zones
            .hand
            .extend([card(CardId::Defend, 1), card(CardId::Strike, 2)]);

        let solution = solve_exact_turn_with_config(
            &EngineState::CombatPlayerTurn,
            &combat,
            ExactTurnConfig {
                max_nodes: 1,
                max_engine_steps: 200,
                ..ExactTurnConfig::default()
            },
        );
        let frontier_outcome = DecisionOutcome {
            survival: SurvivalJudgement::Stable,
            position: PositionClass::TempoNeutral,
            terminality: TerminalForecast::SurvivesWindow,
            resource_delta: ResourceDeltaSummary {
                spent_potions: 0,
                hp_lost: 0,
                exhausted_cards: 0,
                final_hp: combat.entities.player.current_hp,
                final_block: combat.entities.player.block,
            },
            efficiency_score: 0.0,
        };

        let verdict = build_exact_turn_verdict(&ClientInput::EndTurn, &frontier_outcome, &solution);

        assert_eq!(verdict.confidence, ExactnessLevel::Bounded);
        assert!(verdict.truncated);
    }

    #[test]
    fn build_exact_turn_verdict_reports_forced_loss_when_best_line_dies() {
        let mut combat = blank_test_combat();
        combat.turn.energy = 0;
        combat.entities.player.current_hp = 1;
        combat
            .entities
            .monsters
            .push(planned_monster(EnemyId::Cultist, 1));

        let solution = solve_exact_turn_with_config(
            &EngineState::CombatPlayerTurn,
            &combat,
            ExactTurnConfig {
                max_nodes: 16,
                max_engine_steps: 200,
                ..ExactTurnConfig::default()
            },
        );
        let frontier_outcome = DecisionOutcome {
            survival: SurvivalJudgement::SevereRisk,
            position: PositionClass::Collapsing,
            terminality: TerminalForecast::DiesInWindow,
            resource_delta: ResourceDeltaSummary {
                spent_potions: 0,
                hp_lost: 0,
                exhausted_cards: 0,
                final_hp: 1,
                final_block: 0,
            },
            efficiency_score: -10.0,
        };

        let verdict = build_exact_turn_verdict(&ClientInput::EndTurn, &frontier_outcome, &solution);

        assert_eq!(verdict.survival, SurvivalJudgement::ForcedLoss);
    }

    #[test]
    fn classify_proposal_class_marks_block_and_end_turn() {
        let mut combat = blank_test_combat();
        combat.zones.hand.push(card(CardId::Defend, 1));

        assert_eq!(
            classify_proposal_class(
                &combat,
                &ClientInput::PlayCard {
                    card_index: 0,
                    target: None,
                }
            ),
            ProposalClass::Block
        );
        assert_eq!(
            classify_proposal_class(&combat, &ClientInput::EndTurn),
            ProposalClass::EndTurn
        );
    }
}
