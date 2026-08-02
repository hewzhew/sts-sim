use std::collections::BTreeMap;

use blake2::{Blake2b512, Digest};
use sts_oracle_runtime::ai::combat_state_key::combat_exact_state_hash_v2;
use sts_oracle_runtime::content::cards::{get_card_definition, CardId};
use sts_oracle_runtime::eval::combat_case::load_combat_case;
use sts_oracle_runtime::sim::combat::{
    combat_terminal, CombatPosition, CombatStepLimits, CombatStepper, CombatTerminal,
    EngineCombatStepper,
};
use sts_oracle_runtime::state::core::ClientInput;

use super::super::exact_turn_corridor::load_action_segments;
use super::{
    display_path, ActionObservation, CardObservation, EvidenceRecord, FiendFireObservation,
    ImmediateFiendFireObservation, MonsterObservation, PairCandidate, PlayerObservation,
    ReplayFrame, StateObservation, EVIDENCE_SCHEMA_NAME,
};

pub(super) fn replay_pair(
    candidate: &PairCandidate,
    max_engine_steps_per_transition: usize,
) -> Result<EvidenceRecord, String> {
    let loaded = load_combat_case(&candidate.case_path).map_err(|error| {
        format!(
            "cannot load candidate case '{}': {error}",
            candidate.case_path.display()
        )
    })?;
    let inputs = load_action_segments(&candidate.action_paths)?;
    let action_hash = action_sequence_hash(&inputs)?;
    let stepper = EngineCombatStepper;
    let mut position = loaded.position;
    let root_hash = combat_exact_state_hash_v2(&position.engine, &position.combat);
    let record_id = record_id(&root_hash, &action_hash);
    let supplied_action_count = inputs.len();
    let mut frames = Vec::with_capacity(inputs.len());

    for (index, input) in inputs.into_iter().enumerate() {
        if combat_terminal(&position.engine, &position.combat) != CombatTerminal::Unresolved {
            break;
        }
        let legal = stepper.atomic_actions(&position);
        let exact_input_is_legal = legal.iter().any(|candidate| candidate == &input)
            || stepper.choice_for_legal_input(&position, &input).is_some();
        if !exact_input_is_legal {
            return Err(format!(
                "pair replay rejected action {index} as illegal from exact state: {input:?}"
            ));
        }
        let before_position = position.clone();
        let before = snapshot(&position);
        let card = played_card(&position, &input);
        let card_type = card
            .as_ref()
            .map(|card| format!("{:?}", get_card_definition(card.id).card_type));
        let step = stepper.apply_to_stable(
            &position,
            input.clone(),
            CombatStepLimits {
                max_engine_steps: max_engine_steps_per_transition,
                deadline: None,
            },
        );
        if step.truncated || step.timed_out {
            return Err(format!(
                "pair replay transition {index} exceeded the exact transition limit"
            ));
        }
        let observation = ActionObservation {
            index,
            input,
            card,
            card_type,
            before,
            after: snapshot(&step.position),
            terminal_after: format!("{:?}", step.terminal),
        };
        position = step.position;
        frames.push(ReplayFrame {
            before_position,
            observation,
        });
    }

    let final_terminal = format!("{:?}", combat_terminal(&position.engine, &position.combat));
    let observations = frames
        .iter()
        .map(|frame| frame.observation.clone())
        .collect::<Vec<_>>();
    let fiend_fire_observations = fiend_fire_observations_from_replay(
        &record_id,
        &root_hash,
        &frames,
        &final_terminal,
        max_engine_steps_per_transition,
    );
    Ok(EvidenceRecord {
        schema_name: EVIDENCE_SCHEMA_NAME,
        schema_version: 1,
        record_id,
        root_exact_state_hash: root_hash,
        action_sequence_blake2b_512: action_hash,
        provenance: candidate.provenance.clone(),
        source_paths: candidate.source_paths.clone(),
        case_path: Some(display_path(&candidate.case_path)),
        action_paths: candidate
            .action_paths
            .iter()
            .map(|path| display_path(path))
            .collect(),
        replay_exact: true,
        supplied_action_count,
        consumed_action_count: observations.len(),
        final_terminal,
        final_player_hp: position.combat.entities.player.current_hp,
        actions: observations,
        fiend_fire_observations,
    })
}

fn snapshot(position: &CombatPosition) -> StateObservation {
    StateObservation {
        turn: position.combat.turn.turn_count,
        energy: i32::from(position.combat.turn.energy),
        player: PlayerObservation {
            hp: position.combat.entities.player.current_hp,
            block: position.combat.entities.player.block,
        },
        hand: position
            .combat
            .zones
            .hand
            .iter()
            .map(card_observation)
            .collect(),
        monsters: position
            .combat
            .entities
            .monsters
            .iter()
            .map(|monster| MonsterObservation {
                id: monster.id,
                hp: monster.current_hp,
                max_hp: monster.max_hp,
                block: monster.block,
                slot: monster.slot,
                is_dying: monster.is_dying,
                half_dead: monster.half_dead,
                is_escaped: monster.is_escaped,
            })
            .collect(),
    }
}

