use serde::Serialize;
use sts_simulator::ai::combat_search_v2::{
    CombatSearchProfile, CombatSearchV2ChildRolloutPolicy, CombatSearchV2PotionPolicy,
    CombatSearchV2RolloutPolicy, CombatSearchV2TurnPlanPolicy,
};
use sts_simulator::content::cards;
use sts_simulator::content::potions::{get_potion_definition, PotionId};
use sts_simulator::eval::combat_case::{combat_summary, CombatCase};
use sts_simulator::runtime::combat::CombatCard;
use sts_simulator::sim::combat::{
    CombatPosition, CombatStepLimits, CombatStepper, CombatTerminal, EngineCombatStepper,
};
use sts_simulator::sim::combat_action::combat_action_key;
use sts_simulator::state::core::ClientInput;

use super::focus::{review_focus, CombatReviewFocus};
use super::options::ReviewOptions;
use super::search_runner::{review_no_potion_profile, run_profile_search};
use super::search_types::{SearchDiagnosticProgressFacts, SearchReview};

#[derive(Serialize)]
pub(crate) struct ForcedPotionOpeningReview {
    pub(super) schema: &'static str,
    pub(super) contract: &'static str,
    pub(super) lanes: Vec<ForcedPotionOpeningLaneResult>,
}

#[derive(Serialize)]
pub(super) struct ForcedPotionOpeningLaneResult {
    pub(super) lane: &'static str,
    pub(super) intent: &'static str,
    pub(super) search_config_summary: Option<ForcedPotionSearchConfigSummary>,
    pub(super) prefix: ForcedPotionOpeningPrefix,
    pub(super) review: Option<SearchReview>,
    pub(super) focus: Option<CombatReviewFocus>,
    pub(super) progress: Option<SearchDiagnosticProgressFacts>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
pub(super) struct ForcedPotionSearchConfigSummary {
    pub(super) max_nodes: usize,
    pub(super) wall_ms: u64,
    pub(super) turn_plan_policy: &'static str,
    pub(super) potion_policy: &'static str,
    pub(super) max_potions_used: u32,
    pub(super) rollout_policy: &'static str,
    pub(super) child_rollout_policy: &'static str,
    pub(super) setup_bias_policy: &'static str,
    pub(super) phase_guard_policy: &'static str,
}

impl ForcedPotionSearchConfigSummary {
    fn from_profile(profile: CombatSearchProfile) -> Self {
        Self {
            max_nodes: profile.budget.max_nodes,
            wall_ms: profile.budget.wall_ms,
            turn_plan_policy: CombatSearchV2TurnPlanPolicy::from(profile.plugins.turn_plan).label(),
            potion_policy: potion_policy_label(profile.plugins.potion.policy),
            max_potions_used: profile.plugins.potion.max_potions_used.unwrap_or_default(),
            rollout_policy: CombatSearchV2RolloutPolicy::from(profile.plugins.rollout).label(),
            child_rollout_policy: CombatSearchV2ChildRolloutPolicy::from(
                profile.plugins.child_rollout,
            )
            .label(),
            setup_bias_policy: profile.plugins.action_prior.label(),
            phase_guard_policy: profile.plugins.phase_guard.label(),
        }
    }
}

fn potion_policy_label(policy: CombatSearchV2PotionPolicy) -> &'static str {
    match policy {
        CombatSearchV2PotionPolicy::Never => "never",
        CombatSearchV2PotionPolicy::All => "all",
        CombatSearchV2PotionPolicy::SemanticBudgeted => "semantic_budgeted",
    }
}

#[derive(Serialize)]
pub(super) struct ForcedPotionOpeningPrefix {
    pub(super) status: &'static str,
    pub(super) requested_potions: Vec<&'static str>,
    pub(super) actions: Vec<ForcedPotionOpeningAction>,
    pub(super) state_after: Option<ForcedPotionOpeningState>,
    pub(super) error: Option<String>,
}

#[derive(Serialize)]
pub(super) struct ForcedPotionOpeningAction {
    pub(super) potion: &'static str,
    pub(super) slot: Option<usize>,
    pub(super) action_key: Option<String>,
    pub(super) terminal_after: Option<CombatTerminal>,
}

