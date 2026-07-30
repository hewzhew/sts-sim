use sts_simulator::ai::combat_search_v2::{
    high_stakes_semantic_potion_budget, CombatSearchAcceptancePluginId,
    CombatSearchArtifactPluginId, CombatSearchAttemptPolicy, CombatSearchBudgetSpec,
    CombatSearchChildRolloutPluginId, CombatSearchEngineProfile, CombatSearchPhaseGuardPluginId,
    CombatSearchPluginStack, CombatSearchPotionPlugin, CombatSearchProfile,
    CombatSearchRolloutPluginId, CombatSearchTurnPlanPluginId, CombatSearchV2PotionPolicy,
    CombatSearchV2Satisfaction,
};
use sts_simulator::eval::run_control::{
    oracle_active_victory_potion_slot_mask_v1, RunControlCombatSearchQuantum,
    RunControlHpLossLimit, RunControlSearchCombatOptions, RunControlSession,
};

use super::combat_search_survival::owner_audit_search_quality_loss_target;
use super::Args;

const HALLWAY_REFINEMENT_MAX_NODES: usize = 300_000;
const HALLWAY_REFINEMENT_MAX_MS: u64 = 5_000;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum CombatSearchStakes {
    Hallway,
    Elite,
    Boss,
}

pub(super) struct CombatSearchSessionPlan {
    pub(super) search: RunControlSearchCombatOptions,
    pub(super) profile_id: &'static str,
    pub(super) stage: CombatSearchSessionStage,
    pub(super) stakes: CombatSearchStakes,
    pub(super) total_nodes: usize,
    pub(super) total_wall_ms: u64,
    pub(super) potion_policy: CombatSearchV2PotionPolicy,
    pub(super) max_potions_used: Option<u32>,
    pub(super) allowed_potion_slots: Option<u64>,
    pub(super) semantics_fingerprint: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum PotionRescueKind {
    ImproveVerifiedWinQualityGated,
    FindAnyWin,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum CombatSearchSessionStage {
    Canonical,
    NoPotionPrimary,
    NoPotionRefinement,
    ImproveVerifiedWin,
    FindAnyWin,
}

impl CombatSearchSessionStage {
    pub(super) fn label(self) -> &'static str {
        match self {
            Self::Canonical => "canonical",
            Self::NoPotionPrimary => "no_potion_primary",
            Self::NoPotionRefinement => "no_potion_refinement",
            Self::ImproveVerifiedWin => "improve_verified_win",
            Self::FindAnyWin => "find_any_win",
        }
    }

    fn profile_id(self) -> &'static str {
        match self {
            Self::Canonical => "canonical_combat_session",
            Self::NoPotionPrimary => "canonical_combat_no_potion_primary",
            Self::NoPotionRefinement => "canonical_combat_no_potion_refinement",
            Self::ImproveVerifiedWin => "canonical_combat_bounded_potion_rescue",
            Self::FindAnyWin => "canonical_combat_survival_potion_rescue",
        }
    }
}

impl CombatSearchSessionPlan {
    pub(super) fn should_checkpoint_before_search(&self, args: Args) -> bool {
        self.stakes == CombatSearchStakes::Boss && args.checkpoint_before_combat_portfolio
    }
}

pub(super) fn canonical_combat_search_session_plan(
    session: &RunControlSession,
    args: Args,
) -> CombatSearchSessionPlan {
    let stakes = combat_search_stakes(session);
    let quanta = work_quanta(stakes, args);
    let (potion_policy, max_potions_used) = canonical_potion_surface(session, stakes);
    build_combat_search_session_plan(
        session,
        stakes,
        quanta,
        CombatSearchSessionStage::Canonical,
        potion_policy,
        max_potions_used,
        None,
        RunControlHpLossLimit::Unlimited,
        stakes != CombatSearchStakes::Boss,
    )
}

pub(super) fn potion_conserving_primary_search_session_plan(
    session: &RunControlSession,
    args: Args,
) -> Option<CombatSearchSessionPlan> {
    let stakes = combat_search_stakes(session);
    if stakes == CombatSearchStakes::Boss || oracle_active_victory_potion_slot_mask_v1(session) == 0
    {
        return None;
    }
    let mut quanta = work_quanta(stakes, args);
    if quanta.len() < 2 {
        return None;
    }
    quanta.truncate(1);
    Some(build_combat_search_session_plan(
        session,
        stakes,
        quanta,
        CombatSearchSessionStage::NoPotionPrimary,
        CombatSearchV2PotionPolicy::Never,
        Some(0),
        Some(0),
        owner_audit_search_quality_loss_target(session),
        false,
    ))
}

