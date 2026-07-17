use sts_simulator::ai::combat_search_v2::{
    high_stakes_semantic_potion_budget, CombatSearchAcceptancePluginId,
    CombatSearchArtifactPluginId, CombatSearchAttemptPolicy, CombatSearchBudgetSpec,
    CombatSearchChildRolloutPluginId, CombatSearchEngineProfile, CombatSearchPhaseGuardPluginId,
    CombatSearchPluginStack, CombatSearchPotionPlugin, CombatSearchProfile,
    CombatSearchRolloutPluginId, CombatSearchV2PotionPolicy, CombatSearchV2Satisfaction,
};
use sts_simulator::eval::run_control::{
    RunControlCombatSearchQuantum, RunControlHpLossLimit, RunControlSearchCombatOptions,
    RunControlSession,
};

use super::combat_search_survival::owner_audit_hp_loss_limit;
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
    pub(super) stakes: CombatSearchStakes,
    pub(super) total_nodes: usize,
    pub(super) total_wall_ms: u64,
    pub(super) potion_policy: CombatSearchV2PotionPolicy,
    pub(super) max_potions_used: Option<u32>,
    pub(super) semantics_fingerprint: String,
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
    let total_nodes = quanta.iter().fold(0usize, |total, quantum| {
        total.saturating_add(quantum.additional_nodes)
    });
    let total_wall_ms = quanta.iter().fold(0u64, |total, quantum| {
        total.saturating_add(quantum.soft_wall_ms.unwrap_or_default())
    });
    let (potion_policy, max_potions_used) = canonical_potion_surface(session, stakes);
    let profile = CombatSearchProfile {
        label: "canonical_combat_session",
        engine: CombatSearchEngineProfile {
            budget: CombatSearchBudgetSpec {
                max_nodes: total_nodes,
                wall_ms: total_wall_ms,
            },
            plugins: CombatSearchPluginStack {
                child_rollout: CombatSearchChildRolloutPluginId::LazyOnPop,
                rollout: CombatSearchRolloutPluginId::EnemyMechanicsAdaptiveNoPotion,
                phase_guard: CombatSearchPhaseGuardPluginId::Default,
                potion: CombatSearchPotionPlugin {
                    policy: potion_policy,
                    max_potions_used,
                },
                ..CombatSearchPluginStack::default()
            },
        },
        policy: CombatSearchAttemptPolicy {
            acceptance: CombatSearchAcceptancePluginId::CleanAcceptedLineNoNewCurse,
            artifacts: CombatSearchArtifactPluginId::FullTrace,
        },
    };
    let satisfaction = match owner_audit_hp_loss_limit(session) {
        RunControlHpLossLimit::Limit(limit) => {
            CombatSearchV2Satisfaction::HpLossAtMostWithoutNewExternalBurden(limit)
        }
        RunControlHpLossLimit::Unlimited => {
            CombatSearchV2Satisfaction::FirstCompleteWinWithoutNewExternalBurden
        }
    };
    let semantics_fingerprint = profile.semantics_fingerprint();
    let mut search = RunControlSearchCombatOptions::default();
    search.profile = Some(profile);
    search.satisfaction = Some(satisfaction);
    // Satisfaction controls whether more computation is useful. Final line
    // eligibility is deliberately wider: if the work plan cannot reach the
    // reserve target, its best clean survival win remains actionable evidence.
    search.max_hp_loss = Some(RunControlHpLossLimit::Unlimited);
    search.potion_policy = Some(potion_policy);
    search.max_potions_used = max_potions_used;
    search.work_quanta = quanta;
    // A canonical session must be the only search owner. The old complete-line,
    // turn-plan, and turn-pool root searches are therefore not invoked after a
    // gap. Smoke Bomb remains a direct legal survival action, not another search.
    search.disable_no_win_rescue = true;
    search.allow_smoke_bomb_survival_fallback = stakes != CombatSearchStakes::Boss;

    CombatSearchSessionPlan {
        search,
        stakes,
        total_nodes,
        total_wall_ms,
        potion_policy,
        max_potions_used,
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
        CombatSearchStakes::Hallway => (CombatSearchV2PotionPolicy::SemanticBudgeted, Some(2)),
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
    fn hallway_plan_uses_one_superset_session_with_two_quanta() {
        let plan = canonical_combat_search_session_plan(&hallway_session(), args());

        assert_eq!(plan.search.work_quanta.len(), 2);
        assert_eq!(plan.search.work_quanta[0].label, "initial");
        assert_eq!(plan.search.work_quanta[1].label, "refine");
        assert_eq!(
            plan.potion_policy,
            CombatSearchV2PotionPolicy::SemanticBudgeted
        );
        assert_eq!(plan.max_potions_used, Some(2));
        assert_eq!(
            plan.search.max_hp_loss,
            Some(RunControlHpLossLimit::Unlimited)
        );
        assert!(plan.search.disable_no_win_rescue);
        assert!(matches!(
            plan.search.satisfaction,
            Some(CombatSearchV2Satisfaction::HpLossAtMostWithoutNewExternalBurden(_))
        ));
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
    }
}
