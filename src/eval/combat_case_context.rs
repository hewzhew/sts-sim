use serde::{Deserialize, Serialize};

use crate::ai::combat_state_key::combat_exact_state_hash_v2;
use crate::eval::combat_case::{combat_summary, CombatCase, CombatCaseRngSummary};
use crate::eval::run_control::{
    run_control_session_fingerprint_v2, RunControlSession, RunControlSessionCheckpointV1,
};
use crate::state::core::{ActiveCombat, CombatContext};

pub const COMBAT_CASE_PRODUCTION_CONTEXT_SCHEMA_NAME: &str = "CombatCaseProductionContextV1";
pub const COMBAT_CASE_PRODUCTION_CONTEXT_SCHEMA_VERSION: u32 = 1;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CombatCaseReplayCapabilityV1 {
    IsolatedProjection,
    ExactProductionOwner,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CombatCaseProductionContextV1 {
    pub schema_name: String,
    pub schema_version: u32,
    pub root_exact_state_hash: String,
    pub run_session_fingerprint: String,
    pub combat_context: CombatContext,
    pub run_session_checkpoint: RunControlSessionCheckpointV1,
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
    };
    validate_context_payload(case, &context)?;
    Ok(context)
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
    fn exact_context_capture_round_trips_the_production_owner() {
        let (mut case, session) = exact_fixture();
        case.production_context =
            Some(capture_combat_case_production_context_v1(&case, &session).unwrap());

        let payload = serde_json::to_value(&case).unwrap();
        assert!(payload["production_context"]["run_session_checkpoint"][2].is_null());

        assert_eq!(
            case.replay_capability_v1().unwrap(),
            CombatCaseReplayCapabilityV1::ExactProductionOwner
        );
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
