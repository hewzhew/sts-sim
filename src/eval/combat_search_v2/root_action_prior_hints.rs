use std::collections::HashMap;
use std::fs;
use std::path::Path;

use serde::Deserialize;

use crate::ai::combat_search_v2::CombatSearchV2RootActionPrior;

#[derive(Debug, Deserialize)]
struct RootActionPriorHintRecordV0 {
    schema_name: Option<String>,
    root_exact_state_hash: Option<String>,
    #[serde(default)]
    action_prior_hints: Vec<RootActionPriorHintV0>,
}

#[derive(Debug, Deserialize)]
struct RootActionPriorHintV0 {
    action_key: String,
    score: f64,
}

pub fn load_combat_root_action_prior_hints_jsonl_v0(
    path: &Path,
) -> Result<CombatSearchV2RootActionPrior, String> {
    let payload = fs::read_to_string(path).map_err(|err| {
        format!(
            "failed to read root action prior hints from {}: {err}",
            path.display()
        )
    })?;
    parse_combat_root_action_prior_hints_jsonl_v0(&payload)
}

pub fn parse_combat_root_action_prior_hints_jsonl_v0(
    payload: &str,
) -> Result<CombatSearchV2RootActionPrior, String> {
    let mut scores_by_state: HashMap<String, HashMap<String, f64>> = HashMap::new();
    for (line_index, line) in payload.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let record: RootActionPriorHintRecordV0 = serde_json::from_str(line).map_err(|err| {
            format!(
                "invalid CombatRootActionPriorHintV0 JSONL at line {}: {err}",
                line_index + 1
            )
        })?;
        if !matches!(
            record.schema_name.as_deref(),
            Some("CombatRootActionPriorHintV0")
        ) {
            continue;
        }
        let Some(state_hash) = record
            .root_exact_state_hash
            .filter(|state_hash| !state_hash.is_empty())
        else {
            continue;
        };
        let state_scores = scores_by_state.entry(state_hash).or_default();
        for hint in record.action_prior_hints {
            if hint.action_key.is_empty() || !hint.score.is_finite() {
                continue;
            }
            state_scores.insert(hint.action_key, hint.score);
        }
    }
    Ok(CombatSearchV2RootActionPrior::from_scores(scores_by_state))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_root_action_prior_hints_by_exact_state_and_action_key() {
        let payload = r#"
{"schema_name":"CombatRootActionPriorHintV0","root_exact_state_hash":"state-a","action_prior_hints":[{"action_key":"combat/end_turn","score":0.25},{"action_key":"combat/play_card/hand:0/card:Bash+0#10/target:monster_slot:0","score":0.75}]}
{"schema_name":"OtherSchema","root_exact_state_hash":"ignored","action_prior_hints":[{"action_key":"combat/end_turn","score":1.0}]}
{"schema_name":"CombatRootActionPriorHintV0","root_exact_state_hash":"state-b","action_prior_hints":[{"action_key":"combat/end_turn","score":0.5}]}
"#;

        let prior = parse_combat_root_action_prior_hints_jsonl_v0(payload)
            .expect("prior hints should parse");

        assert_eq!(prior.score("state-a", "combat/end_turn"), Some(0.25));
        assert_eq!(
            prior.score(
                "state-a",
                "combat/play_card/hand:0/card:Bash+0#10/target:monster_slot:0"
            ),
            Some(0.75)
        );
        assert_eq!(prior.score("state-b", "combat/end_turn"), Some(0.5));
        assert_eq!(prior.score("ignored", "combat/end_turn"), None);
        assert_eq!(prior.score("state-a", "missing"), None);
    }
}
