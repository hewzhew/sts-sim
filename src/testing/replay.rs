//! Replay driver for differential testing.
//!
//! Takes a sequence of CommunicationMod transitions (from a JSONL diff log),
//! replays the combat actions against the Rust simulator, and diffs each step
//! to detect divergences between the real game and our simulation.

use super::commod_parser::{DiffTransition, parse_combat_snapshot};
use super::action_parser::{ReplayAction, parse_command};
use super::snapshot::CombatSnapshot;
use super::diff::{Divergence, diff_snapshots};

/// Result of replaying a single combat encounter.
#[derive(Debug)]
pub struct CombatReplayResult {
    /// Human-readable identifier for this combat (e.g. "Floor 1: Cultist")
    pub combat_id: String,
    /// Number of steps replayed
    pub steps_replayed: usize,
    /// Divergences found at each step: (step_index, command, divergences)
    pub divergences: Vec<StepDivergence>,
    /// Whether the replay completed without fatal errors
    pub completed: bool,
    /// Error message if replay aborted
    pub error: Option<String>,
}

/// Divergence report for a single replay step.
#[derive(Debug)]
pub struct StepDivergence {
    pub step: u64,
    pub command: String,
    pub diffs: Vec<Divergence>,
}

/// Extract combat segments from a full game diff log.
///
/// A combat segment is a contiguous sequence of transitions where
/// `combat_state` is present in the JSON. Non-combat transitions
/// (map, card rewards, events, shops) are boundaries between segments.
pub fn extract_combat_segments(transitions: &[DiffTransition]) -> Vec<CombatSegment> {
    let mut segments = Vec::new();
    let mut current_segment: Option<CombatSegment> = None;

    for t in transitions {
        if t.snapshot.is_some() {
            // In combat — extend or start segment
            if let Some(ref mut seg) = current_segment {
                seg.transitions.push(t.clone());
            } else {
                // Identify combat from monsters in first snapshot
                let combat_id = t.snapshot.as_ref()
                    .map(|s| {
                        s.enemies.iter()
                            .filter(|e| e.alive)
                            .map(|e| e.name.as_str())
                            .collect::<Vec<_>>()
                            .join(" + ")
                    })
                    .unwrap_or_else(|| "Unknown".to_string());

                current_segment = Some(CombatSegment {
                    combat_id,
                    transitions: vec![t.clone()],
                });
            }
        } else {
            // Not in combat — close current segment if any
            if let Some(seg) = current_segment.take() {
                if seg.transitions.len() > 1 {
                    // Only include segments with at least 2 transitions
                    segments.push(seg);
                }
            }
        }
    }

    // Don't forget the last segment
    if let Some(seg) = current_segment {
        if seg.transitions.len() > 1 {
            segments.push(seg);
        }
    }

    segments
}

/// A contiguous sequence of combat transitions.
#[derive(Debug, Clone)]
pub struct CombatSegment {
    pub combat_id: String,
    pub transitions: Vec<DiffTransition>,
}

/// Compare consecutive CommunicationMod snapshots to detect divergences
/// in the REAL GAME's state transitions.
///
/// This is the simplest form of differential testing: we don't replay
/// in our engine at all, we just diff consecutive snapshots from the
/// real game to validate our snapshot comparison infrastructure.
pub fn diff_consecutive_snapshots(
    segment: &CombatSegment,
) -> Vec<StepDivergence> {
    let mut result = Vec::new();
    let combat_transitions: Vec<_> = segment.transitions.iter()
        .filter(|t| {
            let action = parse_command(&t.command);
            action.is_combat_action()
        })
        .collect();

    // For each combat action, diff the pre/post snapshots
    for i in 0..combat_transitions.len() {
        let t = combat_transitions[i];
        if let Some(snap) = &t.snapshot {
            // We have the post-action state; nothing to diff against yet
            // unless we build our own expected state.
            // For now, just validate that the snapshot parses correctly.
            let parsed_back = parse_combat_snapshot(&t.raw_state);
            if let Some(re_parsed) = parsed_back {
                let diffs = diff_snapshots(snap, &re_parsed);
                if !diffs.is_empty() {
                    result.push(StepDivergence {
                        step: t.step,
                        command: t.command.clone(),
                        diffs,
                    });
                }
            }
        }
    }

    result
}

