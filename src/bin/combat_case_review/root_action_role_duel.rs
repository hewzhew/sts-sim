use std::time::Duration;

use sts_simulator::ai::combat_search_v2::{
    explain_combat_search_v2_initial_decision, CombatSearchV2Config,
    CombatSearchV2DecisionCandidateReport, CombatSearchV2DecisionMicroscopeReport,
    CombatSearchV2PotionPolicy, CombatSearchV2Report, CombatSearchV2RolloutPolicy,
    CombatSearchV2SetupBiasPolicy, CombatSearchV2TurnPlanPolicy,
};
use sts_simulator::content::cards::java_id;
use sts_simulator::content::monsters::EnemyId;
use sts_simulator::eval::combat_case::CombatCase;
use sts_simulator::runtime::combat::CombatCard;
use sts_simulator::sim::combat::{
    CombatPosition, CombatStepLimits, CombatStepper, CombatTerminal, EngineCombatStepper,
};
use sts_simulator::state::core::ClientInput;

use super::focus::review_focus;
use super::key_card_counterfactual::{move_key_card, KeyCardCounterfactualPlacement};
use super::key_card_lifecycle::{key_card_lifecycle, key_card_targets, KeyCardReason};
use super::options::ReviewOptions;
use super::search_runner::run_configured_search;
use super::search_types::SearchReview;

#[path = "root_action_role_duel/selection.rs"]
mod selection;
#[path = "root_action_role_duel/types.rs"]
mod types;

pub(super) use types::RootActionRoleDuelProbe;

use selection::select_duel_candidate_indices;
use types::{
    DuelSelection, RootActionRoleDuel, RootActionRoleDuelBasis, RootActionRoleDuelCandidate,
    RootActionRoleDuelKeyCard, RootActionRoleDuelTransition, RootActionRoleDuelVariant,
};

pub(super) fn run_root_action_role_duel_probe(
    options: &ReviewOptions,
    case: &CombatCase,
) -> Option<RootActionRoleDuelProbe> {
    if !options.root_action_role_duel {
        return None;
    }

    let mut variants = Vec::new();
    let targets = key_card_targets(&case.position.combat);
    if targets.is_empty() {
        variants.push(run_variant(options, case, None));
    } else {
        for target in targets {
            variants.push(run_variant(
                options,
                case,
                Some((&target.card, target.reason)),
            ));
        }
    }

    Some(RootActionRoleDuelProbe {
        schema: "root_action_role_duel_probe_v0",
        contract:
            "review_only_force_existing_root_action_then_child_search_no_runner_policy_change",
        skipped_reason: None,
        variants,
    })
}

fn run_variant(
    options: &ReviewOptions,
    original_case: &CombatCase,
    key_card: Option<(&CombatCard, KeyCardReason)>,
) -> RootActionRoleDuelVariant {
    let mut case = original_case.clone();
    let basis = match key_card {
        Some((card, reason)) => {
            let placement = KeyCardCounterfactualPlacement::OpeningHand;
            if move_key_card(&mut case.position.combat, card.uuid, placement).is_none() {
                return skipped_variant(
                    card,
                    reason,
                    placement,
                    "key_card_not_in_active_combat_zones",
                );
            }
            RootActionRoleDuelBasis {
                label: format!("key_card_opening_hand:{}#{}", java_id(card.id), card.uuid),
                moved_key_card: Some(RootActionRoleDuelKeyCard {
                    card: format!("{}+{}", java_id(card.id), card.upgrades),
                    uuid: card.uuid,
                    reason: reason.label(),
                    placement: placement.label(),
                }),
            }
        }
        None => RootActionRoleDuelBasis {
            label: "original_root".to_string(),
            moved_key_card: None,
        },
    };

    let microscope = explain_combat_search_v2_initial_decision(
        &case.position.engine,
        &case.position.combat,
        review_search_config(options, "root_action_role_duel_microscope"),
    );
    let selections = select_duel_candidate_indices(&microscope);
    let duels = selections
        .iter()
        .filter_map(|selection| run_duel(options, &case, &microscope, selection))
        .collect();

    RootActionRoleDuelVariant {
        basis,
        skipped_reason: None,
        microscope: Some(microscope),
        duels,
    }
}

fn skipped_variant(
    card: &CombatCard,
    reason: KeyCardReason,
    placement: KeyCardCounterfactualPlacement,
    skipped_reason: &'static str,
) -> RootActionRoleDuelVariant {
    RootActionRoleDuelVariant {
        basis: RootActionRoleDuelBasis {
            label: format!("key_card_opening_hand:{}#{}", java_id(card.id), card.uuid),
            moved_key_card: Some(RootActionRoleDuelKeyCard {
                card: format!("{}+{}", java_id(card.id), card.upgrades),
                uuid: card.uuid,
                reason: reason.label(),
                placement: placement.label(),
            }),
        },
        skipped_reason: Some(skipped_reason),
        microscope: None,
        duels: Vec::new(),
    }
}

