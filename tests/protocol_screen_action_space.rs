use std::fs;
use std::path::Path;

use serde_json::Value;
use sts_simulator::protocol::java::{
    build_screen_affordance_snapshot, ProtocolNoncombatActionKind,
};

struct ScreenActionSpaceCase {
    fixture: &'static str,
    source_field: &'static str,
    screen_type: &'static str,
    room_phase: &'static str,
    expected_choice_commands: &'static [(usize, &'static str)],
    expected_labels: &'static [&'static str],
    expected_cancel: Option<&'static str>,
    requires_card_identity: bool,
    requires_active_continuation: bool,
}

const CASES: &[ScreenActionSpaceCase] = &[
    ScreenActionSpaceCase {
        fixture: "combat_grid_select",
        source_field: "combat_action_space",
        screen_type: "GRID",
        room_phase: "COMBAT",
        expected_choice_commands: &[(0, "CHOOSE 0"), (1, "CHOOSE 1")],
        expected_labels: &["strike", "defend"],
        expected_cancel: None,
        requires_card_identity: false,
        requires_active_continuation: false,
    },
    ScreenActionSpaceCase {
        fixture: "combat_discovery_card_reward",
        source_field: "combat_action_space",
        screen_type: "CARD_REWARD",
        room_phase: "COMBAT",
        expected_choice_commands: &[(0, "CHOOSE 0"), (1, "CHOOSE 1")],
        expected_labels: &["flash of steel", "finesse"],
        expected_cancel: Some("SKIP"),
        requires_card_identity: true,
        requires_active_continuation: true,
    },
    ScreenActionSpaceCase {
        fixture: "noncombat_card_reward",
        source_field: "noncombat_action_space",
        screen_type: "CARD_REWARD",
        room_phase: "COMPLETE",
        expected_choice_commands: &[(0, "CHOOSE 0"), (1, "CHOOSE 1")],
        expected_labels: &["pommel strike", "anger"],
        expected_cancel: Some("SKIP"),
        requires_card_identity: false,
        requires_active_continuation: false,
    },
];

fn load_fixture(name: &str) -> Value {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("protocol_screen_action_space")
        .join(format!("{name}.json"));
    let raw = fs::read_to_string(&path)
        .unwrap_or_else(|err| panic!("failed to read fixture {}: {err}", path.display()));
    serde_json::from_str(&raw)
        .unwrap_or_else(|err| panic!("failed to parse fixture {}: {err}", path.display()))
}

#[test]
fn screen_action_space_contract_fixtures_parse_without_legacy_choice_list() {
    for case in CASES {
        let root = load_fixture(case.fixture);
        assert!(
            root.get("game_state")
                .and_then(|game_state| game_state.get("choice_list"))
                .is_none(),
            "{} must not rely on game_state.choice_list",
            case.fixture
        );
        assert_eq!(
            root["game_state"]["screen_type"].as_str(),
            Some(case.screen_type),
            "{} game_state.screen_type",
            case.fixture
        );
        assert_eq!(
            root["game_state"]["room_phase"].as_str(),
            Some(case.room_phase),
            "{} game_state.room_phase",
            case.fixture
        );

        assert_raw_action_space_shape(&root, case);

        let snapshot = build_screen_affordance_snapshot(&root["protocol_meta"])
            .unwrap_or_else(|err| panic!("{} failed action-space parse: {err}", case.fixture))
            .unwrap_or_else(|| panic!("{} missing parsed action space", case.fixture));
        assert_eq!(
            snapshot.screen_type.as_deref(),
            Some(case.screen_type),
            "{} parsed screen_type",
            case.fixture
        );
        assert_eq!(
            snapshot.choice_labels(),
            case.expected_labels
                .iter()
                .map(|label| (*label).to_string())
                .collect::<Vec<_>>(),
            "{} parsed choice labels",
            case.fixture
        );
        for (choice_index, command) in case.expected_choice_commands {
            assert_eq!(
                snapshot.command_for_choice_index(*choice_index),
                Some(*command),
                "{} parsed command for choice {choice_index}",
                case.fixture
            );
        }
        if let Some(cancel_command) = case.expected_cancel {
            assert_eq!(
                snapshot.first_command_for_kind(ProtocolNoncombatActionKind::Cancel),
                Some(cancel_command),
                "{} parsed cancel command",
                case.fixture
            );
        }
    }
}

