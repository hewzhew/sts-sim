use std::collections::BTreeSet;
use std::path::PathBuf;

use sts_oracle_runtime::content::cards::CardId;
use sts_oracle_runtime::state::core::ClientInput;

use super::artifacts::is_trace_artifact_name;
use super::replay::{build_fiend_fire_observation, replay_pair};
use super::{
    ActionObservation, CardObservation, ImmediateFiendFireObservation, MonsterObservation,
    PairCandidate, PlayerObservation, StateObservation,
};

fn monster(id: usize, hp: i32, block: i32, terminal: bool) -> MonsterObservation {
    MonsterObservation {
        id,
        hp,
        max_hp: hp.max(1),
        block,
        slot: id.saturating_sub(1) as u8,
        is_dying: terminal,
        half_dead: false,
        is_escaped: false,
    }
}

fn state(turn: u32, target: MonsterObservation) -> StateObservation {
    StateObservation {
        turn,
        energy: 3,
        player: PlayerObservation { hp: 80, block: 0 },
        hand: Vec::new(),
        monsters: vec![target],
    }
}

fn card(id: CardId, uuid: u32) -> CardObservation {
    CardObservation {
        id,
        uuid,
        upgrades: 0,
        cost_for_turn: None,
        free_to_play_once: false,
    }
}

#[test]
fn a3f37_shape_requires_nonlethal_immediate_fiend_fire() {
    let actions = vec![
        ActionObservation {
            index: 0,
            input: ClientInput::PlayCard {
                card_index: 0,
                target: None,
            },
            card: Some(card(CardId::SwordBoomerang, 10)),
            card_type: Some("Attack".to_string()),
            before: state(2, monster(3, 20, 27, false)),
            after: state(2, monster(3, 20, 21, false)),
            terminal_after: "Unresolved".to_string(),
        },
        ActionObservation {
            index: 1,
            input: ClientInput::PlayCard {
                card_index: 0,
                target: Some(3),
            },
            card: Some(card(CardId::FiendFire, 11)),
            card_type: Some("Attack".to_string()),
            before: state(2, monster(3, 20, 21, false)),
            after: state(2, monster(3, 0, 0, true)),
            terminal_after: "Unresolved".to_string(),
        },
    ];
    let observation = build_fiend_fire_observation(
        "record",
        "root",
        &actions,
        1,
        ImmediateFiendFireObservation {
            status: "non_terminal".to_string(),
            target_after: Some(monster(3, 4, 0, false)),
        },
        "Win",
    );
    assert_eq!(
        observation.classification,
        "confirmed_block_conversion_window"
    );
}

#[test]
fn bronze_preparation_without_positive_target_block_stays_out_of_scope() {
    let actions = vec![
        ActionObservation {
            index: 0,
            input: ClientInput::PlayCard {
                card_index: 0,
                target: Some(2),
            },
            card: Some(card(CardId::Clothesline, 20)),
            card_type: Some("Attack".to_string()),
            before: state(1, monster(3, 57, 0, false)),
            after: state(1, monster(3, 57, 0, false)),
            terminal_after: "Unresolved".to_string(),
        },
        ActionObservation {
            index: 1,
            input: ClientInput::PlayCard {
                card_index: 0,
                target: Some(3),
            },
            card: Some(card(CardId::FiendFire, 21)),
            card_type: Some("Attack".to_string()),
            before: state(1, monster(3, 57, 0, false)),
            after: state(1, monster(3, 1, 0, false)),
            terminal_after: "Unresolved".to_string(),
        },
    ];
    let observation = build_fiend_fire_observation(
        "record",
        "root",
        &actions,
        1,
        ImmediateFiendFireObservation {
            status: "non_terminal".to_string(),
            target_after: Some(monster(3, 9, 0, false)),
        },
        "Win",
    );
    assert_eq!(
        observation.classification,
        "no_positive_block_before_previous_attack"
    );
}

#[test]
fn trace_suffix_discovery_includes_plan_trace_names() {
    assert!(is_trace_artifact_name("combined.plan-trace.json"));
    assert!(is_trace_artifact_name("witness.trace.json"));
    assert!(!is_trace_artifact_name("summary.json"));
}

#[test]
fn tracked_slime_boss_pair_replays_through_typed_timeline() {
    let repository = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let fixture_root = repository.join("fixtures/oracle_witnesses");
    let candidate = PairCandidate {
        case_path: fixture_root.join("seed20260713009_a0_slime_boss.combat-case.json"),
        action_paths: vec![
            fixture_root.join("seed20260713009_a0_slime_boss.local-turn-graph.actions.json")
        ],
        provenance: BTreeSet::from(["test_fixture".to_string()]),
        source_paths: BTreeSet::new(),
    };

    let record = replay_pair(&candidate, 250).expect("tracked exact witness should replay");

    assert_eq!(record.final_terminal, "Win");
    assert_eq!(record.consumed_action_count, record.supplied_action_count);
    assert!(!record.actions.is_empty());
}
