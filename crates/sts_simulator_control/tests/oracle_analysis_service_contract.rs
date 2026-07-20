use std::fs;
use std::io::Cursor;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::thread;
use std::time::Duration;
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::{json, Value};
use sts_simulator::eval::run_control::{
    OracleAnalysisAdvanceRequestV1, OracleAnalysisAdvanceStatusV1, OracleCombatSearchResumeKindV1,
};
use sts_simulator::runtime::branch::{
    call_oracle_analysis_tcp_v1, load_oracle_analysis_workspace_v1,
    load_oracle_run_continuation_v1, serve_oracle_analysis_jsonl_v1, serve_oracle_analysis_tcp_v1,
    OracleAnalysisServiceResponseV1, OracleAnalysisWorkspaceV1, OracleRunBudget, OracleRunConfig,
};

const SEED: u64 = 20_260_713_006;

#[test]
fn service_keeps_one_session_alive_autosaves_and_survives_bad_commands() {
    let workspace_path = unique_workspace_path();
    let continuation_path = workspace_path.with_extension("continuation.json");
    let workspace = OracleAnalysisWorkspaceV1::new(OracleRunConfig {
        seed: SEED,
        ascension: 0,
        budget: OracleRunBudget::default(),
    })
    .expect("analysis workspace");
    let root = workspace.view().expect("root view");
    let root_id = root.node_id;
    let candidate_id = root
        .choices
        .first()
        .expect("root choice")
        .candidate_id
        .clone();

    let requests = [
        json!({"id": "view", "command": "view"}),
        json!({"id": "bad", "command": "try", "choice_ref": "tampered"}),
        json!({"id": "ping", "command": "ping"}),
        json!({
            "id": "choose_path",
            "command": "choose_path",
            "node": root_id,
            "candidate_ids": [candidate_id]
        }),
        json!({"id": "back", "command": "back"}),
        json!({
            "id": "export_continuation",
            "command": "export_continuation",
            "node": root_id,
            "path": continuation_path,
        }),
        json!({"id": "save", "command": "save"}),
        json!({"id": "shutdown", "command": "shutdown"}),
    ];
    let input = requests
        .iter()
        .map(Value::to_string)
        .collect::<Vec<_>>()
        .join("\n")
        + "\n";
    let mut output = Vec::new();
    let exit = serve_oracle_analysis_jsonl_v1(
        &workspace_path,
        workspace,
        Cursor::new(input.into_bytes()),
        &mut output,
    )
    .expect("service loop");

    let responses = String::from_utf8(output)
        .expect("utf8 output")
        .lines()
        .map(|line| {
            serde_json::from_str::<OracleAnalysisServiceResponseV1>(line).expect("JSONL response")
        })
        .collect::<Vec<_>>();
    assert_eq!(responses.first().expect("ready").event, "ready");
    assert!(!response(&responses, "bad").ok);
    assert!(response(&responses, "ping").ok, "service continued");
    assert_eq!(response(&responses, "choose_path").revision, 1);
    assert_eq!(response(&responses, "choose_path").saved_revision, 1);
    let choose_path = response(&responses, "choose_path")
        .result
        .as_ref()
        .expect("choose_path result");
    assert_eq!(choose_path["completed"], true);
    assert_eq!(choose_path["applied"].as_array().map(Vec::len), Some(1));
    assert_eq!(response(&responses, "back").revision, 2);
    assert_eq!(response(&responses, "back").saved_revision, 2);
    assert!(response(&responses, "export_continuation").ok);
    assert_eq!(response(&responses, "shutdown").event, "shutdown");
    assert_eq!(exit.revision, 2);
    assert_eq!(exit.saved_revision, 2);

    let restored = load_oracle_analysis_workspace_v1(&workspace_path).expect("saved workspace");
    assert_eq!(restored.session.cursor_node_id(), root_id);
    assert_eq!(restored.view().expect("restored view").children.len(), 1);
    let continuation =
        load_oracle_run_continuation_v1(&continuation_path).expect("exported continuation");
    assert_eq!(continuation.seed, SEED);
    assert!(continuation.explorer_frontier.is_none());

    let _ = fs::remove_file(workspace_path);
    let _ = fs::remove_file(continuation_path);
}

