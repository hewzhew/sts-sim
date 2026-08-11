use serde_json::Value;

use super::{RunContract, SourceIdentity};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CapsuleReuseDecision {
    Exact,
    Incompatible,
}

pub fn decide_manifest_reuse(
    manifest: &Value,
    expected_contract: RunContract,
    expected_source: &SourceIdentity,
) -> CapsuleReuseDecision {
    if manifest.get("schema").and_then(Value::as_str) != Some("branch_tiny_run_capsule_v5") {
        return CapsuleReuseDecision::Incompatible;
    }
    let Some(contract_value) = manifest.get("run_contract") else {
        return CapsuleReuseDecision::Incompatible;
    };
    let Some(source_value) = manifest.get("source_identity") else {
        return CapsuleReuseDecision::Incompatible;
    };
    let Ok(contract) = serde_json::from_value::<RunContract>(contract_value.clone()) else {
        return CapsuleReuseDecision::Incompatible;
    };
    let Ok(source) = serde_json::from_value::<SourceIdentity>(source_value.clone()) else {
        return CapsuleReuseDecision::Incompatible;
    };
    if contract == expected_contract && source == *expected_source {
        CapsuleReuseDecision::Exact
    } else {
        CapsuleReuseDecision::Incompatible
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::runtime::branch::{Args, RunObjective};

    fn args(seed: u64) -> Args {
        Args {
            seed,
            ascension: 0,
            objective: RunObjective::FirstVictory,
            generations: 1,
            max_branches: 1,
            auto_ops: 1,
            search_nodes: 1,
            search_ms: 1,
            rescue_search_nodes: 1,
            rescue_search_ms: 1,
            boss_search_nodes: 1,
            boss_search_ms: 1,
            wall_ms: Some(1),
            checkpoint_before_atomic_combat_search_session: false,
            wall_capped_search_budget: false,
            wall_capped_boss_budget: false,
        }
    }

    fn source_identity() -> SourceIdentity {
        SourceIdentity {
            git_commit: Some("abc123".to_string()),
            git_dirty: Some(false),
        }
    }

    #[test]
    fn exact_reuse_requires_matching_contract_and_source_identity() {
        let contract = RunContract::from_args(args(1));
        let source = source_identity();
        let manifest = json!({
            "schema": "branch_tiny_run_capsule_v5",
            "run_contract": contract,
            "source_identity": source,
        });

        assert_eq!(
            decide_manifest_reuse(&manifest, contract, &source_identity()),
            CapsuleReuseDecision::Exact
        );
    }

    #[test]
    fn manifest_without_current_identity_is_incompatible() {
        let manifest = json!({
            "args": {"seed": 1}
        });

        assert_eq!(
            decide_manifest_reuse(
                &manifest,
                RunContract::from_args(args(1)),
                &source_identity()
            ),
            CapsuleReuseDecision::Incompatible
        );
    }

    #[test]
    fn old_or_missing_schema_is_incompatible_even_with_matching_payloads() {
        let contract = RunContract::from_args(args(1));
        let source = source_identity();
        for schema in [None, Some("branch_tiny_run_capsule_v4")] {
            let mut manifest = json!({
                "run_contract": contract,
                "source_identity": source,
            });
            if let Some(schema) = schema {
                manifest["schema"] = json!(schema);
            }

            assert_eq!(
                decide_manifest_reuse(&manifest, contract, &source_identity()),
                CapsuleReuseDecision::Incompatible
            );
        }
    }

    #[test]
    fn mismatched_contract_is_incompatible() {
        let source = source_identity();
        let manifest = json!({
            "schema": "branch_tiny_run_capsule_v5",
            "run_contract": RunContract::from_args(args(1)),
            "source_identity": source,
        });

        assert_eq!(
            decide_manifest_reuse(
                &manifest,
                RunContract::from_args(args(2)),
                &source_identity()
            ),
            CapsuleReuseDecision::Incompatible
        );
    }
}