fn played_card(position: &CombatPosition, input: &ClientInput) -> Option<CardObservation> {
    let ClientInput::PlayCard { card_index, .. } = input else {
        return None;
    };
    position
        .combat
        .zones
        .hand
        .get(*card_index)
        .map(card_observation)
}

fn card_observation(card: &sts_oracle_runtime::runtime::combat::CombatCard) -> CardObservation {
    CardObservation {
        id: card.id,
        uuid: card.uuid,
        upgrades: card.upgrades,
        cost_for_turn: card.cost_for_turn,
        free_to_play_once: card.free_to_play_once,
    }
}

fn fiend_fire_observations_from_replay(
    record_id: &str,
    root_hash: &str,
    frames: &[ReplayFrame],
    final_terminal: &str,
    max_engine_steps_per_transition: usize,
) -> Vec<FiendFireObservation> {
    let mut observations = Vec::new();
    let actions = frames
        .iter()
        .map(|frame| frame.observation.clone())
        .collect::<Vec<_>>();
    for (index, frame) in frames.iter().enumerate() {
        if frame.observation.card.as_ref().map(|card| card.id) != Some(CardId::FiendFire) {
            continue;
        }
        let immediate = previous_card_index(&actions, index)
            .map(|previous_index| {
                immediate_fiend_fire_counterfactual(
                    &frames[previous_index].before_position,
                    &frame.observation,
                    max_engine_steps_per_transition,
                )
            })
            .unwrap_or_else(|| ImmediateFiendFireObservation {
                status: "no_previous_card_boundary".to_string(),
                target_after: None,
            });
        observations.push(build_fiend_fire_observation(
            record_id,
            root_hash,
            &actions,
            index,
            immediate,
            final_terminal,
        ));
    }
    observations
}

fn immediate_fiend_fire_counterfactual(
    previous_boundary: &CombatPosition,
    fiend_fire: &ActionObservation,
    max_engine_steps_per_transition: usize,
) -> ImmediateFiendFireObservation {
    let Some(card) = fiend_fire.card.as_ref() else {
        return ImmediateFiendFireObservation {
            status: "missing_fiend_fire_identity".to_string(),
            target_after: None,
        };
    };
    let ClientInput::PlayCard { target, .. } = fiend_fire.input else {
        return ImmediateFiendFireObservation {
            status: "missing_fiend_fire_input".to_string(),
            target_after: None,
        };
    };
    let Some(card_index) = previous_boundary
        .combat
        .zones
        .hand
        .iter()
        .position(|candidate| candidate.uuid == card.uuid)
    else {
        return ImmediateFiendFireObservation {
            status: "fiend_fire_not_in_previous_hand".to_string(),
            target_after: None,
        };
    };
    let input = ClientInput::PlayCard { card_index, target };
    let stepper = EngineCombatStepper;
    let legal = stepper.atomic_actions(previous_boundary);
    if !legal.iter().any(|candidate| candidate == &input)
        && stepper
            .choice_for_legal_input(previous_boundary, &input)
            .is_none()
    {
        return ImmediateFiendFireObservation {
            status: "not_legal_at_previous_boundary".to_string(),
            target_after: None,
        };
    }
    let step = stepper.apply_to_stable(
        previous_boundary,
        input,
        CombatStepLimits {
            max_engine_steps: max_engine_steps_per_transition,
            deadline: None,
        },
    );
    if step.truncated || step.timed_out {
        return ImmediateFiendFireObservation {
            status: "counterfactual_transition_limited".to_string(),
            target_after: None,
        };
    }
    let target_after = target.and_then(|target| snapshot(&step.position).monster(target).cloned());
    let status = if target_after
        .as_ref()
        .is_some_and(MonsterObservation::terminal_like)
    {
        "terminal_like"
    } else {
        "non_terminal"
    };
    ImmediateFiendFireObservation {
        status: status.to_string(),
        target_after,
    }
}

fn previous_card_index(actions: &[ActionObservation], index: usize) -> Option<usize> {
    let turn = actions.get(index)?.before.turn;
    for candidate_index in (0..index).rev() {
        let candidate = &actions[candidate_index];
        if candidate.before.turn != turn {
            break;
        }
        if candidate.card.is_some() {
            return Some(candidate_index);
        }
    }
    None
}

