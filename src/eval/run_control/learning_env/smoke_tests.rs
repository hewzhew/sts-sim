use super::*;
use crate::eval::run_control::{
    LearningModelChoiceV1, LearningModelDecisionV1, LearningSelectionStepV1,
};
use std::time::{Duration, Instant};

const EPISODE_STEP_LIMIT: usize = 20_000;

#[derive(Default)]
struct SmokeStats {
    total_steps: usize,
    strategic_steps: usize,
    combat_steps: usize,
    victories: usize,
}

struct SmokeRng(u64);

impl SmokeRng {
    fn pick(&mut self, len: usize) -> usize {
        debug_assert!(len > 0);
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        ((self.0 >> 32) as usize) % len
    }
}

#[test]
fn seeded_legal_action_walk_reaches_terminal_without_unsupported_boundary() {
    let started = Instant::now();
    let mut stats = SmokeStats::default();
    let seeds = 1..=5;

    for seed in seeds.clone() {
        run_smoke_episode(seed, &mut stats).unwrap_or_else(|error| panic!("seed {seed}: {error}"));
    }

    print_summary(seeds.count(), &stats, started.elapsed());
}

fn run_smoke_episode(seed: u64, stats: &mut SmokeStats) -> Result<(), String> {
    let mut env = LearningEnvV1::new(RunControlConfig {
        seed,
        ..RunControlConfig::default()
    });
    let mut rng = SmokeRng(seed ^ 0x9e37_79b9_7f4a_7c15);
    let mut boundary = env.observe()?;

    for episode_step in 0..EPISODE_STEP_LIMIT {
        let action = smoke_action(&boundary, &mut rng, stats)
            .map_err(|error| format!("step {episode_step}: {error}"))?;
        let step = env
            .step(action)
            .map_err(|error| format!("step {episode_step}: {error}"))?;
        stats.total_steps += 1;
        boundary = step.boundary;
        if step.terminated {
            stats.victories += usize::from(step.reward > 0);
            return Ok(());
        }
    }

    Err(format!(
        "legal action walk exceeded the {EPISODE_STEP_LIMIT}-step cap"
    ))
}

fn smoke_action(
    boundary: &LearningBoundaryV1,
    rng: &mut SmokeRng,
    stats: &mut SmokeStats,
) -> Result<LearningActionV1, String> {
    match boundary {
        LearningBoundaryV1::Strategic { .. } => stats.strategic_steps += 1,
        LearningBoundaryV1::Combat { .. } => stats.combat_steps += 1,
        LearningBoundaryV1::Terminal { .. } => {
            return Err("terminal boundary was not marked terminated by the prior step".to_string())
        }
        LearningBoundaryV1::Unsupported => {
            return Err("unsupported learning boundary".to_string());
        }
    }
    let decision = LearningModelDecisionV1::from_boundary(boundary)
        .map_err(|error| format!("model input: {error}"))?;
    match decision
        .choose(rng.pick(decision.candidates.len()))
        .map_err(|error| format!("root choice: {error}"))?
    {
        LearningModelChoiceV1::Apply(action) => Ok(action),
        LearningModelChoiceV1::DecodeSelection(mut draft) => loop {
            let selection = draft.decision();
            match draft
                .choose(rng.pick(selection.candidates.len()))
                .map_err(|error| format!("selection choice: {error}"))?
            {
                LearningSelectionStepV1::Continue => {}
                LearningSelectionStepV1::Apply(action) => break Ok(action),
            }
        },
    }
}

fn print_summary(episodes: usize, stats: &SmokeStats, elapsed: Duration) {
    eprintln!(
        "learning_env_smoke episodes={} steps={} strategic_steps={} combat_steps={} victories={} elapsed_ms={} steps_per_second={:.0}",
        episodes,
        stats.total_steps,
        stats.strategic_steps,
        stats.combat_steps,
        stats.victories,
        elapsed.as_millis(),
        stats.total_steps as f64 / elapsed.as_secs_f64()
    );
}
