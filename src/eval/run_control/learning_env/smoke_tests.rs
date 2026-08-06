use super::*;
use crate::sim::combat_action_surface::{
    CombatLegalActionSurfaceV2, CombatSelectionActionFamilyV2, CombatSelectionDomainCandidateV2,
    CombatSelectionInputEncodingV2, CombatSelectionStatusV2,
};
use crate::state::selection::{SelectionResolution, SelectionScope};
use std::collections::HashSet;
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
        LearningBoundaryV1::Strategic { boundary } => {
            if !boundary.legal_candidates.completeness.is_complete() {
                return Err("incomplete strategic action surface".to_string());
            }
            let candidates = &boundary.legal_candidates.candidates;
            if candidates.is_empty() {
                return Err("complete strategic boundary has no candidate".to_string());
            }
            stats.strategic_steps += 1;
            Ok(LearningActionV1::StrategicCandidate {
                candidate_id: candidates[rng.pick(candidates.len())].candidate_id.clone(),
            })
        }
        LearningBoundaryV1::Combat { boundary } => {
            if boundary.observation_completeness != LearningObservationCompletenessV1::Complete {
                return Err("incomplete combat observation".to_string());
            }
            stats.combat_steps += 1;
            Ok(LearningActionV1::CombatInput {
                input: smoke_combat_input(&boundary.legal_actions, rng)?,
            })
        }
        LearningBoundaryV1::Terminal { .. } => {
            Err("terminal boundary was not marked terminated by the prior step".to_string())
        }
        LearningBoundaryV1::Unsupported => Err("unsupported learning boundary".to_string()),
    }
}

fn smoke_combat_input(
    surface: &CombatLegalActionSurfaceV2,
    rng: &mut SmokeRng,
) -> Result<ClientInput, String> {
    let enabled_families = surface
        .selection_families
        .iter()
        .filter(|family| family.selection_status == CombatSelectionStatusV2::Enabled)
        .collect::<Vec<_>>();
    let choice_count = surface.atomic_actions.len() + enabled_families.len();
    if choice_count == 0 {
        return Err("complete combat action surface has no legal action".to_string());
    }
    let choice = rng.pick(choice_count);
    if let Some(input) = surface.atomic_actions.get(choice) {
        return Ok(input.clone());
    }
    smoke_selection_input(enabled_families[choice - surface.atomic_actions.len()], rng)
}

fn smoke_selection_input(
    family: &CombatSelectionActionFamilyV2,
    rng: &mut SmokeRng,
) -> Result<ClientInput, String> {
    let required = usize::try_from(family.declared_min)
        .map_err(|_| "selection minimum does not fit usize".to_string())?;
    match family.input_encoding {
        CombatSelectionInputEncodingV2::SubmitSelectionHandCardUuids
        | CombatSelectionInputEncodingV2::SubmitSelectionGridCardUuids => {
            let mut eligible = family
                .raw_domain
                .iter()
                .filter_map(|candidate| match candidate {
                    CombatSelectionDomainCandidateV2::CardUuid {
                        uuid,
                        eligible: true,
                        ..
                    } => Some(*uuid),
                    _ => None,
                })
                .collect::<Vec<_>>();
            let selected = pick_distinct(&mut eligible, required, rng)?;
            let scope = match family.input_encoding {
                CombatSelectionInputEncodingV2::SubmitSelectionHandCardUuids => {
                    SelectionScope::Hand
                }
                CombatSelectionInputEncodingV2::SubmitSelectionGridCardUuids => {
                    SelectionScope::Grid
                }
                CombatSelectionInputEncodingV2::SubmitScryDiscardIndices => unreachable!(),
            };
            Ok(ClientInput::SubmitSelection(
                SelectionResolution::card_uuids(scope, selected),
            ))
        }
        CombatSelectionInputEncodingV2::SubmitScryDiscardIndices => {
            let mut seen_uuids = HashSet::new();
            let mut eligible = family
                .raw_domain
                .iter()
                .filter_map(|candidate| match candidate {
                    CombatSelectionDomainCandidateV2::ScryIndex {
                        index,
                        card_uuid: Some(uuid),
                        currently_present: true,
                        ..
                    } if seen_uuids.insert(*uuid) => usize::try_from(*index).ok(),
                    _ => None,
                })
                .collect::<Vec<_>>();
            Ok(ClientInput::SubmitScryDiscard(pick_distinct(
                &mut eligible,
                required,
                rng,
            )?))
        }
    }
}

fn pick_distinct<T: Copy>(
    candidates: &mut Vec<T>,
    amount: usize,
    rng: &mut SmokeRng,
) -> Result<Vec<T>, String> {
    if amount > candidates.len() {
        return Err(format!(
            "selection requires {amount} values but only {} are eligible",
            candidates.len()
        ));
    }
    let mut selected = Vec::with_capacity(amount);
    for _ in 0..amount {
        let index = rng.pick(candidates.len());
        selected.push(candidates.swap_remove(index));
    }
    Ok(selected)
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