#[test]
fn loopback_endpoint_accepts_independent_calls_and_removes_discovery_file_on_shutdown() {
    let workspace_path = unique_workspace_path();
    let endpoint_path = workspace_path.with_extension("endpoint.json");
    let workspace = OracleAnalysisWorkspaceV1::new(OracleRunConfig {
        seed: SEED,
        ascension: 0,
        budget: OracleRunBudget::default(),
    })
    .expect("analysis workspace");
    let server_workspace_path = workspace_path.clone();
    let server_endpoint_path = endpoint_path.clone();
    let server = thread::spawn(move || {
        serve_oracle_analysis_tcp_v1(
            &server_workspace_path,
            workspace,
            "127.0.0.1:0".parse::<SocketAddr>().expect("bind address"),
            &server_endpoint_path,
        )
    });

    for _ in 0..200 {
        if endpoint_path.is_file() {
            break;
        }
        thread::sleep(Duration::from_millis(10));
    }
    assert!(endpoint_path.is_file(), "endpoint discovery file appeared");

    let ping = call_oracle_analysis_tcp_v1(&endpoint_path, r#"{"id":"ping","command":"ping"}"#)
        .expect("ping resident service");
    assert!(ping.ok);
    assert_eq!(ping.id, Some(json!("ping")));
    let status =
        call_oracle_analysis_tcp_v1(&endpoint_path, r#"{"id":"status","command":"status"}"#)
            .expect("summarize resident service");
    let summary = status.result.expect("status result");
    assert!(summary.get("deck").is_none(), "status stays compact");
    assert!(
        summary["choice_count"]
            .as_u64()
            .is_some_and(|count| count > 0),
        "status retains actionable choices"
    );
    let node = summary["node_id"].as_u64().expect("status node id");
    let explain = call_oracle_analysis_tcp_v1(
        &endpoint_path,
        &format!(r#"{{"id":"explain","command":"explain","node":{node},"owner_rank":0}}"#),
    )
    .expect("explain one resident choice");
    assert!(explain.ok);
    assert!(explain.result.expect("explanation")["label"].is_string());
    let view = call_oracle_analysis_tcp_v1(&endpoint_path, r#"{"id":"view","command":"view"}"#)
        .expect("view resident service");
    assert!(view.ok);
    let shutdown =
        call_oracle_analysis_tcp_v1(&endpoint_path, r#"{"id":"shutdown","command":"shutdown"}"#)
            .expect("shutdown resident service");
    assert_eq!(shutdown.event, "shutdown");

    let exit = server.join().expect("server thread").expect("server exit");
    assert_eq!(exit.revision, 0);
    assert_eq!(exit.saved_revision, 0);
    assert!(!endpoint_path.exists(), "endpoint file removed on exit");
    assert!(workspace_path.is_file(), "workspace saved on shutdown");

    let _ = fs::remove_file(workspace_path);
}

#[test]
fn analysis_workspace_either_resumes_or_materializes_a_verified_combat_witness() {
    let mut budget = OracleRunBudget::default();
    budget.hallway_nodes = 1;
    budget.hallway_ms = 100;
    let mut workspace = OracleAnalysisWorkspaceV1::new(OracleRunConfig {
        seed: SEED,
        ascension: 0,
        budget,
    })
    .expect("analysis workspace");

    for _ in 0..32 {
        let view = workspace.view().expect("analysis view");
        if view.boundary == sts_simulator::eval::run_control::OracleRunBoundaryV1::Combat {
            break;
        }
        let choice_ref = view
            .choices
            .first()
            .unwrap_or_else(|| panic!("node {} has no route to combat", view.node_id))
            .choice_ref
            .clone();
        workspace
            .try_choice(&choice_ref)
            .expect("choose toward combat");
    }
    assert_eq!(
        workspace.view().expect("combat view").boundary,
        sts_simulator::eval::run_control::OracleRunBoundaryV1::Combat
    );

    let request = OracleAnalysisAdvanceRequestV1 {
        max_quanta: 1,
        quantum_nodes: 1,
        quantum_ms: Some(100),
        wall_ms: Some(100),
    };
    let (first, _) = workspace.advance(request.clone()).expect("first advance");
    let first_progress = first.combat.expect("first progress");
    assert_eq!(first_progress.restart_count, 0);
    assert_eq!(
        first_progress.resume_kind,
        OracleCombatSearchResumeKindV1::Fresh
    );

    if let OracleAnalysisAdvanceStatusV1::BoundaryReached { child_node_id } = first.status {
        assert_eq!(
            workspace.view().expect("materialized child").node_id,
            child_node_id
        );
        assert!(
            first_progress.incumbent_action_count.is_some(),
            "a combat boundary may be materialized early only from a verified witness"
        );
        return;
    }
    assert_eq!(first.status, OracleAnalysisAdvanceStatusV1::BudgetUnknown);

    let (second, _) = workspace.advance(request).expect("resumed advance");
    assert_eq!(second.status, OracleAnalysisAdvanceStatusV1::BudgetUnknown);
    let second_progress = second.combat.expect("second progress");
    assert!(
        second_progress.generation_work > first_progress.generation_work,
        "the second advance must continue the resident tactical frontier"
    );
    assert_eq!(second_progress.restart_count, 0);
    assert_eq!(
        second_progress.resume_kind,
        OracleCombatSearchResumeKindV1::SearchResumeExact
    );
}

fn response<'a>(
    responses: &'a [OracleAnalysisServiceResponseV1],
    id: &str,
) -> &'a OracleAnalysisServiceResponseV1 {
    responses
        .iter()
        .find(|response| response.id.as_ref() == Some(&json!(id)))
        .unwrap_or_else(|| panic!("missing response {id}"))
}

fn unique_workspace_path() -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    std::env::temp_dir().join(format!(
        "oracle-analysis-service-{}-{nonce}.json",
        std::process::id()
    ))
}
