use crate::content::monsters::factory::EncounterId;
use crate::engine::campfire_candidates::CampfireCandidate;
use crate::eval::campfire_evaluation::CampfireEvaluationBatch;
use crate::eval::campfire_projection::CampfireProjection;
use crate::eval::combat_lab_v1::{derive_shuffle_seed_v1, CombatLabShuffleScheduleV1};
use crate::eval::fingerprint::{combat_state_fingerprint_v1, StateFingerprintV1};
use crate::runtime::combat::CombatCard;
use crate::runtime::rng::StsRng;
use crate::sim::combat::CombatPosition;
use crate::sim::combat_start::build_natural_combat_start;
use crate::state::map::node::RoomType;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CampfireSurvivalInformationScope {
    ExactStateOracle,
}

#[derive(Clone, Debug)]
pub struct CampfireSurvivalScenarioSpec {
    pub encounter_id: EncounterId,
    pub room_type: RoomType,
    pub schedule: CombatLabShuffleScheduleV1,
    pub sample_index: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CampfireSurvivalScenarioGap {
    ChanceOutcomeNotMaterialized,
    PostRevealRecourseNotMaterialized,
    DeckIdentityChanged,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CampfireSurvivalScenarioGapRecord {
    pub candidate: CampfireCandidate,
    pub gap: CampfireSurvivalScenarioGap,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CampfireSurvivalScenarioCell {
    pub candidate: CampfireCandidate,
    pub shuffle_seed: u64,
    pub start: CombatPosition,
    pub state_fingerprint: StateFingerprintV1,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CampfireSurvivalScenarioSample {
    pub context_fingerprint: String,
    pub information_scope: CampfireSurvivalInformationScope,
    pub encounter_id: EncounterId,
    pub room_type: RoomType,
    pub sample_index: u64,
    pub shuffle_seed: u64,
    pub cells: Vec<CampfireSurvivalScenarioCell>,
    pub gaps: Vec<CampfireSurvivalScenarioGapRecord>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CampfireSurvivalScenarioError {
    NonCombatRoomType {
        room_type: RoomType,
    },
    CombatStart {
        candidate: CampfireCandidate,
        message: String,
    },
}

pub fn compile_aligned_campfire_survival_sample(
    evaluation: &CampfireEvaluationBatch,
    spec: CampfireSurvivalScenarioSpec,
) -> Result<CampfireSurvivalScenarioSample, CampfireSurvivalScenarioError> {
    if !matches!(
        spec.room_type,
        RoomType::MonsterRoom | RoomType::MonsterRoomElite | RoomType::MonsterRoomBoss
    ) {
        return Err(CampfireSurvivalScenarioError::NonCombatRoomType {
            room_type: spec.room_type,
        });
    }

    let shuffle_seed = derive_shuffle_seed_v1(&spec.schedule, spec.sample_index);
    let root_deck = &evaluation.context.public_root.master_deck;
    let mut cells = Vec::new();
    let mut gaps = Vec::new();

    for candidate in &evaluation.candidates {
        let exact = match &candidate.projection {
            CampfireProjection::Exact(exact) => exact,
            CampfireProjection::Chance(_) => {
                gaps.push(CampfireSurvivalScenarioGapRecord {
                    candidate: candidate.candidate,
                    gap: CampfireSurvivalScenarioGap::ChanceOutcomeNotMaterialized,
                });
                continue;
            }
            CampfireProjection::ChanceThenDecision(_) => {
                gaps.push(CampfireSurvivalScenarioGapRecord {
                    candidate: candidate.candidate,
                    gap: CampfireSurvivalScenarioGap::PostRevealRecourseNotMaterialized,
                });
                continue;
            }
        };
        if !same_deck_identity(root_deck, &exact.run_state.master_deck) {
            gaps.push(CampfireSurvivalScenarioGapRecord {
                candidate: candidate.candidate,
                gap: CampfireSurvivalScenarioGap::DeckIdentityChanged,
            });
            continue;
        }

        let mut projected = exact.run_state.clone();
        projected.rng_pool.shuffle_rng = StsRng::new(shuffle_seed);
        let (engine, combat) =
            build_natural_combat_start(&mut projected, spec.encounter_id, spec.room_type).map_err(
                |message| CampfireSurvivalScenarioError::CombatStart {
                    candidate: candidate.candidate,
                    message,
                },
            )?;
        let start = CombatPosition::new(engine, combat);
        cells.push(CampfireSurvivalScenarioCell {
            candidate: candidate.candidate,
            shuffle_seed,
            state_fingerprint: combat_state_fingerprint_v1(&start),
            start,
        });
    }

    Ok(CampfireSurvivalScenarioSample {
        context_fingerprint: evaluation.context.context_fingerprint.clone(),
        information_scope: CampfireSurvivalInformationScope::ExactStateOracle,
        encounter_id: spec.encounter_id,
        room_type: spec.room_type,
        sample_index: spec.sample_index,
        shuffle_seed,
        cells,
        gaps,
    })
}

fn same_deck_identity(root: &[CombatCard], projected: &[CombatCard]) -> bool {
    root.iter()
        .map(|card| card.uuid)
        .eq(projected.iter().map(|card| card.uuid))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::cards::CardId;
    use crate::content::monsters::factory::EncounterId;
    use crate::content::relics::{RelicId, RelicState};
    use crate::engine::campfire_candidates::CampfireCandidate;
    use crate::eval::campfire_evaluation::{
        build_campfire_evaluation_batch, CampfireContinuationProfile, CampfireEvaluationHorizon,
        CampfireEvaluationSpec, CampfireRunGoal,
    };
    use crate::eval::combat_lab_v1::{CombatLabShuffleGeneratorV1, CombatLabShuffleScheduleV1};
    use crate::runtime::combat::CombatCard;
    use crate::state::map::node::RoomType;
    use crate::state::run::RunState;

    fn evaluation_spec() -> CampfireEvaluationSpec {
        CampfireEvaluationSpec {
            run_goal: CampfireRunGoal::Act3Victory,
            horizon: CampfireEvaluationHorizon::UntilNextCampfireOrActTerminal {
                route_horizon_nodes: 5,
            },
            route_path_budget: 2_000,
            continuation_profile: CampfireContinuationProfile {
                profile_id: "survival-scenario-test".to_string(),
                source_identity: "test-source".to_string(),
            },
            public_scenario_distribution_id: "explicit-encounter-test".to_string(),
            mechanics_version: "sts-simulator-test-v1".to_string(),
        }
    }

    fn candidate_run() -> RunState {
        let mut run = RunState::new(17, 0, false, "Ironclad");
        run.current_hp = 20;
        run.master_deck = vec![
            CombatCard::new(CardId::Strike, 101),
            CombatCard::new(CardId::Defend, 102),
        ];
        run.relics = vec![
            RelicState::new(RelicId::Girya),
            RelicState::new(RelicId::Shovel),
            RelicState::new(RelicId::PeacePipe),
        ];
        run
    }

    fn scenario_spec() -> CampfireSurvivalScenarioSpec {
        CampfireSurvivalScenarioSpec {
            encounter_id: EncounterId::JawWorm,
            room_type: RoomType::MonsterRoom,
            schedule: CombatLabShuffleScheduleV1 {
                generator: CombatLabShuffleGeneratorV1::SplitMix64V1,
                seed: 91,
            },
            sample_index: 0,
        }
    }

    #[test]
    fn exact_rest_and_smith_share_one_aligned_natural_combat_scenario() {
        let evaluation =
            build_campfire_evaluation_batch(&candidate_run(), evaluation_spec()).unwrap();
        let sample =
            compile_aligned_campfire_survival_sample(&evaluation, scenario_spec()).unwrap();
        let rest = sample
            .cells
            .iter()
            .find(|cell| cell.candidate == CampfireCandidate::Rest)
            .expect("exact Rest should be compiled");
        let smith = sample
            .cells
            .iter()
            .find(|cell| cell.candidate == CampfireCandidate::Smith { card_uuid: 101 })
            .expect("UUID-preserving Smith should be compiled");

        assert_eq!(
            sample.information_scope,
            CampfireSurvivalInformationScope::ExactStateOracle
        );
        assert_eq!(rest.start.combat.entities.player.current_hp, 44);
        assert_eq!(smith.start.combat.entities.player.current_hp, 20);
        assert_eq!(
            rest.start.combat.entities.monsters,
            smith.start.combat.entities.monsters
        );
        assert_eq!(
            rest.start
                .combat
                .zones
                .hand
                .iter()
                .map(|card| card.uuid)
                .collect::<Vec<_>>(),
            smith
                .start
                .combat
                .zones
                .hand
                .iter()
                .map(|card| card.uuid)
                .collect::<Vec<_>>()
        );
        assert!(sample.gaps.iter().any(|gap| {
            gap.gap == CampfireSurvivalScenarioGap::ChanceOutcomeNotMaterialized
                && gap.candidate == CampfireCandidate::Dig
        }));
        assert!(sample.gaps.iter().any(|gap| {
            gap.gap == CampfireSurvivalScenarioGap::DeckIdentityChanged
                && matches!(gap.candidate, CampfireCandidate::Toke { .. })
        }));
    }

    #[test]
    fn dream_catcher_rest_stays_a_post_reveal_recourse_gap() {
        let mut root = candidate_run();
        root.relics.push(RelicState::new(RelicId::DreamCatcher));
        let evaluation = build_campfire_evaluation_batch(&root, evaluation_spec()).unwrap();
        let sample =
            compile_aligned_campfire_survival_sample(&evaluation, scenario_spec()).unwrap();

        assert!(!sample
            .cells
            .iter()
            .any(|cell| cell.candidate == CampfireCandidate::Rest));
        assert!(sample.gaps.iter().any(|gap| {
            gap.candidate == CampfireCandidate::Rest
                && gap.gap == CampfireSurvivalScenarioGap::PostRevealRecourseNotMaterialized
        }));
    }
}
