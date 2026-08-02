use std::sync::Arc;

use serde::{Deserialize, Serialize};

use crate::ai::combat_search_v2::{
    CombatSearchV2ChildRolloutPolicy, CombatSearchV2PhaseGuardPolicy, CombatSearchV2PotionPolicy,
    CombatSearchV2RolloutPolicy, CombatSearchV2Satisfaction, CombatSearchV2SetupBiasPolicy,
    CombatSearchV2TurnPlanPolicy,
};
use crate::ai::combat_state_key::combat_exact_state_hash_v2;
use crate::eval::combat_case::{combat_summary, CombatCase, CombatCaseRngSummary};
use crate::eval::combat_guidance_bundle::CombatGuidanceBundleV1;
use crate::eval::run_control::{
    run_control_session_fingerprint_v2, OracleRunCombatBudgetsV1, OracleRunCombatQualityPolicyV1,
    RunControlCombatSegmentMode, RunControlHpLossLimit, RunControlSearchCombatOptions,
    RunControlSession, RunControlSessionCheckpointV1,
};
use crate::state::core::{ActiveCombat, CombatContext};

pub const COMBAT_CASE_PRODUCTION_CONTEXT_SCHEMA_NAME: &str = "CombatCaseProductionContextV1";
pub const COMBAT_CASE_PRODUCTION_CONTEXT_SCHEMA_VERSION: u32 = 1;
pub const COMBAT_CASE_REPLAY_IDENTITY_SCHEMA_NAME: &str = "CombatCaseReplayIdentityV1";
pub const COMBAT_CASE_REPLAY_IDENTITY_SCHEMA_VERSION: u32 = 1;

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CombatCaseReplayCapabilityV1 {
    IsolatedProjection,
    ExactProductionState,
    ExactProductionOwner,
}

