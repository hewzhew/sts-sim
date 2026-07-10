use super::session::{RunControlCommandOutcome, RunControlSession};

pub(super) struct NonCombatAutoApplication {
    pub outcome: RunControlCommandOutcome,
    pub summary: String,
}

pub(super) fn apply_branch_experiment_noncombat_policy(
    session: &mut RunControlSession,
) -> Result<Option<NonCombatAutoApplication>, String> {
    if let Some((outcome, summary)) =
        super::event_policy::apply_match_and_keep_policy_choice(session)?
    {
        return Ok(Some(NonCombatAutoApplication { outcome, summary }));
    }
    if let Some((outcome, summary)) =
        super::event_policy::apply_note_for_yourself_policy_choice(session)?
    {
        return Ok(Some(NonCombatAutoApplication { outcome, summary }));
    }

    Ok(None)
}