pub(super) fn potion_conserving_refinement_search_session_plan(
    session: &RunControlSession,
    args: Args,
    rescue_kind: PotionRescueKind,
) -> Option<CombatSearchSessionPlan> {
    let stakes = combat_search_stakes(session);
    if stakes == CombatSearchStakes::Boss {
        return None;
    }
    let quanta = work_quanta(stakes, args)
        .into_iter()
        .skip(1)
        .collect::<Vec<_>>();
    if quanta.is_empty() {
        return None;
    }
    let allowed_potion_slots = oracle_active_victory_potion_slot_mask_v1(session);
    let (stage, potion_policy, max_potions_used) = if allowed_potion_slots == 0 {
        (
            CombatSearchSessionStage::NoPotionRefinement,
            CombatSearchV2PotionPolicy::Never,
            Some(0),
        )
    } else {
        (
            match rescue_kind {
                PotionRescueKind::ImproveVerifiedWinQualityGated => {
                    CombatSearchSessionStage::ImproveVerifiedWin
                }
                PotionRescueKind::FindAnyWin => CombatSearchSessionStage::FindAnyWin,
            },
            // The outer staged contract has already selected exact potion
            // identities. Reapplying the legacy tactical gate here would
            // incorrectly hide common stat/energy rescues in ordinary rooms.
            CombatSearchV2PotionPolicy::All,
            Some(1),
        )
    };
    Some(build_combat_search_session_plan(
        session,
        stakes,
        quanta,
        stage,
        potion_policy,
        max_potions_used,
        Some(allowed_potion_slots),
        RunControlHpLossLimit::Unlimited,
        rescue_kind == PotionRescueKind::FindAnyWin,
    ))
}

#[allow(clippy::too_many_arguments)]
fn build_combat_search_session_plan(
    session: &RunControlSession,
    stakes: CombatSearchStakes,
    quanta: Vec<RunControlCombatSearchQuantum>,
    stage: CombatSearchSessionStage,
    potion_policy: CombatSearchV2PotionPolicy,
    max_potions_used: Option<u32>,
    allowed_potion_slots: Option<u64>,
    max_hp_loss: RunControlHpLossLimit,
    allow_smoke_bomb_survival_fallback: bool,
) -> CombatSearchSessionPlan {
    let total_nodes = quanta.iter().fold(0usize, |total, quantum| {
        total.saturating_add(quantum.additional_nodes)
    });
    let total_wall_ms = quanta.iter().fold(0u64, |total, quantum| {
        total.saturating_add(quantum.soft_wall_ms.unwrap_or_default())
    });
    let profile_id = stage.profile_id();
    let profile = CombatSearchProfile {
        label: profile_id,
        engine: CombatSearchEngineProfile {
            budget: CombatSearchBudgetSpec {
                max_nodes: total_nodes,
                wall_ms: total_wall_ms,
            },
            plugins: CombatSearchPluginStack {
                child_rollout: CombatSearchChildRolloutPluginId::LazyOnPop,
                rollout: CombatSearchRolloutPluginId::EnemyMechanicsAdaptiveNoPotion,
                turn_plan: CombatSearchTurnPlanPluginId::TacticalEnemyTurnBoundaryFrontierSeed,
                phase_guard: CombatSearchPhaseGuardPluginId::Default,
                potion: CombatSearchPotionPlugin {
                    policy: potion_policy,
                    max_potions_used,
                    allowed_potion_slots,
                },
                ..CombatSearchPluginStack::default()
            },
        },
        policy: CombatSearchAttemptPolicy {
            acceptance: CombatSearchAcceptancePluginId::CleanAcceptedLineNoNewCurse,
            artifacts: CombatSearchArtifactPluginId::FullTrace,
        },
    };
    let satisfaction = match owner_audit_search_quality_loss_target(session) {
        RunControlHpLossLimit::Limit(limit) => {
            CombatSearchV2Satisfaction::HpLossAtMostWithoutNewExternalBurden(limit)
        }
        RunControlHpLossLimit::Unlimited => {
            CombatSearchV2Satisfaction::FirstCompleteWinWithoutNewExternalBurden
        }
    };
    let semantics_fingerprint = profile.semantics_fingerprint();
    // A canonical session must be the only search owner. The old complete-line,
    // turn-plan, and turn-pool root searches are therefore not invoked after a
    // gap. Smoke Bomb remains a direct legal survival action, not another search.
    let search = RunControlSearchCombatOptions {
        profile: Some(profile),
        satisfaction: Some(satisfaction),
        max_hp_loss: Some(max_hp_loss),
        potion_policy: Some(potion_policy),
        max_potions_used,
        allowed_potion_slots,
        work_quanta: quanta,
        enable_legacy_no_win_rescue: false,
        allow_smoke_bomb_survival_fallback,
        ..RunControlSearchCombatOptions::default()
    };

    CombatSearchSessionPlan {
        search,
        profile_id,
        stage,
        stakes,
        total_nodes,
        total_wall_ms,
        potion_policy,
        max_potions_used,
        allowed_potion_slots,
        semantics_fingerprint,
    }
}

