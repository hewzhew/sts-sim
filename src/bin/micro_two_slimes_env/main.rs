use std::io::{self, BufRead, Write};

use sts_simulator::ai::micro_two_slimes::{MicroRequest, MicroTwoSlimesEnv};

fn main() {
    let stdin = io::stdin();
    let mut stdout = io::stdout();
    let mut env = MicroTwoSlimesEnv::new(1);

    for line in stdin.lock().lines() {
        let line = match line {
            Ok(line) => line,
            Err(error) => {
                writeln!(
                    stdout,
                    "{{\"error\":\"stdin_read_failed\",\"detail\":{}}}",
                    serde_json::to_string(&error.to_string()).unwrap()
                )
                .unwrap();
                break;
            }
        };
        if line.trim().is_empty() {
            continue;
        }

        let response = match serde_json::from_str::<MicroRequest>(&line) {
            Ok(MicroRequest::Reset { seed }) => {
                serde_json::to_string(&env.reset(seed.unwrap_or(1))).unwrap()
            }
            Ok(MicroRequest::Step { action }) => serde_json::to_string(&env.step(action)).unwrap(),
            Ok(MicroRequest::Close) => break,
            Err(error) => serde_json::json!({
                "error": "bad_request",
                "detail": error.to_string()
            })
            .to_string(),
        };

        writeln!(stdout, "{response}").unwrap();
        stdout.flush().unwrap();
    }
}