#[derive(Serialize)]
pub(super) struct ForcedPotionOpeningState {
    pub(super) turn: u32,
    pub(super) hp: i32,
    pub(super) max_hp: i32,
    pub(super) block: i32,
    pub(super) energy: u8,
    pub(super) hand: Vec<String>,
    pub(super) potions: Vec<Option<&'static str>>,
    pub(super) living_enemy_count: usize,
    pub(super) total_enemy_hp: i32,
}

#[derive(Clone, Copy)]
struct ForcedPotionOpeningLaneSpec {
    lane: &'static str,
    intent: &'static str,
    potions: &'static [PotionId],
}

const NO_POTION: &[PotionId] = &[];
const DEX_T1: &[PotionId] = &[PotionId::DexterityPotion];
const SWIFT_T1: &[PotionId] = &[PotionId::SwiftPotion];
const DEX_SWIFT_T1: &[PotionId] = &[PotionId::DexterityPotion, PotionId::SwiftPotion];

pub(super) fn run_forced_potion_opening_lanes(
    options: &ReviewOptions,
    case: &CombatCase,
) -> Option<ForcedPotionOpeningReview> {
    if !options.forced_potion_opening_lanes {
        return None;
    }

    let lanes = forced_potion_opening_specs()
        .into_iter()
        .map(|spec| run_forced_potion_opening_lane(options, case, spec))
        .collect();

    Some(ForcedPotionOpeningReview {
        schema: "forced_potion_opening_lanes_v0",
        contract:
            "review_only_forced_opening_potion_prefix_then_no_potion_search_no_runner_policy_change",
        lanes,
    })
}

fn forced_potion_opening_specs() -> [ForcedPotionOpeningLaneSpec; 4] {
    [
        ForcedPotionOpeningLaneSpec {
            lane: "p0_no_opening_potion",
            intent: "no forced opening potion; subsequent search cannot use potions",
            potions: NO_POTION,
        },
        ForcedPotionOpeningLaneSpec {
            lane: "p1_dexterity_t1",
            intent: "force Dexterity Potion before searching; subsequent search cannot use potions",
            potions: DEX_T1,
        },
        ForcedPotionOpeningLaneSpec {
            lane: "p2_swift_t1",
            intent: "force Swift Potion before searching; subsequent search cannot use potions",
            potions: SWIFT_T1,
        },
        ForcedPotionOpeningLaneSpec {
            lane: "p3_dexterity_swift_t1",
            intent:
                "force Dexterity Potion then Swift Potion before searching; subsequent search cannot use potions",
            potions: DEX_SWIFT_T1,
        },
    ]
}

fn run_forced_potion_opening_lane(
    options: &ReviewOptions,
    case: &CombatCase,
    spec: ForcedPotionOpeningLaneSpec,
) -> ForcedPotionOpeningLaneResult {
    let prefix_result = apply_forced_potion_prefix(case, spec.potions);
    let Some(prefixed_case) = prefix_result.case else {
        return ForcedPotionOpeningLaneResult {
            lane: spec.lane,
            intent: spec.intent,
            search_config_summary: None,
            prefix: prefix_result.prefix,
            review: None,
            focus: None,
            progress: None,
        };
    };

    let nodes = options
        .quality_lane_total_nodes
        .unwrap_or(options.slow_nodes);
    let wall_ms = options.quality_lane_total_ms.unwrap_or(options.slow_ms);
    let profile = review_no_potion_profile(spec.lane, nodes, wall_ms, options);
    let search_config_summary = ForcedPotionSearchConfigSummary::from_profile(profile);
    let (review, _) = run_profile_search(&prefixed_case, profile, options.action_preview_limit);
    let focus = review_focus(std::slice::from_ref(&review));
    let progress = review.facts.diagnostic_progress.clone();
    ForcedPotionOpeningLaneResult {
        lane: spec.lane,
        intent: spec.intent,
        search_config_summary: Some(search_config_summary),
        prefix: prefix_result.prefix,
        review: Some(review),
        focus,
        progress,
    }
}

struct ForcedPotionPrefixApplication {
    prefix: ForcedPotionOpeningPrefix,
    case: Option<CombatCase>,
}