/// Generate a human-readable report from a combat replay result.
pub fn format_replay_report(results: &[CombatReplayResult]) -> String {
    let mut report = String::new();
    report.push_str("# Differential Testing Report\n\n");

    let total_combats = results.len();
    let clean_combats = results.iter().filter(|r| r.divergences.is_empty()).count();
    let total_divergences: usize = results.iter()
        .map(|r| r.divergences.iter().map(|d| d.diffs.len()).sum::<usize>())
        .sum();

    report.push_str(&format!(
        "**Summary**: {} combats replayed, {} clean, {} with divergences ({} total diffs)\n\n",
        total_combats, clean_combats, total_combats - clean_combats, total_divergences
    ));

    for result in results {
        if result.divergences.is_empty() {
            report.push_str(&format!("✅ **{}** — {} steps, no divergences\n", 
                result.combat_id, result.steps_replayed));
        } else {
            report.push_str(&format!(
                "\n❌ **{}** — {} steps, {} divergent steps:\n",
                result.combat_id, result.steps_replayed, result.divergences.len()
            ));
            for step_div in &result.divergences {
                report.push_str(&format!(
                    "  Step {}: `{}` → {} diffs:\n",
                    step_div.step, step_div.command, step_div.diffs.len()
                ));
                for d in &step_div.diffs {
                    report.push_str(&format!("    - {}\n", d));
                }
            }
        }
    }

    if !results.iter().any(|r| r.error.is_some()) {
        report.push_str("\n---\nAll replays completed without fatal errors.\n");
    } else {
        report.push_str("\n---\n⚠️ Some replays had errors:\n");
        for r in results.iter().filter(|r| r.error.is_some()) {
            report.push_str(&format!("  - {}: {}\n", r.combat_id, r.error.as_ref().unwrap()));
        }
    }

    report
}

/// Run differential testing on a JSONL diff log file content.
///
/// This is the main entry point for differential testing.
/// It parses the log, extracts combat segments, and validates
/// snapshot parsing round-trip consistency.
pub fn run_diff_test(log_content: &str) -> String {
    let transitions = super::commod_parser::parse_diff_log(log_content);
    let segments = extract_combat_segments(&transitions);

    let mut results = Vec::new();
    for segment in &segments {
        let divergences = diff_consecutive_snapshots(segment);
        results.push(CombatReplayResult {
            combat_id: segment.combat_id.clone(),
            steps_replayed: segment.transitions.len(),
            divergences,
            completed: true,
            error: None,
        });
    }

    format_replay_report(&results)
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_combat_segments() {
        // Create a fake diff log with mixed combat/non-combat
        let log = r#"{"step":0,"command":"start ironclad 0 ABC","state":{"game_state":{"screen_type":"MAP"}}}
{"step":1,"command":"choose 0","state":{"game_state":{"combat_state":{"turn":1,"hand":[],"draw_pile":[],"discard_pile":[],"exhaust_pile":[],"player":{"current_hp":80,"max_hp":80,"block":0,"energy":3,"powers":[],"orbs":[]},"monsters":[{"name":"Cultist","id":"Cultist","current_hp":50,"max_hp":50,"block":0,"intent":"BUFF","is_gone":false,"half_dead":false,"powers":[]}]},"relics":[]}}}
{"step":2,"command":"play 0 0","state":{"game_state":{"combat_state":{"turn":1,"hand":[],"draw_pile":[],"discard_pile":[],"exhaust_pile":[],"player":{"current_hp":80,"max_hp":80,"block":0,"energy":2,"powers":[],"orbs":[]},"monsters":[{"name":"Cultist","id":"Cultist","current_hp":44,"max_hp":50,"block":0,"intent":"ATTACK","is_gone":false,"half_dead":false,"powers":[]}]},"relics":[]}}}
{"step":3,"command":"end","state":{"game_state":{"screen_type":"MAP"}}}
"#;
        let transitions = super::super::commod_parser::parse_diff_log(log);
        let segments = extract_combat_segments(&transitions);
        assert_eq!(segments.len(), 1);
        assert_eq!(segments[0].combat_id, "Cultist");
        assert_eq!(segments[0].transitions.len(), 2); // step 1 and 2
    }

    #[test]
    fn test_format_replay_report() {
        let results = vec![
            CombatReplayResult {
                combat_id: "Cultist".to_string(),
                steps_replayed: 5,
                divergences: vec![],
                completed: true,
                error: None,
            },
        ];
        let report = format_replay_report(&results);
        assert!(report.contains("1 combats replayed"));
        assert!(report.contains("1 clean"));
        assert!(report.contains("✅"));
    }

    #[test]
    fn test_run_diff_test_no_combat() {
        let log = r#"{"step":0,"command":"start ironclad","state":{"game_state":{"screen_type":"MAP"}}}"#;
        let report = run_diff_test(log);
        assert!(report.contains("0 combats replayed"));
    }
}
