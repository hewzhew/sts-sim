use sts_simulator::diff::replay::{
    derive_combat_replay_view, inspect_combat_replay_step, load_live_session_replay_path,
    CombatReplayStepStatus,
};
use sts_simulator::verification::combat::build_live_split_combat_snapshots_from_root;

fn monster_line(root: &serde_json::Value) -> Result<(i64, i64, String, Vec<String>), String> {
    let (truth, observation) = build_live_split_combat_snapshots_from_root(root)?;
    let truth_monster = truth
        .get("monsters")
        .and_then(|monsters| monsters.as_array())
        .and_then(|monsters| monsters.first())
        .ok_or_else(|| "snapshot missing truth monster[0]".to_string())?;
    let observation_monster = observation
        .get("monsters")
        .and_then(|monsters| monsters.as_array())
        .and_then(|monsters| monsters.first())
        .ok_or_else(|| "snapshot missing observation monster[0]".to_string())?;

    let hp = truth_monster["current_hp"].as_i64().unwrap_or(-1);
    let block = truth_monster["block"].as_i64().unwrap_or(-1);
    let intent = observation_monster["intent"]
        .as_str()
        .unwrap_or("?")
        .to_string();
    let powers = truth_monster["powers"]
        .as_array()
        .map(|powers| {
            powers
                .iter()
                .map(|power| {
                    format!(
                        "{}({})",
                        power["id"].as_str().unwrap_or("?"),
                        power["amount"]
                    )
                })
                .collect()
        })
        .unwrap_or_default();

    Ok((hp, block, intent, powers))
}

fn print_step(
    step_index: usize,
    step: &sts_simulator::diff::replay::CombatReplayStep,
) -> Result<(), String> {
    let label = format!(
        "[{:02}] command_id={} response_id={:?} frame_id={:?}",
        step_index, step.command_id, step.response_id, step.state_frame_id
    );
    match step.status {
        CombatReplayStepStatus::Executable => {
            let (hp, block, intent, powers) = monster_line(&step.after_root)?;
            println!(
                "{} {:<30} | Monster HP: {:3}, Block: {:2}, Intent: {}",
                label, step.command_text, hp, block, intent
            );
            if !powers.is_empty() {
                println!("      Powers: {}", powers.join(", "));
            }
        }
        CombatReplayStepStatus::SkippedNoncombat => {
            println!("{label} {:<30} | skipped_noncombat", step.command_text);
        }
        CombatReplayStepStatus::Unsupported => {
            println!(
                "{label} {:<30} | unsupported {}",
                step.command_text,
                step.skip_reason.as_deref().unwrap_or("")
            );
        }
        CombatReplayStepStatus::InsufficientContext => {
            println!(
                "{label} {:<30} | insufficient_context {}",
                step.command_text,
                step.skip_reason.as_deref().unwrap_or("")
            );
        }
    }
    Ok(())
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        println!("Usage: cargo run --bin view_replay <replay_path> [step_index]");
        return;
    }

    let path = std::path::Path::new(&args[1]);
    let replay = match load_live_session_replay_path(path) {
        Ok(replay) => replay,
        Err(err) => {
            eprintln!("failed to load replay {}: {err}", path.display());
            return;
        }
    };
    let combat_view = derive_combat_replay_view(&replay);

    println!(
        "Replay: {} | frames={} steps={}",
        replay.source_path.as_deref().unwrap_or("<unknown>"),
        replay.total_frames,
        combat_view.steps.len()
    );

    if args.len() >= 3 {
        let step_index: usize = args[2].parse().unwrap_or(0);
        let Some(step) = combat_view.steps.get(step_index) else {
            eprintln!("step_index {} out of range", step_index);
            return;
        };
        if let Err(err) = print_step(step_index, step) {
            eprintln!("failed to render step {}: {err}", step_index);
            return;
        }
        if step.status == CombatReplayStepStatus::Executable {
            match inspect_combat_replay_step(&combat_view, step_index) {
                Ok(inspection) => {
                    if inspection.diffs.is_empty() {
                        println!("      diffs: []");
                    } else {
                        println!(
                            "      diffs: {}",
                            serde_json::to_string_pretty(&inspection.diffs)
                                .unwrap_or_else(|_| "[]".to_string())
                        );
                    }
                }
                Err(err) => eprintln!("failed to inspect step {}: {err}", step_index),
            }
        }
        return;
    }

    for (step_index, step) in combat_view.steps.iter().enumerate() {
        if let Err(err) = print_step(step_index, step) {
            eprintln!("failed to render step {}: {err}", step_index);
        }
    }
}