fn apply_forced_potion_prefix(
    case: &CombatCase,
    potions: &[PotionId],
) -> ForcedPotionPrefixApplication {
    let stepper = EngineCombatStepper;
    let mut position = case.position.clone();
    let mut actions = Vec::new();

    for potion in potions {
        let Some(slot) = find_potion_slot(&position, *potion) else {
            return prefix_failure(
                "unavailable",
                potions,
                actions,
                Some(prefix_state(&position)),
                format!("{} is not present", potion_label(*potion)),
            );
        };
        let input = ClientInput::UsePotion {
            potion_index: slot,
            target: None,
        };
        if !stepper.legal_actions(&position).contains(&input) {
            actions.push(ForcedPotionOpeningAction {
                potion: potion_label(*potion),
                slot: Some(slot),
                action_key: Some(combat_action_key(&position.combat, &input)),
                terminal_after: None,
            });
            return prefix_failure(
                "illegal",
                potions,
                actions,
                Some(prefix_state(&position)),
                format!("{} is not a legal current action", potion_label(*potion)),
            );
        }
        let action_key = combat_action_key(&position.combat, &input);
        let step = stepper.apply_to_stable(
            &position,
            input,
            CombatStepLimits {
                max_engine_steps: 1_000,
                deadline: None,
            },
        );
        actions.push(ForcedPotionOpeningAction {
            potion: potion_label(*potion),
            slot: Some(slot),
            action_key: Some(action_key),
            terminal_after: Some(step.terminal),
        });
        position = step.position;
        if step.truncated {
            return prefix_failure(
                "truncated",
                potions,
                actions,
                Some(prefix_state(&position)),
                "forced potion prefix did not reach a stable boundary".to_string(),
            );
        }
        if step.terminal != CombatTerminal::Unresolved {
            return prefix_failure(
                "terminal",
                potions,
                actions,
                Some(prefix_state(&position)),
                format!("forced potion prefix reached terminal {:?}", step.terminal),
            );
        }
    }

    let mut prefixed_case = case.clone();
    prefixed_case.position = position;
    prefixed_case.combat = combat_summary(&prefixed_case.position);
    prefixed_case.run.hp = prefixed_case.position.combat.entities.player.current_hp;
    prefixed_case.run.max_hp = prefixed_case.position.combat.entities.player.max_hp;

    ForcedPotionPrefixApplication {
        prefix: ForcedPotionOpeningPrefix {
            status: "applied",
            requested_potions: potion_labels(potions),
            actions,
            state_after: Some(prefix_state(&prefixed_case.position)),
            error: None,
        },
        case: Some(prefixed_case),
    }
}

fn prefix_failure(
    status: &'static str,
    requested: &[PotionId],
    actions: Vec<ForcedPotionOpeningAction>,
    state_after: Option<ForcedPotionOpeningState>,
    error: String,
) -> ForcedPotionPrefixApplication {
    ForcedPotionPrefixApplication {
        prefix: ForcedPotionOpeningPrefix {
            status,
            requested_potions: potion_labels(requested),
            actions,
            state_after,
            error: Some(error),
        },
        case: None,
    }
}

fn find_potion_slot(position: &CombatPosition, id: PotionId) -> Option<usize> {
    position
        .combat
        .entities
        .potions
        .iter()
        .position(|potion| potion.as_ref().is_some_and(|potion| potion.id == id))
}

fn prefix_state(position: &CombatPosition) -> ForcedPotionOpeningState {
    ForcedPotionOpeningState {
        turn: position.combat.turn.turn_count,
        hp: position.combat.entities.player.current_hp,
        max_hp: position.combat.entities.player.max_hp,
        block: position.combat.entities.player.block,
        energy: position.combat.turn.energy,
        hand: position.combat.zones.hand.iter().map(card_label).collect(),
        potions: position
            .combat
            .entities
            .potions
            .iter()
            .map(|potion| potion.as_ref().map(|potion| potion_label(potion.id)))
            .collect(),
        living_enemy_count: position
            .combat
            .entities
            .monsters
            .iter()
            .filter(|monster| monster.is_alive_for_action())
            .count(),
        total_enemy_hp: position
            .combat
            .entities
            .monsters
            .iter()
            .filter(|monster| monster.is_alive_for_action())
            .map(|monster| monster.current_hp.max(0) + monster.block.max(0))
            .sum(),
    }
}

fn potion_labels(potions: &[PotionId]) -> Vec<&'static str> {
    potions.iter().copied().map(potion_label).collect()
}

fn potion_label(potion: PotionId) -> &'static str {
    get_potion_definition(potion).name
}

fn card_label(card: &CombatCard) -> String {
    format!("{}+{}", cards::java_id(card.id), card.upgrades)
}
