use std::fs;
use std::io::{self, Write};
use std::path::PathBuf;

use clap::{Parser, ValueEnum};
use sts_simulator::eval::run_play::{
    canonical_player_class, parse_run_play_command, render_run_play_state, run_play_help,
    RunPlayConfig, RunPlaySession,
};

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
    skip_neow: bool,

    #[arg(long)]
    script: Option<PathBuf>,
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
    let mut session = RunPlaySession::new(RunPlayConfig {
        seed: args.seed,
        ascension_level: args.ascension,
        final_act: args.final_act,
        player_class,
        skip_neow: args.skip_neow,
    });

    println!("{}", render_run_play_state(&session));
    if let Some(script) = args.script.as_ref() {
        run_script(&mut session, script)?;
    } else {
        run_repl(&mut session)?;
    }
    Ok(())
}

fn run_script(session: &mut RunPlaySession, script: &PathBuf) -> Result<(), String> {
    let payload = fs::read_to_string(script).map_err(|err| err.to_string())?;
    for (line_number, line) in payload.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        println!("> {trimmed}");
        if execute_line(session, trimmed)
            .map_err(|err| format!("{}:{}: {err}", script.display(), line_number + 1))?
        {
            break;
        }
    }
    Ok(())
}

fn run_repl(session: &mut RunPlaySession) -> Result<(), String> {
    println!("{}", run_play_help());
    let stdin = io::stdin();
    loop {
        print!("run-play> ");
        io::stdout().flush().map_err(|err| err.to_string())?;
        let mut line = String::new();
        let bytes = stdin.read_line(&mut line).map_err(|err| err.to_string())?;
        if bytes == 0 {
            break;
        }
        if execute_line(session, &line)? {
            break;
        }
    }
    Ok(())
}

fn execute_line(session: &mut RunPlaySession, line: &str) -> Result<bool, String> {
    let command = parse_run_play_command(line)?;
    let outcome = session.apply_command(command)?;
    if !outcome.message.is_empty() {
        println!("{}", outcome.message);
    }
    Ok(outcome.should_quit)
}
