use super::BranchStatus;

pub(super) fn reject_dirty_win_status(
    rejects_new_curses: bool,
    lane_label: &'static str,
    status: BranchStatus,
    before_curses: usize,
    after_curses: usize,
) -> BranchStatus {
    if !rejects_new_curses
        || matches!(status, BranchStatus::CombatGap { .. })
        || after_curses <= before_curses
    {
        return status;
    }

    let gained_curses = after_curses.saturating_sub(before_curses);
    BranchStatus::CombatGap {
        boundary: "Combat".to_string(),
        reason: format!("{lane_label} rejected dirty win: gained {gained_curses} curse card(s)"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejecting_lane_turns_new_curse_advance_into_combat_gap() {
        let status = reject_dirty_win_status(
            true,
            "hallway_quality_potion_rescue",
            BranchStatus::AwaitingAuto {
                boundary: "Card Reward".to_string(),
                reason: "advanced".to_string(),
            },
            1,
            2,
        );

        match status {
            BranchStatus::CombatGap { boundary, reason } => {
                assert_eq!(boundary, "Combat");
                assert!(reason.contains("hallway_quality_potion_rescue rejected dirty win"));
                assert!(reason.contains("gained 1 curse card"));
            }
            _ => panic!("expected combat gap"),
        }
    }

    #[test]
    fn existing_combat_gap_is_not_rewritten() {
        let status = reject_dirty_win_status(
            true,
            "hallway_quality_potion_rescue",
            BranchStatus::CombatGap {
                boundary: "Combat".to_string(),
                reason: "no win".to_string(),
            },
            0,
            1,
        );

        match status {
            BranchStatus::CombatGap { reason, .. } => assert_eq!(reason, "no win"),
            _ => panic!("expected original combat gap"),
        }
    }
}
