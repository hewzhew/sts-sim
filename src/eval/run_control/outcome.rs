use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};
use sts_combat_planner::{
    CombatOutcomeDatasetErrorV1, CombatOutcomeFeatureVectorV1, CombatOutcomeLabelProvenanceV1,
    CombatOutcomeTrainingBatchV1, CombatOutcomeTrainingCaseV1, CombatOutcomeTrainingExampleV1,
};

use crate::engine::run_loop::FinishedActiveCombat;
use crate::runtime::combat::CombatState;
use crate::sim::combat::{combat_terminal, CombatTerminal};
use crate::state::core::ClientInput;

pub const COMBAT_BASELINE_OUTCOME_SCHEMA_NAME: &str = "CombatBaselineOutcomeV1";
pub const COMBAT_BASELINE_OUTCOME_SCHEMA_VERSION: u32 = 1;

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CombatBaselineOutcomeV1 {
    pub schema_name: String,
    pub schema_version: u32,
    pub case_id: String,
    pub terminal: CombatTerminal,
    pub start_hp: i32,
    pub final_hp: i32,
    pub hp_loss: i32,
    pub turns: u32,
    pub potions_used: u32,
    pub potions_discarded: u32,
    pub cards_played: u32,
}

impl CombatBaselineOutcomeV1 {
    pub fn terminal(&self) -> CombatTerminal {
        self.terminal
    }
}

#[derive(Clone, Debug, Default, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct CombatOutcomeTracker {
    active: Option<CombatOutcomeDraft>,
    last: Option<CombatBaselineOutcomeV1>,
    #[serde(default)]
    last_training_examples: Vec<CombatOutcomeTrainingExampleV1>,
    #[serde(default)]
    training_cases: Vec<CombatOutcomeTrainingCaseV1>,
}

