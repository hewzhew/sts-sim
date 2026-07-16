use serde_json::Value;

use super::{RunContract, SourceIdentity};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CapsuleReuseDecision {
    Exact,
    UnknownLegacy,
    Incompatible,
}

pub fn decide_manifest_reuse(
    manifest: &Value,
    expected_contract: RunContract,
    expected_source: &SourceIdentity,
) -> CapsuleReuseDecision {
    let Some(contract_value) = manifest.get("run_contract") else {
        return CapsuleReuseDecision::UnknownLegacy;
    };
    let Some(source_value) = manifest.get("source_identity") else {
        return CapsuleReuseDecision::UnknownLegacy;
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
            checkpoint_before_combat_portfolio: false,
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
            "run_contract": contract,
            "source_identity": source,
        });

        assert_eq!(
            decide_manifest_reuse(&manifest, contract, &source_identity()),
            CapsuleReuseDecision::Exact
        );
    }

    #[test]
    fn legacy_manifest_without_identity_is_unknown_not_exact() {
        let manifest = json!({
            "args": {"seed": 1}
        });

        assert_eq!(
            decide_manifest_reuse(
                &manifest,
                RunContract::from_args(args(1)),
                &source_identity()
            ),
            CapsuleReuseDecision::UnknownLegacy
        );
    }

    #[test]
    fn mismatched_contract_is_incompatible() {
        let source = source_identity();
        let manifest = json!({
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
