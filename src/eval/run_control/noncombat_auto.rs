use super::session::{RunControlCommandOutcome, RunControlSession};
use super::trace_annotation::RunControlTraceAnnotationV1;

pub(super) struct NonCombatAutoApplication {
    pub outcome: RunControlCommandOutcome,
    pub summary: String,
    pub stop_after_reason: Option<&'static str>,
}

pub(super) fn apply_planner_noncombat_policy(
    session: &mut RunControlSession,
) -> Result<Option<NonCombatAutoApplication>, String> {
    if let Some((outcome, summary)) = super::campfire_policy::apply_campfire_policy_action(session)?
    {
        return Ok(Some(NonCombatAutoApplication {
            outcome,
            summary,
            stop_after_reason: None,
        }));
    }
    if let Some((outcome, summary)) = super::shop_policy::apply_shop_policy_action(session)? {
        return Ok(Some(NonCombatAutoApplication {
            outcome,
            summary,
            stop_after_reason: Some(
                "shop policy changed shop/run state; inspect shop before continuing",
            ),
        }));
    }
    if let Some((outcome, summary)) =
        super::run_choice_policy::apply_run_choice_policy_deck_selection(session)?
    {
        return Ok(Some(NonCombatAutoApplication {
            outcome,
            summary,
            stop_after_reason: None,
        }));
    }
    if let Some((outcome, summary)) =
        super::boss_relic_policy::apply_boss_relic_policy_pick(session)?
    {
        return Ok(Some(NonCombatAutoApplication {
            outcome,
            summary,
            stop_after_reason: None,
        }));
    }
    if let Some((outcome, summary)) = super::event_policy::apply_event_policy_choice(session)? {
        return Ok(Some(NonCombatAutoApplication {
            outcome,
            summary,
            stop_after_reason: None,
        }));
    }
    if let Some((outcome, summary)) =
        super::card_reward_auto::apply_card_reward_policy_pick(session)?
    {
        return Ok(Some(NonCombatAutoApplication {
            outcome,
            summary,
            stop_after_reason: None,
        }));
    }
    if let Some((outcome, summary)) = super::card_reward_auto::apply_card_reward_item_open(session)?
    {
        return Ok(Some(NonCombatAutoApplication {
            outcome,
            summary,
            stop_after_reason: None,
        }));
    }

    Ok(None)
}

pub(super) fn apply_branch_experiment_noncombat_policy(
    session: &mut RunControlSession,
) -> Result<Option<NonCombatAutoApplication>, String> {
    if let Some((outcome, summary)) =
        super::event_policy::apply_match_and_keep_policy_choice(session)?
    {
        return Ok(Some(NonCombatAutoApplication {
            outcome,
            summary,
            stop_after_reason: None,
        }));
    }

    Ok(None)
}

pub(super) fn planner_noncombat_policy_stop_annotation(
    session: &RunControlSession,
) -> Result<Option<(RunControlTraceAnnotationV1, String)>, String> {
    super::card_reward_auto::card_reward_policy_stop_annotation(session)
}