pub(super) fn build_fiend_fire_observation(
    record_id: &str,
    root_hash: &str,
    actions: &[ActionObservation],
    index: usize,
    immediate: ImmediateFiendFireObservation,
    final_terminal: &str,
) -> FiendFireObservation {
    let fiend_fire = &actions[index];
    let previous_index = previous_card_index(actions, index);
    let previous = previous_index.map(|previous_index| &actions[previous_index]);
    let target = match fiend_fire.input {
        ClientInput::PlayCard { target, .. } => target,
        _ => None,
    };
    let target_before_previous = previous
        .and_then(|previous| target.and_then(|target| previous.before.monster(target)))
        .cloned();
    let target_after_previous = previous
        .and_then(|previous| target.and_then(|target| previous.after.monster(target)))
        .cloned();
    let target_before_fiend_fire = target
        .and_then(|target| fiend_fire.before.monster(target))
        .cloned();
    let target_after_fiend_fire = target
        .and_then(|target| fiend_fire.after.monster(target))
        .cloned();
    let previous_card = previous.and_then(|previous| previous.card.as_ref().map(|card| card.id));
    let previous_card_type = previous.and_then(|previous| previous.card_type.clone());
    let classification = classify_fiend_fire(
        previous,
        target,
        target_before_previous.as_ref(),
        target_after_previous.as_ref(),
        target_after_fiend_fire.as_ref(),
        &immediate,
        final_terminal,
    );
    FiendFireObservation {
        record_id: record_id.to_string(),
        root_exact_state_hash: root_hash.to_string(),
        turn: fiend_fire.before.turn,
        previous_action_index: previous_index,
        fiend_fire_action_index: index,
        previous_card,
        previous_card_type,
        target_id: target,
        target_before_previous,
        target_after_previous,
        target_before_fiend_fire,
        target_after_fiend_fire,
        immediate_fiend_fire: immediate,
        full_line_terminal: final_terminal.to_string(),
        classification,
    }
}

fn classify_fiend_fire(
    previous: Option<&ActionObservation>,
    target: Option<usize>,
    target_before_previous: Option<&MonsterObservation>,
    target_after_previous: Option<&MonsterObservation>,
    target_after_fiend_fire: Option<&MonsterObservation>,
    immediate: &ImmediateFiendFireObservation,
    final_terminal: &str,
) -> String {
    let Some(previous) = previous else {
        return "no_previous_card".to_string();
    };
    if previous.card_type.as_deref() != Some("Attack") {
        return "previous_card_not_attack".to_string();
    }
    if target.is_none() {
        return "fiend_fire_has_no_target".to_string();
    }
    let Some(before) = target_before_previous else {
        return "missing_previous_target_state".to_string();
    };
    let Some(after) = target_after_previous else {
        return "missing_previous_target_state".to_string();
    };
    if before.block <= 0 {
        return "no_positive_block_before_previous_attack".to_string();
    }
    if after.block >= before.block {
        return "previous_attack_did_not_reduce_target_block".to_string();
    }
    if !target_after_fiend_fire.is_some_and(MonsterObservation::terminal_like) {
        return "fiend_fire_not_terminal_like".to_string();
    }
    match immediate.status.as_str() {
        "terminal_like" => "immediate_fiend_fire_already_terminal_like".to_string(),
        "non_terminal" if final_terminal == "Win" => {
            "confirmed_block_conversion_window".to_string()
        }
        "non_terminal" => "local_block_conversion_without_complete_win".to_string(),
        "unavailable_trace_only" => "observed_block_conversion_candidate".to_string(),
        _ => "block_conversion_counterfactual_unknown".to_string(),
    }
}

pub(super) fn action_sequence_hash(inputs: &[ClientInput]) -> Result<String, String> {
    let bytes = serde_json::to_vec(inputs).map_err(|error| error.to_string())?;
    let mut digest = Blake2b512::new();
    digest.update(bytes);
    Ok(digest
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect())
}

pub(super) fn record_id(root_hash: &str, action_hash: &str) -> String {
    format!(
        "{}:{}",
        root_hash.get(..12).unwrap_or(root_hash),
        action_hash.get(..12).unwrap_or(action_hash)
    )
}

pub(super) fn deduplicate_records(records: Vec<EvidenceRecord>) -> Vec<EvidenceRecord> {
    let mut by_identity = BTreeMap::<String, EvidenceRecord>::new();
    for mut record in records {
        let key = format!(
            "{}|{}",
            record.root_exact_state_hash, record.action_sequence_blake2b_512
        );
        if let Some(existing) = by_identity.get_mut(&key) {
            if record.replay_exact && !existing.replay_exact {
                record.provenance.extend(existing.provenance.clone());
                record.source_paths.extend(existing.source_paths.clone());
                *existing = record;
            } else {
                existing.provenance.extend(record.provenance);
                existing.source_paths.extend(record.source_paths);
            }
        } else {
            by_identity.insert(key, record);
        }
    }
    by_identity.into_values().collect()
}