fn run_duel(
    options: &ReviewOptions,
    case: &CombatCase,
    microscope: &CombatSearchV2DecisionMicroscopeReport,
    selection: &DuelSelection,
) -> Option<RootActionRoleDuel> {
    let candidate = microscope.candidates.get(selection.candidate_index)?;
    let stepper = EngineCombatStepper;
    let step = stepper.apply_to_stable(
        &case.position,
        candidate.input.clone(),
        CombatStepLimits {
            max_engine_steps: 250,
            deadline: None,
        },
    );
    let root_transition = root_transition(&step.position, &step, candidate);
    let child_case = child_case(case, &step.position);
    let (child_search, child_report) = if step.alive
        && !step.truncated
        && !step.timed_out
        && matches!(step.terminal, CombatTerminal::Unresolved)
    {
        let (search, report) =
            run_child_search(options, &child_case, root_potions_used(&candidate.input));
        (Some(search), Some(report))
    } else {
        (None, None)
    };
    let child_best_complete_final_state = child_report
        .as_ref()
        .and_then(|report| report.best_complete_trajectory.as_ref())
        .map(|trajectory| trajectory.final_state.clone());
    let child_focus = child_search
        .as_ref()
        .map(|search| review_focus(std::slice::from_ref(search)));
    let key_card_lifecycle_after_root = child_focus
        .as_ref()
        .and_then(|focus| key_card_lifecycle(&child_case.position, focus.as_ref()));

    Some(RootActionRoleDuel {
        selection_reasons: selection.reasons.clone(),
        root_candidate: RootActionRoleDuelCandidate {
            ordered_index: candidate.ordered_index,
            action_key: candidate.action_key.clone(),
            action_role: candidate.action_role,
            selected_by_best_complete: candidate.selected_by_best_complete,
            input: candidate.input.clone(),
        },
        root_transition,
        child_search,
        child_best_complete_final_state,
        child_focus: child_focus.flatten(),
        key_card_lifecycle_after_root,
    })
}

fn run_child_search(
    options: &ReviewOptions,
    case: &CombatCase,
    root_potions_used: u32,
) -> (SearchReview, CombatSearchV2Report) {
    run_configured_search(
        "root_action_role_duel_child",
        case,
        review_search_config(options, "root_action_role_duel_child").with_max_potions_used(
            options
                .diagnostic_potion_max
                .saturating_sub(root_potions_used),
        ),
        options.action_preview_limit,
    )
}

trait RootDuelSearchConfigExt {
    fn with_max_potions_used(self, max_potions_used: u32) -> Self;
}

impl RootDuelSearchConfigExt for CombatSearchV2Config {
    fn with_max_potions_used(mut self, max_potions_used: u32) -> Self {
        self.max_potions_used = Some(max_potions_used);
        self
    }
}

fn review_search_config(options: &ReviewOptions, label: &'static str) -> CombatSearchV2Config {
    let rollout_policy = if options.disable_rollout {
        CombatSearchV2RolloutPolicy::Disabled
    } else {
        CombatSearchV2RolloutPolicy::EnemyMechanicsAdaptiveNoPotion
    };
    CombatSearchV2Config {
        max_nodes: options.slow_nodes,
        wall_time: Some(Duration::from_millis(options.slow_ms)),
        turn_plan_policy: CombatSearchV2TurnPlanPolicy::DiagnosticOnly,
        potion_policy: CombatSearchV2PotionPolicy::All,
        max_potions_used: Some(options.diagnostic_potion_max),
        rollout_policy,
        child_rollout_policy: options.child_rollout_policy(),
        setup_bias_policy: CombatSearchV2SetupBiasPolicy::KeyCardOnline,
        input_label: Some(label.to_string()),
        ..CombatSearchV2Config::default()
    }
}

fn child_case(case: &CombatCase, position: &CombatPosition) -> CombatCase {
    let mut child = case.clone();
    child.position = position.clone();
    child
}

fn root_transition(
    position: &CombatPosition,
    step: &sts_simulator::sim::combat::CombatStepResult,
    candidate: &CombatSearchV2DecisionCandidateReport,
) -> RootActionRoleDuelTransition {
    RootActionRoleDuelTransition {
        status: step_status(step),
        terminal: step.terminal,
        engine_steps: step.engine_steps,
        player_hp: position.combat.entities.player.current_hp,
        player_block: position.combat.entities.player.block,
        energy: position.combat.turn.energy,
        living_enemy_count: position
            .combat
            .entities
            .monsters
            .iter()
            .filter(|monster| !monster.is_dead_or_escaped())
            .count(),
        cultists_alive: position
            .combat
            .entities
            .monsters
            .iter()
            .filter(|monster| {
                !monster.is_dead_or_escaped()
                    && EnemyId::from_id(monster.monster_type) == Some(EnemyId::Cultist)
            })
            .count(),
        total_enemy_hp: candidate.one_step.total_enemy_hp,
        visible_incoming_damage: candidate.one_step.visible_incoming_damage,
        survival_margin: candidate.one_step.survival_margin,
    }
}

fn step_status(step: &sts_simulator::sim::combat::CombatStepResult) -> &'static str {
    if step.timed_out {
        "timed_out"
    } else if step.truncated {
        "engine_step_limit"
    } else if !step.alive {
        "player_dead"
    } else {
        "stable"
    }
}

fn root_potions_used(input: &ClientInput) -> u32 {
    if matches!(input, ClientInput::UsePotion { .. }) {
        1
    } else {
        0
    }
}
