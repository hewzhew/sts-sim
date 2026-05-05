use std::path::PathBuf;

use clap::{Parser, Subcommand};

use sts_simulator::fixtures::author_spec::CombatAuthorSpec;
use sts_simulator::fixtures::combat_case::{
    assert_case, case_from_scenario_fixture, compile_combat_author_case, load_case_from_path,
    write_case_to_path, CombatCaseReducer,
};
use sts_simulator::fixtures::scenario::ScenarioFixture;

#[derive(Parser, Debug)]
#[command(name = "combat_case")]
struct Args {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    Verify {
        #[arg(long)]
        case: PathBuf,
    },
    Reduce {
        #[arg(long)]
        case: PathBuf,
        #[arg(long)]
        out: PathBuf,
    },
    ConvertScenario {
        #[arg(long)]
        fixture: PathBuf,
        #[arg(long)]
        out: PathBuf,
    },
    CompileAuthorSpec {
        #[arg(long)]
        author_spec: PathBuf,
        #[arg(long)]
        out: PathBuf,
    },
    Materialize {
        #[arg(long)]
        case: PathBuf,
        #[arg(long)]
        out: PathBuf,
    },
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    match args.command {
        Command::Verify { case } => {
            let case = load_case_from_path(&case)?;
            assert_case(&case)?;
            println!("verified combat case '{}'", case.id);
        }
        Command::Reduce { case, out } => {
            let case = load_case_from_path(&case)?;
            let reduced = CombatCaseReducer::reduce(&case)?;
            write_case_to_path(&reduced, &out)?;
            println!("wrote {}", out.display());
        }
        Command::ConvertScenario { fixture, out } => {
            let payload = std::fs::read_to_string(&fixture)?;
            let fixture: ScenarioFixture = serde_json::from_str(&payload)?;
            let case = case_from_scenario_fixture(&fixture)?;
            write_case_to_path(&case, &out)?;
            println!("wrote {}", out.display());
        }
        Command::CompileAuthorSpec { author_spec, out } => {
            let payload = std::fs::read_to_string(&author_spec)?;
            let spec: CombatAuthorSpec = serde_json::from_str(&payload)?;
            let case = compile_combat_author_case(&spec)?;
            write_case_to_path(&case, &out)?;
            println!("wrote {}", out.display());
        }
        Command::Materialize { case, out } => {
            let case = load_case_from_path(&case)?;
            let materialized = CombatCaseReducer::materialize(&case)?;
            write_case_to_path(&materialized, &out)?;
            println!("wrote {}", out.display());
        }
    }
    Ok(())
}