fn assert_raw_action_space_shape(root: &Value, case: &ScreenActionSpaceCase) {
    let protocol_meta = root["protocol_meta"]
        .as_object()
        .unwrap_or_else(|| panic!("{} missing protocol_meta object", case.fixture));
    let action_space = protocol_meta
        .get(case.source_field)
        .and_then(Value::as_object)
        .unwrap_or_else(|| panic!("{} missing {}", case.fixture, case.source_field));
    assert_eq!(
        action_space.get("screen_type").and_then(Value::as_str),
        Some(case.screen_type),
        "{} {}.screen_type",
        case.fixture,
        case.source_field
    );
    let actions = action_space
        .get("actions")
        .and_then(Value::as_array)
        .unwrap_or_else(|| panic!("{} {}.actions missing", case.fixture, case.source_field));
    assert!(
        !actions.is_empty(),
        "{} {}.actions must be non-empty",
        case.fixture,
        case.source_field
    );

    for action in actions {
        let action_id = required_str(action, "action_id", case.fixture);
        let kind = required_str(action, "kind", case.fixture);
        let command = required_str(action, "command", case.fixture);
        assert!(
            !action_id.is_empty() && !command.is_empty(),
            "{} actions must have stable ids and commands",
            case.fixture
        );

        if case.source_field == "combat_action_space" {
            assert!(
                action
                    .get("target_required")
                    .and_then(Value::as_bool)
                    .is_some(),
                "{} combat action {action_id} missing target_required",
                case.fixture
            );
            assert!(
                action
                    .get("target_options")
                    .and_then(Value::as_array)
                    .is_some(),
                "{} combat action {action_id} missing target_options",
                case.fixture
            );
        }

        if matches!(kind, "submit_choice" | "choose") {
            assert!(
                action.get("choice_index").and_then(Value::as_u64).is_some(),
                "{} choice action {action_id} missing choice_index",
                case.fixture
            );
            assert!(
                action
                    .get("choice_label")
                    .and_then(Value::as_str)
                    .is_some_and(|label| !label.is_empty()),
                "{} choice action {action_id} missing choice_label",
                case.fixture
            );
            if case.requires_card_identity {
                assert!(
                    action
                        .get("card_uuid")
                        .and_then(Value::as_str)
                        .is_some_and(|uuid| !uuid.is_empty()),
                    "{} card reward action {action_id} missing card_uuid",
                    case.fixture
                );
                assert!(
                    action
                        .get("card_id")
                        .and_then(Value::as_str)
                        .is_some_and(|id| !id.is_empty()),
                    "{} card reward action {action_id} missing card_id",
                    case.fixture
                );
            }
        }
    }

    if case.requires_active_continuation {
        let continuation = protocol_meta
            .get("continuation_state")
            .and_then(Value::as_object)
            .unwrap_or_else(|| panic!("{} missing continuation_state", case.fixture));
        assert_eq!(
            continuation.get("state").and_then(Value::as_str),
            Some("active"),
            "{} continuation_state.state",
            case.fixture
        );
        assert_eq!(
            continuation.get("screen_type").and_then(Value::as_str),
            Some(case.screen_type),
            "{} continuation_state.screen_type",
            case.fixture
        );
    }
}

fn required_str<'a>(value: &'a Value, key: &str, fixture: &str) -> &'a str {
    value
        .get(key)
        .and_then(Value::as_str)
        .unwrap_or_else(|| panic!("{fixture} action missing {key}"))
}