fn combat_search_stakes(session: &RunControlSession) -> CombatSearchStakes {
    session
        .active_combat
        .as_ref()
        .map_or(CombatSearchStakes::Hallway, |active| {
            if active.combat_state.meta.is_boss_fight {
                CombatSearchStakes::Boss
            } else if active.combat_state.meta.is_elite_fight {
                CombatSearchStakes::Elite
            } else {
                CombatSearchStakes::Hallway
            }
        })
}

fn work_quanta(stakes: CombatSearchStakes, args: Args) -> Vec<RunControlCombatSearchQuantum> {
    let refinement = match stakes {
        CombatSearchStakes::Hallway => RunControlCombatSearchQuantum {
            label: "refine",
            additional_nodes: args
                .boss_search_nodes
                .min(HALLWAY_REFINEMENT_MAX_NODES)
                .max(args.rescue_search_nodes),
            soft_wall_ms: Some(
                args.boss_search_ms
                    .min(HALLWAY_REFINEMENT_MAX_MS)
                    .max(args.rescue_search_ms),
            ),
        },
        CombatSearchStakes::Elite => RunControlCombatSearchQuantum {
            label: "refine",
            additional_nodes: args.rescue_search_nodes,
            soft_wall_ms: Some(args.rescue_search_ms),
        },
        CombatSearchStakes::Boss => RunControlCombatSearchQuantum {
            label: "refine",
            additional_nodes: args.boss_search_nodes,
            soft_wall_ms: Some(args.boss_search_ms),
        },
    };
    let mut quanta = vec![RunControlCombatSearchQuantum {
        label: "initial",
        additional_nodes: args.search_nodes,
        soft_wall_ms: Some(args.search_ms),
    }];
    if refinement.additional_nodes > 0 && refinement.soft_wall_ms != Some(0) {
        quanta.push(refinement);
    }
    quanta
}