#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CombatCaseReplayIdentityV1 {
    pub schema_name: String,
    pub schema_version: u32,
    pub capability: CombatCaseReplayCapabilityV1,
    pub root_exact_state_hash: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub run_session_fingerprint: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub owner_policy_fingerprint: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum CombatCaseProductionOwnerV1 {
    OracleAnalysis {
        policy_fingerprint: String,
        budgets: CombatCaseOracleCombatBudgetsV1,
    },
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CombatCaseOracleCombatBudgetsV1 {
    pub hallway: CombatCaseSearchOptionsV1,
    pub elite: CombatCaseSearchOptionsV1,
    pub boss: CombatCaseSearchOptionsV1,
    pub quality_policy: OracleRunCombatQualityPolicyV1,
    pub initial_divisor: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub guidance_bundle: Option<CombatGuidanceBundleV1>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CombatCaseSearchOptionsV1 {
    pub max_nodes: Option<usize>,
    pub max_actions_per_line: Option<usize>,
    pub max_engine_steps_per_action: Option<usize>,
    pub wall_ms: Option<u64>,
    pub satisfaction: Option<CombatSearchV2Satisfaction>,
    pub max_hp_loss: Option<RunControlHpLossLimit>,
    pub potion_policy: Option<CombatSearchV2PotionPolicy>,
    pub max_potions_used: Option<u32>,
    pub allowed_potion_slots: Option<u64>,
    pub rollout_policy: Option<CombatSearchV2RolloutPolicy>,
    pub child_rollout_policy: Option<CombatSearchV2ChildRolloutPolicy>,
    pub rollout_max_evaluations: Option<usize>,
    pub rollout_max_actions: Option<usize>,
    pub rollout_beam_width: Option<usize>,
    pub turn_plan_policy: Option<CombatSearchV2TurnPlanPolicy>,
    pub phase_guard_policy: Option<CombatSearchV2PhaseGuardPolicy>,
    pub setup_bias_policy: Option<CombatSearchV2SetupBiasPolicy>,
    pub segment_mode: Option<RunControlCombatSegmentMode>,
    pub enable_legacy_no_win_rescue: bool,
    pub allow_smoke_bomb_survival_fallback: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CombatCaseProductionContextV1 {
    pub schema_name: String,
    pub schema_version: u32,
    pub root_exact_state_hash: String,
    pub run_session_fingerprint: String,
    pub combat_context: CombatContext,
    pub run_session_checkpoint: RunControlSessionCheckpointV1,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub production_owner: Option<CombatCaseProductionOwnerV1>,
}

pub fn capture_combat_case_production_context_v1(
    case: &CombatCase,
    session: &RunControlSession,
) -> Result<CombatCaseProductionContextV1, String> {
    let root_exact_state_hash =
        combat_exact_state_hash_v2(&case.position.engine, &case.position.combat);
    validate_case_against_session(case, session, &root_exact_state_hash)?;

    let mut run_session_checkpoint = RunControlSessionCheckpointV1::from_session(session);
    run_session_checkpoint.clear_combat_diagnostics_for_external_checkpoint();
    let active_combat = run_session_checkpoint
        .take_active_combat_for_external_ref()
        .ok_or_else(|| "production combat case requires an active combat checkpoint".to_string())?;
    let context = CombatCaseProductionContextV1 {
        schema_name: COMBAT_CASE_PRODUCTION_CONTEXT_SCHEMA_NAME.to_string(),
        schema_version: COMBAT_CASE_PRODUCTION_CONTEXT_SCHEMA_VERSION,
        root_exact_state_hash,
        run_session_fingerprint: run_control_session_fingerprint_v2(session),
        combat_context: active_combat.context,
        run_session_checkpoint,
        production_owner: None,
    };
    validate_context_payload(case, &context)?;
    Ok(context)
}

pub fn capture_oracle_analysis_combat_case_production_context_v1(
    case: &CombatCase,
    session: &RunControlSession,
    budgets: &OracleRunCombatBudgetsV1,
) -> Result<CombatCaseProductionContextV1, String> {
    let mut context = capture_combat_case_production_context_v1(case, session)?;
    let budgets = CombatCaseOracleCombatBudgetsV1::capture(budgets)?;
    context.production_owner = Some(CombatCaseProductionOwnerV1::OracleAnalysis {
        policy_fingerprint: owner_policy_fingerprint(&budgets),
        budgets,
    });
    Ok(context)
}

pub fn restore_combat_case_oracle_analysis_owner_v1(
    case: &CombatCase,
) -> Result<(RunControlSession, OracleRunCombatBudgetsV1), String> {
    let session = restore_combat_case_production_session_v1(case)?;
    let owner = case
        .production_context
        .as_ref()
        .and_then(|context| context.production_owner.as_ref())
        .ok_or_else(|| {
            "combat case has exact production state but no production owner".to_string()
        })?;
    validate_production_owner(owner)?;
    let CombatCaseProductionOwnerV1::OracleAnalysis { budgets, .. } = owner;
    Ok((session, budgets.restore()))
}

pub fn combat_case_replay_identity_v1(
    case: &CombatCase,
) -> Result<CombatCaseReplayIdentityV1, String> {
    let root_exact_state_hash =
        combat_exact_state_hash_v2(&case.position.engine, &case.position.combat);
    let Some(context) = case.production_context.as_ref() else {
        return Ok(CombatCaseReplayIdentityV1 {
            schema_name: COMBAT_CASE_REPLAY_IDENTITY_SCHEMA_NAME.to_string(),
            schema_version: COMBAT_CASE_REPLAY_IDENTITY_SCHEMA_VERSION,
            capability: CombatCaseReplayCapabilityV1::IsolatedProjection,
            root_exact_state_hash,
            run_session_fingerprint: None,
            owner_policy_fingerprint: None,
        });
    };
    validate_context_payload(case, context)?;
    let owner_policy_fingerprint = context.production_owner.as_ref().map(|owner| {
        let CombatCaseProductionOwnerV1::OracleAnalysis {
            policy_fingerprint, ..
        } = owner;
        policy_fingerprint.clone()
    });
    Ok(CombatCaseReplayIdentityV1 {
        schema_name: COMBAT_CASE_REPLAY_IDENTITY_SCHEMA_NAME.to_string(),
        schema_version: COMBAT_CASE_REPLAY_IDENTITY_SCHEMA_VERSION,
        capability: if owner_policy_fingerprint.is_some() {
            CombatCaseReplayCapabilityV1::ExactProductionOwner
        } else {
            CombatCaseReplayCapabilityV1::ExactProductionState
        },
        root_exact_state_hash,
        run_session_fingerprint: Some(context.run_session_fingerprint.clone()),
        owner_policy_fingerprint,
    })
}

impl CombatCaseOracleCombatBudgetsV1 {
    fn capture(budgets: &OracleRunCombatBudgetsV1) -> Result<Self, String> {
        if budgets.initial_divisor == 0 {
            return Err("oracle combat initial divisor must be positive".to_string());
        }
        Ok(Self {
            hallway: CombatCaseSearchOptionsV1::capture(&budgets.hallway)?,
            elite: CombatCaseSearchOptionsV1::capture(&budgets.elite)?,
            boss: CombatCaseSearchOptionsV1::capture(&budgets.boss)?,
            quality_policy: budgets.quality_policy,
            initial_divisor: budgets.initial_divisor,
            guidance_bundle: budgets.guidance_bundle.as_deref().cloned(),
        })
    }

    fn restore(&self) -> OracleRunCombatBudgetsV1 {
        OracleRunCombatBudgetsV1 {
            hallway: self.hallway.restore(),
            elite: self.elite.restore(),
            boss: self.boss.restore(),
            quality_policy: self.quality_policy,
            initial_divisor: self.initial_divisor,
            guidance_bundle: self.guidance_bundle.clone().map(Arc::new),
        }
    }
}

impl CombatCaseSearchOptionsV1 {
    fn capture(options: &RunControlSearchCombatOptions) -> Result<Self, String> {
        if options.profile.is_some() {
            return Err(
                "combat case owner context does not support an implicit named search profile"
                    .to_string(),
            );
        }
        if !options.work_quanta.is_empty() {
            return Err(
                "combat case owner context requires externally serviced search quanta".to_string(),
            );
        }
        Ok(Self {
            max_nodes: options.max_nodes,
            max_actions_per_line: options.max_actions_per_line,
            max_engine_steps_per_action: options.max_engine_steps_per_action,
            wall_ms: options.wall_ms,
            satisfaction: options.satisfaction,
            max_hp_loss: options.max_hp_loss,
            potion_policy: options.potion_policy,
            max_potions_used: options.max_potions_used,
            allowed_potion_slots: options.allowed_potion_slots,
            rollout_policy: options.rollout_policy,
            child_rollout_policy: options.child_rollout_policy,
            rollout_max_evaluations: options.rollout_max_evaluations,
            rollout_max_actions: options.rollout_max_actions,
            rollout_beam_width: options.rollout_beam_width,
            turn_plan_policy: options.turn_plan_policy,
            phase_guard_policy: options.phase_guard_policy,
            setup_bias_policy: options.setup_bias_policy,
            segment_mode: options.segment_mode,
            enable_legacy_no_win_rescue: options.enable_legacy_no_win_rescue,
            allow_smoke_bomb_survival_fallback: options.allow_smoke_bomb_survival_fallback,
        })
    }

    fn restore(&self) -> RunControlSearchCombatOptions {
        RunControlSearchCombatOptions {
            profile: None,
            max_nodes: self.max_nodes,
            max_actions_per_line: self.max_actions_per_line,
            max_engine_steps_per_action: self.max_engine_steps_per_action,
            wall_ms: self.wall_ms,
            satisfaction: self.satisfaction,
            max_hp_loss: self.max_hp_loss,
            potion_policy: self.potion_policy,
            max_potions_used: self.max_potions_used,
            allowed_potion_slots: self.allowed_potion_slots,
            rollout_policy: self.rollout_policy,
            child_rollout_policy: self.child_rollout_policy,
            rollout_max_evaluations: self.rollout_max_evaluations,
            rollout_max_actions: self.rollout_max_actions,
            rollout_beam_width: self.rollout_beam_width,
            turn_plan_policy: self.turn_plan_policy,
            phase_guard_policy: self.phase_guard_policy,
            setup_bias_policy: self.setup_bias_policy,
            segment_mode: self.segment_mode,
            enable_legacy_no_win_rescue: self.enable_legacy_no_win_rescue,
            allow_smoke_bomb_survival_fallback: self.allow_smoke_bomb_survival_fallback,
            work_quanta: Vec::new(),
        }
    }
}

pub fn validate_combat_case_production_context_v1(case: &CombatCase) -> Result<(), String> {
    let context = case
        .production_context
        .as_ref()
        .ok_or_else(|| "combat case has no exact production context".to_string())?;
    validate_context_payload(case, context).map(|_| ())
}

pub fn restore_combat_case_production_session_v1(
    case: &CombatCase,
) -> Result<RunControlSession, String> {
    let context = case
        .production_context
        .as_ref()
        .ok_or_else(|| "combat case supports isolated replay only".to_string())?;
    validate_context_payload(case, context)
}

fn validate_context_payload(
    case: &CombatCase,
    context: &CombatCaseProductionContextV1,
) -> Result<RunControlSession, String> {
    validate_context_header(context)?;
    if let Some(owner) = context.production_owner.as_ref() {
        validate_production_owner(owner)?;
    }
    let case_root = combat_exact_state_hash_v2(&case.position.engine, &case.position.combat);
    if context.root_exact_state_hash != case_root {
        return Err(format!(
            "combat case root hash mismatch: context {}, case {case_root}",
            context.root_exact_state_hash
        ));
    }
    let session = restore_context_session(case, context)?;
    validate_case_against_session(case, &session, &case_root)?;
    let restored_fingerprint = run_control_session_fingerprint_v2(&session);
    if context.run_session_fingerprint != restored_fingerprint {
        return Err(format!(
            "combat case run-context fingerprint mismatch: context {}, checkpoint {restored_fingerprint}",
            context.run_session_fingerprint
        ));
    }
    Ok(session)
}

fn validate_production_owner(owner: &CombatCaseProductionOwnerV1) -> Result<(), String> {
    let CombatCaseProductionOwnerV1::OracleAnalysis {
        policy_fingerprint,
        budgets,
    } = owner;
    if budgets.initial_divisor == 0 {
        return Err("combat case owner initial divisor must be positive".to_string());
    }
    let actual = owner_policy_fingerprint(budgets);
    if policy_fingerprint != &actual {
        return Err(format!(
            "combat case owner-policy fingerprint mismatch: context {policy_fingerprint}, payload {actual}"
        ));
    }
    Ok(())
}

fn owner_policy_fingerprint(budgets: &CombatCaseOracleCombatBudgetsV1) -> String {
    crate::eval::fingerprint::hash_serializable(budgets)
}

fn restore_context_session(
    case: &CombatCase,
    context: &CombatCaseProductionContextV1,
) -> Result<RunControlSession, String> {
    let mut checkpoint = context.run_session_checkpoint.clone();
    if checkpoint.take_active_combat_for_external_ref().is_some() {
        return Err(
            "combat case production checkpoint must externalize its active combat".to_string(),
        );
    }
    checkpoint.restore_active_combat_from_external_ref(ActiveCombat::new(
        case.position.engine.clone(),
        case.position.combat.clone(),
        context.combat_context.clone(),
    ));
    checkpoint.into_session()
}

fn validate_context_header(context: &CombatCaseProductionContextV1) -> Result<(), String> {
    if context.schema_name != COMBAT_CASE_PRODUCTION_CONTEXT_SCHEMA_NAME
        || context.schema_version != COMBAT_CASE_PRODUCTION_CONTEXT_SCHEMA_VERSION
    {
        return Err(format!(
            "unsupported combat case production context {} version {}",
            context.schema_name, context.schema_version
        ));
    }
    Ok(())
}

fn validate_case_against_session(
    case: &CombatCase,
    session: &RunControlSession,
    expected_root: &str,
) -> Result<(), String> {
    let session_position = session.current_active_combat_position()?;
    let session_root =
        combat_exact_state_hash_v2(&session_position.engine, &session_position.combat);
    if session_root != expected_root {
        return Err(format!(
            "combat case active-combat hash mismatch: case {expected_root}, checkpoint {session_root}"
        ));
    }
    if case.source.seed != session.run_state.seed
        || case.source.ascension != session.run_state.ascension_level
    {
        return Err(format!(
            "combat case source mismatch: case seed {} A{}, checkpoint seed {} A{}",
            case.source.seed,
            case.source.ascension,
            session.run_state.seed,
            session.run_state.ascension_level
        ));
    }

    let expected_run = (
        session.run_state.act_num,
        session.run_state.floor_num,
        session.run_state.current_hp,
        session.run_state.max_hp,
        session.run_state.gold,
        session.run_state.master_deck.len(),
        session.run_state.relics.len(),
        session.run_state.potions.len(),
    );
    let case_run = (
        case.run.act,
        case.run.floor,
        case.run.hp,
        case.run.max_hp,
        case.run.gold,
        case.run.deck_size,
        case.run.relic_count,
        case.run.potion_slots,
    );
    if case_run != expected_run {
        return Err("combat case run summary does not match production checkpoint".to_string());
    }
    if case.combat != combat_summary(&case.position) {
        return Err("combat case combat summary does not match its exact root".to_string());
    }
    if case.run_rng != CombatCaseRngSummary::from_pool(&session.run_state.rng_pool) {
        return Err("combat case run RNG summary does not match production checkpoint".to_string());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::eval::combat_case::{CombatCaseGap, CombatCaseRunSummary, CombatCaseSource};
    use crate::eval::run_control::{RunControlConfig, RunControlSession};
    use crate::state::core::{ActiveCombat, CombatContext, EngineState, RoomCombatContext};
    use crate::state::map::node::RoomType;

    fn exact_fixture() -> (CombatCase, RunControlSession) {
        let mut session = RunControlSession::new(RunControlConfig {
            seed: 71,
            ascension_level: 4,
            player_class: "Ironclad",
            ..RunControlConfig::default()
        });
        let combat = crate::test_support::blank_test_combat();
        session.run_state.act_num = 2;
        session.run_state.floor_num = 23;
        session.run_state.current_hp = combat.entities.player.current_hp;
        session.run_state.max_hp = combat.entities.player.max_hp;
        session.run_state.gold = combat.entities.player.gold;
        session.run_state.master_deck = combat.meta.master_deck_snapshot.to_vec();
        session.run_state.relics = combat.entities.player.relics.clone();
        session.run_state.potions = combat.entities.potions.clone();
        session.run_state.rng_pool = combat.rng.pool.clone();
        session.engine_state = EngineState::CombatPlayerTurn;
        session.active_combat = Some(ActiveCombat::new(
            EngineState::CombatPlayerTurn,
            combat,
            CombatContext::Room(RoomCombatContext {
                room_type: RoomType::MonsterRoom,
            }),
        ));
        let position = session.current_active_combat_position().unwrap();
        let case = CombatCase::new(
            CombatCaseSource {
                seed: 71,
                ascension: 4,
                generation: 3,
                branch_id: 9,
                parent_id: Some(8),
            },
            CombatCaseGap {
                boundary: "fixture".to_string(),
                reason: "contract".to_string(),
                search_nodes: 10,
                search_ms: 20,
                rescue_search_nodes: 0,
                rescue_search_ms: 0,
            },
            CombatCaseRunSummary {
                act: session.run_state.act_num,
                floor: session.run_state.floor_num,
                hp: session.run_state.current_hp,
                max_hp: session.run_state.max_hp,
                gold: session.run_state.gold,
                deck_size: session.run_state.master_deck.len(),
                relic_count: session.run_state.relics.len(),
                potion_slots: session.run_state.potions.len(),
            },
            Vec::new(),
            None,
            Vec::new(),
            CombatCaseRngSummary::from_pool(&session.run_state.rng_pool),
            position,
        );
        (case, session)
    }

    #[test]
    fn exact_context_capture_round_trips_the_production_state() {
        let (mut case, session) = exact_fixture();
        case.production_context =
            Some(capture_combat_case_production_context_v1(&case, &session).unwrap());

        let payload = serde_json::to_value(&case).unwrap();
        assert!(payload["production_context"]["run_session_checkpoint"][2].is_null());

        assert_eq!(
            case.replay_capability_v1().unwrap(),
            CombatCaseReplayCapabilityV1::ExactProductionState
        );
        let identity = case.replay_identity_v1().unwrap();
        assert_eq!(
            identity.root_exact_state_hash,
            combat_exact_state_hash_v2(&case.position.engine, &case.position.combat)
        );
        assert_eq!(
            identity.run_session_fingerprint.as_deref(),
            case.production_context
                .as_ref()
                .map(|context| context.run_session_fingerprint.as_str())
        );
        assert!(identity.owner_policy_fingerprint.is_none());
        let restored = restore_combat_case_production_session_v1(&case).unwrap();
        assert_eq!(
            run_control_session_fingerprint_v2(&restored),
            run_control_session_fingerprint_v2(&session)
        );
    }

    #[test]
    fn exact_context_rejects_a_changed_combat_root() {
        let (mut case, session) = exact_fixture();
        case.production_context =
            Some(capture_combat_case_production_context_v1(&case, &session).unwrap());
        case.position.combat.entities.player.current_hp -= 1;
        case.combat = combat_summary(&case.position);

        let error = restore_combat_case_production_session_v1(&case).unwrap_err();
        assert!(error.contains("root hash mismatch"), "{error}");
    }

    #[test]
    fn oracle_owner_context_round_trips_policy_without_default_inference() {
        let (mut case, session) = exact_fixture();
        let mut options = RunControlSearchCombatOptions {
            max_nodes: Some(12_345),
            wall_ms: Some(678),
            satisfaction: Some(CombatSearchV2Satisfaction::FirstCompleteWin),
            potion_policy: Some(CombatSearchV2PotionPolicy::Never),
            max_potions_used: Some(0),
            ..RunControlSearchCombatOptions::default()
        };
        options.rollout_policy = Some(CombatSearchV2RolloutPolicy::TurnBeamNoPotion);
        let mut budgets = OracleRunCombatBudgetsV1::uniform(options);
        budgets.quality_policy = OracleRunCombatQualityPolicyV1::StrategicRun;
        budgets.initial_divisor = 3;
        case.production_context = Some(
            capture_oracle_analysis_combat_case_production_context_v1(&case, &session, &budgets)
                .unwrap(),
        );

        assert_eq!(
            case.replay_capability_v1().unwrap(),
            CombatCaseReplayCapabilityV1::ExactProductionOwner
        );
        let identity = case.replay_identity_v1().unwrap();
        assert!(identity.run_session_fingerprint.is_some());
        assert!(identity.owner_policy_fingerprint.is_some());
        let (_, restored) = restore_combat_case_oracle_analysis_owner_v1(&case).unwrap();
        assert_eq!(restored.hallway.max_nodes, Some(12_345));
        assert_eq!(restored.hallway.wall_ms, Some(678));
        assert_eq!(
            restored.hallway.satisfaction,
            Some(CombatSearchV2Satisfaction::FirstCompleteWin)
        );
        assert_eq!(restored.initial_divisor, 3);
        assert_eq!(
            restored.quality_policy,
            OracleRunCombatQualityPolicyV1::StrategicRun
        );
    }

    #[test]
    fn oracle_owner_context_rejects_a_tampered_policy_payload() {
        let (mut case, session) = exact_fixture();
        let budgets = OracleRunCombatBudgetsV1::uniform(RunControlSearchCombatOptions::default());
        case.production_context = Some(
            capture_oracle_analysis_combat_case_production_context_v1(&case, &session, &budgets)
                .unwrap(),
        );
        let owner = case
            .production_context
            .as_mut()
            .unwrap()
            .production_owner
            .as_mut()
            .unwrap();
        let CombatCaseProductionOwnerV1::OracleAnalysis { budgets, .. } = owner;
        budgets.hallway.max_nodes = Some(123_456);

        let error = case.replay_capability_v1().unwrap_err();
        assert!(
            error.contains("owner-policy fingerprint mismatch"),
            "{error}"
        );
    }

    #[test]
    fn exact_context_rejects_a_foreign_run_fingerprint() {
        let (mut case, session) = exact_fixture();
        case.production_context =
            Some(capture_combat_case_production_context_v1(&case, &session).unwrap());
        case.production_context
            .as_mut()
            .unwrap()
            .run_session_fingerprint = "foreign".to_string();

        let error = restore_combat_case_production_session_v1(&case).unwrap_err();
        assert!(
            error.contains("run-context fingerprint mismatch"),
            "{error}"
        );
    }

    #[test]
    fn legacy_case_remains_isolated_only() {
        let (case, _) = exact_fixture();

        assert_eq!(
            case.replay_capability_v1().unwrap(),
            CombatCaseReplayCapabilityV1::IsolatedProjection
        );
        let identity = case.replay_identity_v1().unwrap();
        assert_eq!(
            identity.root_exact_state_hash,
            combat_exact_state_hash_v2(&case.position.engine, &case.position.combat)
        );
        assert!(identity.run_session_fingerprint.is_none());
        assert!(identity.owner_policy_fingerprint.is_none());
        assert!(restore_combat_case_production_session_v1(&case)
            .unwrap_err()
            .contains("isolated replay only"));
    }

    #[test]
    fn derived_position_refresh_clears_exact_production_context() {
        let (mut case, session) = exact_fixture();
        case.production_context =
            Some(capture_combat_case_production_context_v1(&case, &session).unwrap());
        case.position.combat.entities.player.current_hp -= 1;

        case.refresh_derived_summaries_and_clear_production_context();

        assert!(case.production_context.is_none());
        assert_eq!(case.combat, combat_summary(&case.position));
        assert_eq!(case.run.hp, case.position.combat.entities.player.current_hp);
    }
}