#[derive(Clone, Debug, PartialEq, serde::Deserialize, serde::Serialize)]
struct CombatOutcomeDraft {
    start_hp: i32,
    potions_used: u32,
    potions_discarded: u32,
    observed_turns: Vec<CombatOutcomeFeatureVectorV1>,
    last_observed_turn: Option<u32>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PendingPotionObservation {
    kind: PotionObservationKind,
    uuid: u32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum PotionObservationKind {
    Use,
    Discard,
}

impl CombatOutcomeTracker {
    pub fn ensure_started(&mut self, combat: Option<&CombatState>) -> bool {
        if self.active.is_some() {
            return false;
        }
        let Some(combat) = combat else {
            return false;
        };
        self.active = Some(CombatOutcomeDraft {
            start_hp: combat.entities.player.current_hp,
            potions_used: 0,
            potions_discarded: 0,
            observed_turns: vec![CombatOutcomeFeatureVectorV1::from_combat(combat)],
            last_observed_turn: Some(combat.turn.turn_count),
        });
        true
    }

    pub fn observe_player_turn_boundary(&mut self, combat: Option<&CombatState>) {
        let Some(combat) = combat else {
            return;
        };
        let Some(active) = self.active.as_mut() else {
            return;
        };
        if active.last_observed_turn == Some(combat.turn.turn_count) {
            return;
        }
        active
            .observed_turns
            .push(CombatOutcomeFeatureVectorV1::from_combat(combat));
        active.last_observed_turn = Some(combat.turn.turn_count);
    }

    pub fn observe_input_before(
        &self,
        combat: Option<&CombatState>,
        input: &ClientInput,
    ) -> Option<PendingPotionObservation> {
        let combat = combat?;
        match input {
            ClientInput::UsePotion { potion_index, .. } => combat
                .entities
                .potions
                .get(*potion_index)
                .and_then(|slot| slot.as_ref())
                .map(|potion| PendingPotionObservation {
                    kind: PotionObservationKind::Use,
                    uuid: potion.uuid,
                }),
            ClientInput::DiscardPotion(slot) => combat
                .entities
                .potions
                .get(*slot)
                .and_then(|slot| slot.as_ref())
                .map(|potion| PendingPotionObservation {
                    kind: PotionObservationKind::Discard,
                    uuid: potion.uuid,
                }),
            _ => None,
        }
    }

    pub fn observe_input_after(
        &mut self,
        observation: Option<PendingPotionObservation>,
        combat: Option<&CombatState>,
    ) {
        let Some(observation) = observation else {
            return;
        };
        let Some(combat) = combat else {
            return;
        };
        if potion_uuid_exists(combat, observation.uuid) {
            return;
        }
        let Some(active) = self.active.as_mut() else {
            return;
        };
        match observation.kind {
            PotionObservationKind::Use => {
                active.potions_used = active.potions_used.saturating_add(1);
            }
            PotionObservationKind::Discard => {
                active.potions_discarded = active.potions_discarded.saturating_add(1);
            }
        }
    }

    pub fn finish(
        &mut self,
        case_id: impl Into<String>,
        split_group_id: impl Into<String>,
        finished: &FinishedActiveCombat,
        continuation_policy_manifest: impl Into<String>,
    ) -> CombatBaselineOutcomeV1 {
        let draft = self.active.take().unwrap_or(CombatOutcomeDraft {
            start_hp: finished.combat_state.entities.player.current_hp,
            potions_used: 0,
            potions_discarded: 0,
            observed_turns: Vec::new(),
            last_observed_turn: None,
        });
        let final_hp = finished.combat_state.entities.player.current_hp;
        let case_id = case_id.into();
        let outcome = CombatBaselineOutcomeV1 {
            schema_name: COMBAT_BASELINE_OUTCOME_SCHEMA_NAME.to_string(),
            schema_version: COMBAT_BASELINE_OUTCOME_SCHEMA_VERSION,
            case_id: case_id.clone(),
            terminal: combat_terminal(&finished.engine_state, &finished.combat_state),
            start_hp: draft.start_hp,
            final_hp,
            hp_loss: (draft.start_hp - final_hp).max(0),
            turns: finished.combat_state.turn.turn_count,
            potions_used: draft.potions_used,
            potions_discarded: draft.potions_discarded,
            cards_played: finished
                .combat_state
                .turn
                .counters
                .card_ids_played_this_combat
                .len() as u32,
        };
        let continuation_policy_manifest = continuation_policy_manifest.into();
        self.last_training_examples = match outcome.terminal {
            CombatTerminal::Win | CombatTerminal::Loss => draft
                .observed_turns
                .into_iter()
                .map(|features| CombatOutcomeTrainingExampleV1 {
                    features,
                    victory: outcome.terminal == CombatTerminal::Win,
                    terminal_hp_fraction: f64::from(final_hp.max(0))
                        / f64::from(finished.combat_state.entities.player.max_hp.max(1)),
                    provenance: CombatOutcomeLabelProvenanceV1::RealizedBehaviorCombat,
                    continuation_policy_manifest: continuation_policy_manifest.clone(),
                })
                .collect(),
            CombatTerminal::Unresolved => Vec::new(),
        };
        if !self.last_training_examples.is_empty() {
            self.training_cases.push(CombatOutcomeTrainingCaseV1 {
                case_id,
                split_group_id: split_group_id.into(),
                examples: self.last_training_examples.clone(),
            });
        }
        self.last = Some(outcome.clone());
        outcome
    }

    pub fn last(&self) -> Option<&CombatBaselineOutcomeV1> {
        self.last.as_ref()
    }

    pub fn last_training_examples(&self) -> &[CombatOutcomeTrainingExampleV1] {
        &self.last_training_examples
    }

    pub fn training_batches(
        &self,
    ) -> Result<Vec<CombatOutcomeTrainingBatchV1>, CombatOutcomeDatasetErrorV1> {
        let mut cases_by_policy = std::collections::BTreeMap::<String, Vec<_>>::new();
        for case in &self.training_cases {
            let Some(example) = case.examples.first() else {
                continue;
            };
            cases_by_policy
                .entry(example.continuation_policy_manifest.clone())
                .or_default()
                .push(case.clone());
        }
        cases_by_policy
            .into_iter()
            .map(|(policy, cases)| CombatOutcomeTrainingBatchV1::new(policy, cases))
            .collect()
    }
}

pub fn load_combat_baseline_outcome_v1(path: &Path) -> Result<CombatBaselineOutcomeV1, String> {
    let payload = fs::read_to_string(path).map_err(|err| err.to_string())?;
    let baseline: CombatBaselineOutcomeV1 =
        serde_json::from_str(&payload).map_err(|err| err.to_string())?;
    validate_combat_baseline_outcome_v1(&baseline)?;
    Ok(baseline)
}

pub fn save_combat_baseline_outcome_v1(
    path: &Path,
    baseline: &CombatBaselineOutcomeV1,
) -> Result<(), String> {
    validate_combat_baseline_outcome_v1(baseline)?;
    if let Some(parent) = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        fs::create_dir_all(parent).map_err(|err| err.to_string())?;
    }
    let payload = serde_json::to_string_pretty(baseline).map_err(|err| err.to_string())?;
    fs::write(path, payload).map_err(|err| err.to_string())
}

pub fn validate_combat_baseline_outcome_v1(
    baseline: &CombatBaselineOutcomeV1,
) -> Result<(), String> {
    if baseline.schema_name != COMBAT_BASELINE_OUTCOME_SCHEMA_NAME {
        return Err(format!(
            "unsupported combat baseline schema '{}'",
            baseline.schema_name
        ));
    }
    if baseline.schema_version != COMBAT_BASELINE_OUTCOME_SCHEMA_VERSION {
        return Err(format!(
            "unsupported combat baseline schema_version {}",
            baseline.schema_version
        ));
    }
    if baseline.case_id.trim().is_empty() {
        return Err("combat baseline case_id cannot be empty".to_string());
    }
    if baseline.hp_loss != (baseline.start_hp - baseline.final_hp).max(0) {
        return Err("combat baseline hp_loss does not match start/final hp".to_string());
    }
    Ok(())
}

fn potion_uuid_exists(combat: &CombatState, uuid: u32) -> bool {
    combat
        .entities
        .potions
        .iter()
        .any(|slot| slot.as_ref().is_some_and(|potion| potion.uuid == uuid))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixtures::combat_start_spec::{compile_combat_start_spec, CombatStartSpec};

    #[test]
    fn baseline_validation_rejects_bad_hp_loss() {
        let mut baseline = sample_baseline();
        baseline.hp_loss += 1;

        let err = validate_combat_baseline_outcome_v1(&baseline)
            .expect_err("bad hp_loss should be rejected");

        assert!(err.contains("hp_loss"));
    }

    #[test]
    fn tracker_builds_outcome_from_finished_combat() {
        let spec: CombatStartSpec = serde_json::from_str(
            r#"{
                "name": "jaw_worm_starter",
                "player_class": "Ironclad",
                "ascension_level": 0,
                "encounter_id": "JawWorm",
                "room_type": "monster",
                "seed": 1,
                "player_current_hp": 80,
                "player_max_hp": 80,
                "master_deck": [
                    {"id": "Strike_R", "count": 5},
                    {"id": "Defend_R", "count": 4},
                    "Bash"
                ]
            }"#,
        )
        .expect("spec parses");
        let (_engine_state, mut combat_state) =
            compile_combat_start_spec(&spec).expect("spec compiles");
        combat_state.entities.player.current_hp = 72;
        combat_state.turn.counters.card_ids_played_this_combat =
            vec![crate::content::cards::CardId::Strike];

        let mut tracker = CombatOutcomeTracker::default();
        tracker.ensure_started(Some(&combat_state));
        combat_state.turn.turn_count = combat_state.turn.turn_count.saturating_add(1);
        tracker.observe_player_turn_boundary(Some(&combat_state));
        combat_state.entities.player.current_hp = 65;
        for monster in &mut combat_state.entities.monsters {
            monster.current_hp = 0;
        }
        let finished = FinishedActiveCombat {
            engine_state: crate::state::core::EngineState::GameOver(
                crate::state::core::RunResult::Victory,
            ),
            combat_state,
        };
        let outcome = tracker.finish("jaw", "test-run-root", &finished, "test-realized-policy-v1");

        assert_eq!(outcome.start_hp, 72);
        assert_eq!(outcome.final_hp, 65);
        assert_eq!(outcome.hp_loss, 7);
        assert_eq!(outcome.cards_played, 1);
        assert_eq!(tracker.last_training_examples().len(), 2);
        assert!(tracker
            .last_training_examples()
            .iter()
            .all(|example| example.continuation_policy_manifest == "test-realized-policy-v1"));
        let batches = tracker.training_batches().expect("training batch is valid");
        assert_eq!(batches.len(), 1);
        assert_eq!(batches[0].cases[0].case_id, "jaw");
        assert_eq!(batches[0].cases[0].split_group_id, "test-run-root");
    }

    fn sample_baseline() -> CombatBaselineOutcomeV1 {
        CombatBaselineOutcomeV1 {
            schema_name: COMBAT_BASELINE_OUTCOME_SCHEMA_NAME.to_string(),
            schema_version: COMBAT_BASELINE_OUTCOME_SCHEMA_VERSION,
            case_id: "case".to_string(),
            terminal: CombatTerminal::Win,
            start_hp: 80,
            final_hp: 70,
            hp_loss: 10,
            turns: 4,
            potions_used: 0,
            potions_discarded: 0,
            cards_played: 8,
        }
    }
}
