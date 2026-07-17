use std::env;
use std::path::PathBuf;

use sts_combat_planner::{
    load_combat_outcome_training_batch_v1, save_combat_outcome_model_artifact_v1,
    train_combat_outcome_model_artifact_v1, CombatOutcomeModelTrainingConfigV1,
};

fn main() {
    if let Err(message) = run() {
        eprintln!("combat_outcome_train: {message}");
        std::process::exit(2);
    }
}

fn run() -> Result<(), String> {
    let mut args = env::args_os().skip(1);
    let output = args.next().map(PathBuf::from).ok_or_else(usage)?;
    let model_id = args
        .next()
        .and_then(|value| value.into_string().ok())
        .ok_or_else(usage)?;
    let inputs = args.map(PathBuf::from).collect::<Vec<_>>();
    if inputs.is_empty() {
        return Err(usage());
    }
    let batches = inputs
        .iter()
        .map(|path| load_combat_outcome_training_batch_v1(path))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| format!("could not load training batch: {error:?}"))?;
    let artifact = train_combat_outcome_model_artifact_v1(
        &batches,
        model_id,
        CombatOutcomeModelTrainingConfigV1::default(),
        5,
        0,
    )
    .map_err(|error| format!("model was not produced: {error:?}"))?;
    save_combat_outcome_model_artifact_v1(&output, &artifact)
        .map_err(|error| format!("could not save model artifact: {error:?}"))?;
    println!(
        "saved {} ({} fit cases, {} calibration cases)",
        output.display(),
        artifact.split.training_case_ids.len(),
        artifact.split.calibration_case_ids.len()
    );
    Ok(())
}

fn usage() -> String {
    "usage: combat_outcome_train <output-model.json> <model-id> <batch.json> [batch.json ...]"
        .to_string()
}
