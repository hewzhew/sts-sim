use std::path::PathBuf;

use clap::{Parser, ValueEnum};
use sts_simulator::eval::run_control::{
    canonical_player_class, render_run_control_state, AutoCombatCaptureConfig, RunControlConfig,
    RunControlSession, SessionTraceRecorder,
};

mod terminal;
use terminal::{run_repl, run_script};

#[derive(Parser, Debug)]
#[command(about = "Thin simulator run/play driver with exact combat capture support")]
struct Args {
    #[arg(long, default_value_t = 1)]
    seed: u64,

    #[arg(long, default_value_t = 0)]
    ascension: u8,

    #[arg(long, value_enum, default_value_t = CliPlayerClass::Ironclad)]
    class: CliPlayerClass,

    #[arg(long)]
    final_act: bool,

    #[arg(long)]
    script: Option<PathBuf>,

    #[arg(long)]
    trace: Option<PathBuf>,

    #[arg(long)]
    auto_capture_combat: bool,

    #[arg(long)]
    auto_capture_combat_root: Option<PathBuf>,
}

#[derive(Clone, Debug, ValueEnum)]
enum CliPlayerClass {
    Ironclad,
    Silent,
    Defect,
    Watcher,
}

impl CliPlayerClass {
    fn as_str(&self) -> &'static str {
        match self {
            Self::Ironclad => "ironclad",
            Self::Silent => "silent",
            Self::Defect => "defect",
            Self::Watcher => "watcher",
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    let player_class = canonical_player_class(args.class.as_str())?;
    let mut session = RunControlSession::new(RunControlConfig {
        seed: args.seed,
        ascension_level: args.ascension,
        final_act: args.final_act,
        player_class,
        reward_automation: Default::default(),
        auto_capture: AutoCombatCaptureConfig {
            enabled: args.auto_capture_combat,
            root: args.auto_capture_combat_root.clone(),
        },
    });

    println!("{}", render_run_control_state(&session));
    if args.auto_capture_combat {
        let root = args
            .auto_capture_combat_root
            .as_ref()
            .map(|path| path.display().to_string())
            .unwrap_or_else(|| "tools/artifacts/benchmarks/seed<seed>_act<act>".to_string());
        println!("auto combat capture enabled: {root}");
    }
    let mut trace = args
        .trace
        .as_ref()
        .map(|path| SessionTraceRecorder::new(path.clone(), &session));

    if let Some(script) = args.script.as_ref() {
        run_script(&mut session, script.as_path(), trace.as_mut())?;
    } else {
        run_repl(&mut session, trace.as_mut())?;
    }
    Ok(())
}
