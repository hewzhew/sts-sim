use std::fs;
use std::path::PathBuf;

use serde_json::{json, Value};

use super::run_chain_state::{capsule_state, manifest_wall_ms, CapsuleState};
use super::run_slice_request::ContinueSliceRequest;
use super::run_slice_result::RunSliceResult;
use super::{branch_runtime, Args, ArgsOverrides, ContinueCapsuleArgs};

pub(super) fn run(
    mut args: Args,
    overrides: ArgsOverrides,
    chain: ContinueCapsuleArgs,
) -> Result<(), String> {
    if args.wall_ms.is_none() {
        args.wall_ms = manifest_wall_ms(&chain.capsule)?;
    }
    if args.wall_ms.is_none() {
        return Err(format!(
            "--continue-capsule requires --wall-ms; no previous wall_ms found in {}",
            chain.capsule.join("manifest.json").display()
        ));
    }
    let mut slices = Vec::new();
    for index in 0..chain.max_slices {
        let before = capsule_state(&chain.capsule)?;
        if index == 0 && before.manifest_exists && !before.frontier_exists {
            slices.push(before.into_value(index, false, None, None));
            break;
        }
        let resume = before.frontier_exists;
        let slice = run_slice(args, overrides, &chain.capsule, resume);
        let after = capsule_state(&chain.capsule)?;
        let should_continue = after.is_wall_pause();
        let success = slice.is_ok();
        let slice_result = match &slice {
            Ok(result) => Some(serde_json::to_value(result).map_err(|err| err.to_string())?),
            Err(err) => Some(json!({"runtime_error": err})),
        };
        print_slice_summary(index, chain.max_slices, resume, success, &after);
        slices.push(after.into_value(index, resume, Some(success), slice_result));
        write_chain(&chain.capsule, chain.max_slices, &slices)?;
        if let Err(err) = slice {
            return Err(format!("continuation slice {index} failed: {err}"));
        }
        if !should_continue {
            break;
        }
    }
    write_chain(&chain.capsule, chain.max_slices, &slices)
}

fn run_slice(
    args: Args,
    overrides: ArgsOverrides,
    capsule_path: &PathBuf,
    resume: bool,
) -> Result<RunSliceResult, String> {
    let request = ContinueSliceRequest {
        args,
        overrides,
        capsule_path: capsule_path.clone(),
        resume,
        human_output: false,
    }
    .prepare()?;
    branch_runtime::BranchRuntime::run_slice(request)
}

fn write_chain(capsule: &PathBuf, max_slices: usize, slices: &[Value]) -> Result<(), String> {
    fs::create_dir_all(capsule).map_err(|err| err.to_string())?;
    let path = capsule.join("chain.json");
    let payload = json!({
        "schema": "branch_tiny_run_chain",
        "capsule": capsule.display().to_string(),
        "max_slices": max_slices,
        "slices": slices,
    });
    fs::write(
        &path,
        serde_json::to_string_pretty(&payload).map_err(|err| err.to_string())?,
    )
    .map_err(|err| format!("failed to write {}: {err}", path.display()))
}

fn print_slice_summary(
    index: usize,
    max_slices: usize,
    resumed: bool,
    success: bool,
    state: &CapsuleState,
) {
    println!(
        "continue_slice {}/{} resumed={} success={} status={} reason={} generation={} branch={} boundary={} owner={} frontier={} result={}",
        index + 1,
        max_slices,
        resumed,
        success,
        state.status.as_deref().unwrap_or("-"),
        state.reason.as_deref().unwrap_or("-"),
        state
            .generation
            .map(|value| value.to_string())
            .unwrap_or_else(|| "-".to_string()),
        state
            .branch_id
            .map(|value| value.to_string())
            .unwrap_or_else(|| "-".to_string()),
        state.boundary.as_deref().unwrap_or("-"),
        state.owner.as_deref().unwrap_or("-"),
        state.frontier_exists,
        state.result_exists
    );
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::super::run_contract::RunObjective;
    use super::*;

    fn sample_args(seed: u64) -> Args {
        Args {
            seed,
            ascension: 0,
            objective: RunObjective::FirstVictory,
            generations: 0,
            max_branches: 1,
            auto_ops: 64,
            search_nodes: 1,
            search_ms: 1,
            rescue_search_nodes: 1,
            rescue_search_ms: 1,
            boss_search_nodes: 1,
            boss_search_ms: 1,
            wall_ms: Some(5_000),
            checkpoint_before_combat_portfolio: false,
            shop_boss_preview_bundle_limit: 0,
            shop_boss_preview_target_floor: None,
            wall_capped_search_budget: false,
            wall_capped_boss_budget: false,
        }
    }

    #[test]
    fn run_chain_start_slice_uses_requested_seed() {
        let capsule = std::env::temp_dir().join("branch_tiny_chain_start_seed");
        let _ = fs::remove_dir_all(&capsule);

        run(
            sample_args(123),
            ArgsOverrides::default(),
            ContinueCapsuleArgs {
                capsule: capsule.clone(),
                max_slices: 1,
            },
        )
        .unwrap();

        let manifest: Value =
            serde_json::from_str(&fs::read_to_string(capsule.join("manifest.json")).unwrap())
                .unwrap();
        assert_eq!(manifest["run_contract"]["game"]["seed"], 123);
        let chain: Value =
            serde_json::from_str(&fs::read_to_string(capsule.join("chain.json")).unwrap()).unwrap();
        assert_eq!(chain["slices"][0]["runtime_success"], true);
        assert!(chain["slices"][0]["process_success"].is_null());
        assert_eq!(
            chain["slices"][0]["slice_result"]["contract"]["game"]["seed"],
            123
        );

        let _ = fs::remove_dir_all(capsule);
    }
}