fn canonical_potion_surface(
    session: &RunControlSession,
    stakes: CombatSearchStakes,
) -> (CombatSearchV2PotionPolicy, Option<u32>) {
    let Some(combat) = session
        .active_combat
        .as_ref()
        .map(|active| &active.combat_state)
    else {
        return (CombatSearchV2PotionPolicy::Never, Some(0));
    };
    let usable = combat
        .entities
        .potions
        .iter()
        .flatten()
        .any(|potion| potion.can_use);
    if !usable {
        return (CombatSearchV2PotionPolicy::Never, Some(0));
    }
    match stakes {
        CombatSearchStakes::Hallway => (CombatSearchV2PotionPolicy::SemanticBudgeted, Some(1)),
        CombatSearchStakes::Elite => (CombatSearchV2PotionPolicy::SemanticBudgeted, Some(1)),
        CombatSearchStakes::Boss => (
            CombatSearchV2PotionPolicy::All,
            Some(
                high_stakes_semantic_potion_budget(combat)
                    .unwrap_or_default()
                    .max(3),
            ),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::branch::owner_audit::run_contract::RunObjective;
    use sts_simulator::content::potions::{Potion, PotionId};
    use sts_simulator::eval::run_control::RunControlConfig;
    use sts_simulator::state::core::{ActiveCombat, CombatContext, EngineState, RoomCombatContext};
    use sts_simulator::state::map::node::RoomType;

    fn args() -> Args {
        Args {
            seed: 1,
            ascension: 0,
            objective: RunObjective::FirstVictory,
            generations: 1,
            max_branches: 1,
            auto_ops: 1,
            search_nodes: 10,
            search_ms: 100,
            rescue_search_nodes: 20,
            rescue_search_ms: 200,
            boss_search_nodes: 30,
            boss_search_ms: 300,
            wall_ms: None,
            checkpoint_before_combat_portfolio: false,
            wall_capped_search_budget: false,
            wall_capped_boss_budget: false,
        }
    }

    fn hallway_session() -> RunControlSession {
        let mut session = RunControlSession::new(RunControlConfig::default());
        let mut combat = crate::test_support::blank_test_combat();
        combat.entities.potions = vec![Some(Potion::new(PotionId::BlockPotion, 1))];
        session.active_combat = Some(ActiveCombat::new(
            EngineState::CombatPlayerTurn,
            combat,
            CombatContext::Room(RoomCombatContext {
                room_type: RoomType::MonsterRoom,
            }),
        ));
        session
    }

    #[test]
    fn canonical_hallway_fallback_uses_one_potion_cap_across_two_quanta() {
        let plan = canonical_combat_search_session_plan(&hallway_session(), args());

        assert_eq!(plan.search.work_quanta.len(), 2);
        assert_eq!(plan.search.work_quanta[0].label, "initial");
        assert_eq!(plan.search.work_quanta[1].label, "refine");
        assert_eq!(
            plan.potion_policy,
            CombatSearchV2PotionPolicy::SemanticBudgeted
        );
        assert_eq!(plan.max_potions_used, Some(1));
        assert_eq!(
            plan.search.max_hp_loss,
            Some(RunControlHpLossLimit::Unlimited)
        );
        assert!(!plan.search.enable_legacy_no_win_rescue);
        assert!(matches!(
            plan.search.satisfaction,
            Some(CombatSearchV2Satisfaction::HpLossAtMostWithoutNewExternalBurden(_))
        ));
    }

    #[test]
    fn hallway_plan_stages_no_potion_quality_before_exact_single_slot_rescue() {
        let session = hallway_session();
        let primary =
            potion_conserving_primary_search_session_plan(&session, args()).expect("primary");
        let refinement = potion_conserving_refinement_search_session_plan(
            &session,
            args(),
            PotionRescueKind::ImproveVerifiedWinQualityGated,
        )
        .expect("refinement");

        assert_eq!(primary.stage, CombatSearchSessionStage::NoPotionPrimary);
        assert_eq!(primary.search.work_quanta.len(), 1);
        assert_eq!(primary.search.work_quanta[0].label, "initial");
        assert_eq!(primary.potion_policy, CombatSearchV2PotionPolicy::Never);
        assert_eq!(primary.max_potions_used, Some(0));
        assert_eq!(primary.search.allowed_potion_slots, Some(0));
        assert!(matches!(
            primary.search.max_hp_loss,
            Some(RunControlHpLossLimit::Limit(_))
        ));
        assert!(!primary.search.allow_smoke_bomb_survival_fallback);

        assert_eq!(
            refinement.stage,
            CombatSearchSessionStage::ImproveVerifiedWin
        );
        assert_eq!(refinement.search.work_quanta.len(), 1);
        assert_eq!(refinement.search.work_quanta[0].label, "refine");
        assert_eq!(refinement.potion_policy, CombatSearchV2PotionPolicy::All);
        assert_eq!(refinement.max_potions_used, Some(1));
        assert_eq!(refinement.search.allowed_potion_slots, Some(1));
        assert_eq!(
            refinement.search.max_hp_loss,
            Some(RunControlHpLossLimit::Unlimited)
        );
        assert!(!refinement.search.allow_smoke_bomb_survival_fallback);
        assert_eq!(
            primary.total_nodes.saturating_add(refinement.total_nodes),
            canonical_combat_search_session_plan(&session, args()).total_nodes
        );
    }

    #[test]
    fn quality_gated_refinement_opens_active_flexible_potions() {
        let mut session = hallway_session();
        session
            .active_combat
            .as_mut()
            .unwrap()
            .combat_state
            .entities
            .potions = vec![Some(Potion::new(PotionId::PowerPotion, 2))];

        let improve = potion_conserving_refinement_search_session_plan(
            &session,
            args(),
            PotionRescueKind::ImproveVerifiedWinQualityGated,
        )
        .expect("quality-gated refinement");
        let survival = potion_conserving_refinement_search_session_plan(
            &session,
            args(),
            PotionRescueKind::FindAnyWin,
        )
        .expect("survival refinement");

        assert_eq!(improve.stage, CombatSearchSessionStage::ImproveVerifiedWin);
        assert_eq!(improve.search.allowed_potion_slots, Some(1));
        assert_eq!(improve.max_potions_used, Some(1));
        assert!(!improve.search.allow_smoke_bomb_survival_fallback);
        assert_eq!(survival.stage, CombatSearchSessionStage::FindAnyWin);
        assert_eq!(survival.search.allowed_potion_slots, Some(1));
        assert_eq!(survival.max_potions_used, Some(1));
        assert!(survival.search.allow_smoke_bomb_survival_fallback);
    }

    #[test]
    fn zero_wall_refinement_is_not_authorized() {
        let mut args = args();
        args.rescue_search_ms = 0;
        args.boss_search_ms = 0;

        let plan = canonical_combat_search_session_plan(&hallway_session(), args);

        assert_eq!(plan.search.work_quanta.len(), 1);
        assert_eq!(plan.search.work_quanta[0].label, "initial");
        assert_eq!(plan.total_wall_ms, args.search_ms);
        assert_eq!(plan.total_nodes, args.search_nodes);
        assert!(potion_conserving_primary_search_session_plan(&hallway_session(), args).is_none());
    }
}
