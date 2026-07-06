use sts_simulator::ai::combat_search_v2::explain_combat_search_v2_initial_decision;
use sts_simulator::eval::combat_case::CombatCase;
use sts_simulator::runtime::combat::CombatCard;

use super::key_card_lifecycle::{key_card_targets, KeyCardReason};
use super::options::ReviewOptions;

#[path = "root_action_role_duel/basis.rs"]
mod basis;
#[path = "root_action_role_duel/config.rs"]
mod config;
#[path = "root_action_role_duel/execution.rs"]
mod execution;
#[path = "root_action_role_duel/selection.rs"]
mod selection;
#[path = "root_action_role_duel/transition.rs"]
mod transition;
#[path = "root_action_role_duel/types.rs"]
mod types;

pub(super) use types::RootActionRoleDuelProbe;

use basis::{prepare_duel_variant_case, PreparedRootActionRoleDuelVariant};
use config::duel_search_config;
use execution::run_duel;
use selection::select_duel_candidate_indices;
use types::RootActionRoleDuelVariant;

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
    let prepared = match prepare_duel_variant_case(original_case, key_card) {
        PreparedRootActionRoleDuelVariant::Ready(prepared) => prepared,
        PreparedRootActionRoleDuelVariant::Skipped(variant) => return variant,
    };

    let microscope = explain_combat_search_v2_initial_decision(
        &prepared.case.position.engine,
        &prepared.case.position.combat,
        duel_search_config(options, "root_action_role_duel_microscope"),
    );
    let selections = select_duel_candidate_indices(&microscope);
    let duels = selections
        .iter()
        .filter_map(|selection| run_duel(options, &prepared.case, &microscope, selection))
        .collect();

    RootActionRoleDuelVariant {
        basis: prepared.basis,
        skipped_reason: None,
        microscope: Some(microscope),
        duels,
    }
}
