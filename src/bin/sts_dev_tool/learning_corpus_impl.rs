// Mechanical split from main.rs for learning corpus/log helper types and functions.

#[derive(Debug, Deserialize)]
struct FindingsValueExample {
    rust: String,
    java: String,
}

#[derive(Debug, Deserialize)]
struct FindingsFamily {
    category: String,
    key: String,
    count: usize,
    first_frame: u64,
    last_frame: u64,
    #[serde(default)]
    example_frames: Vec<u64>,
    #[serde(default)]
    example_snapshot_ids: Vec<String>,
    #[serde(default)]
    example_rust_java_values: Vec<FindingsValueExample>,
    #[serde(default)]
    combat_labels: Vec<String>,
    #[serde(default)]
    event_labels: Vec<String>,
    #[serde(default)]
    suggested_artifacts: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct FindingsReport {
    run_id: String,
    classification_label: String,
    counts: sts_simulator::cli::live_comm_admin::LiveRunCounts,
    families: Vec<FindingsFamily>,
}

#[derive(Debug, Deserialize, Serialize)]
struct DecisionBotStrengthSummary {
    run_id: String,
    classification_label: String,
    highest_floor: usize,
    highest_act: usize,
    #[serde(default)]
    slow_search_count: usize,
    #[serde(default)]
    search_timeout_count: usize,
    #[serde(default)]
    exact_turn_disagree_count: usize,
    #[serde(default)]
    exact_turn_skip_count: usize,
    #[serde(default)]
    exact_turn_takeover_count: usize,
    #[serde(default)]
    strict_dominance_disagreement_count: usize,
    #[serde(default)]
    high_threat_disagreement_count: usize,
    #[serde(default)]
    regime_counts: BTreeMap<String, usize>,
}

#[derive(Debug, Clone, Serialize)]
struct DecisionAuditLine {
    frame: Option<u64>,
    line_number: usize,
    snippet: String,
    skipped: bool,
    agrees: bool,
    screened_out_count: usize,
    regime: Option<String>,
    frontier_class: Option<String>,
    dominance: Option<String>,
    confidence: Option<String>,
    takeover: Option<bool>,
    takeover_reason: Option<String>,
    chosen_by: Option<String>,
    frontier_survival: Option<String>,
    exact_survival: Option<String>,
    alternatives: Option<usize>,
    rejection_reasons: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
struct DecisionClusterExample {
    category: String,
    frame: Option<u64>,
    line_number: usize,
    snippet: String,
    screened_out_count: usize,
    regime: Option<String>,
    frontier_class: Option<String>,
    dominance: Option<String>,
    chosen_by: Option<String>,
    takeover_reason: Option<String>,
    frontier_survival: Option<String>,
    exact_survival: Option<String>,
    rejection_reasons: Vec<String>,
}

#[derive(Debug, Serialize)]
struct DecisionExperimentReport {
    run_id: String,
    classification_label: String,
    parity_clean: bool,
    debug_path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    bot_strength: Option<DecisionBotStrengthSummary>,
    category_counts: BTreeMap<String, usize>,
    examples: Vec<DecisionClusterExample>,
}

#[derive(Debug, Serialize)]
struct ExportedDisagreementFixture {
    category: String,
    frame: u64,
    response_id: u64,
    fixture_path: String,
    snippet: String,
    regime: Option<String>,
    frontier_class: Option<String>,
    dominance: Option<String>,
}

#[derive(Debug, Serialize)]
struct ExportedDisagreementFixtureReport {
    run_id: String,
    classification_label: String,
    debug_path: String,
    raw_path: String,
    window_lookback: usize,
    requested_categories: Vec<String>,
    exported: Vec<ExportedDisagreementFixture>,
    missing_frames: Vec<u64>,
}

#[derive(Debug, Serialize)]
struct DecisionTrainingMoveRecord {
    input: String,
    avg_score: f32,
    visits: u32,
    projected_hp: i32,
    projected_block: i32,
    projected_unblocked: i32,
    projected_enemy_total: i32,
    immediate_incoming: i32,
    cluster_size: usize,
}

#[derive(Debug, Serialize)]
struct DecisionTrainingExample {
    fixture_name: String,
    fixture_path: String,
    disagreement_category: Option<String>,
    tags: Vec<String>,
    source: Option<String>,
    source_path: Option<String>,
    response_id: Option<u64>,
    frame_id: Option<u64>,
    observed_command_text: Option<String>,
    audit_source: String,
    bot_chosen_action: String,
    exact_best_action: Option<String>,
    preferred_action: String,
    preferred_action_source: String,
    needs_exact_trigger_target: bool,
    has_strict_disagreement_target: bool,
    has_high_threat_target: bool,
    has_screening_activity_target: bool,
    screened_out_count: usize,
    frontier_self_consistent_target: bool,
    regime: Option<String>,
    frontier_class: Option<String>,
    dominance: Option<String>,
    confidence: Option<String>,
    takeover_reason: Option<String>,
    frontier_survival: Option<String>,
    exact_survival: Option<String>,
    chosen_by: Option<String>,
    legal_moves: usize,
    reduced_legal_moves: usize,
    timed_out: bool,
    top_moves: Vec<DecisionTrainingMoveRecord>,
    root_pipeline: Option<serde_json::Value>,
    decision_trace: Option<serde_json::Value>,
    exact_turn_verdict: Option<serde_json::Value>,
}

#[derive(Debug, Serialize)]
struct DecisionTrainingSetSummary {
    fixture_count: usize,
    out_path: String,
    category_counts: BTreeMap<String, usize>,
    audit_source_counts: BTreeMap<String, usize>,
    preferred_action_source_counts: BTreeMap<String, usize>,
    regime_counts: BTreeMap<String, usize>,
    needs_exact_trigger_target_count: usize,
    high_threat_target_count: usize,
    strict_disagreement_target_count: usize,
    screening_activity_target_count: usize,
    frontier_self_consistent_target_count: usize,
}

#[derive(Debug, Serialize)]
struct ProposalTrainingExample {
    fixture_name: String,
    fixture_path: String,
    disagreement_category: Option<String>,
    response_id: Option<u64>,
    frame_id: Option<u64>,
    audit_source: String,
    regime: Option<String>,
    needs_exact_trigger_target: bool,
    has_strict_disagreement_target: bool,
    has_high_threat_target: bool,
    proposal_input: String,
    proposal_class: Option<String>,
    disposition: String,
    is_frontier_choice: bool,
    is_exact_best: bool,
    veto_target: bool,
    exact_confidence: Option<String>,
    reasons: Vec<String>,
    frontier_outcome: Option<serde_json::Value>,
    exact_outcome: Option<serde_json::Value>,
}

#[derive(Debug, Serialize)]
struct ProposalTrainingSetSummary {
    proposal_count: usize,
    out_path: String,
    audit_source_counts: BTreeMap<String, usize>,
    disposition_counts: BTreeMap<String, usize>,
    proposal_class_counts: BTreeMap<String, usize>,
    reason_counts: BTreeMap<String, usize>,
    veto_target_count: usize,
    exact_best_count: usize,
    needs_exact_trigger_target_count: usize,
}

#[derive(Debug, Serialize)]
struct DecisionCorpusRunSummary {
    run_id: String,
    classification_label: String,
    exported_fixture_count: usize,
    live_shadow_record_count: usize,
    fixture_rerun_record_count: usize,
    missing_frame_count: usize,
}

#[derive(Debug, Serialize)]
struct DecisionCorpusSummary {
    run_count: usize,
    fixture_count: usize,
    categories: Vec<String>,
    out_dir: String,
    runs: Vec<DecisionCorpusRunSummary>,
    frame_summary: DecisionTrainingSetSummary,
    proposal_summary: ProposalTrainingSetSummary,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct StateCorpusRecord {
    sample_id: String,
    source_kind: String,
    source_path: String,
    fixture_name: Option<String>,
    combat_case_id: Option<String>,
    run_id: Option<String>,
    response_id: Option<u64>,
    frame_id: Option<u64>,
    player_class: Option<String>,
    ascension_level: Option<u8>,
    engine_state: String,
    screen_type: Option<String>,
    regime: Option<String>,
    curriculum_buckets: Vec<String>,
    encounter_signature: Vec<String>,
    living_monsters: usize,
    legal_moves: usize,
    reduced_legal_moves: usize,
    timed_out: bool,
    needs_exact_trigger_target: bool,
    has_screening_activity_target: bool,
    screened_out_count: usize,
    decision_probe_source: String,
    decision_audit: serde_json::Value,
    combat_snapshot: serde_json::Value,
}

#[derive(Debug, Serialize)]
struct StateCorpusSummary {
    candidate_count: usize,
    sample_count: usize,
    out_path: String,
    include_bucket_filters: Vec<String>,
    exclude_bucket_filters: Vec<String>,
    source_kind_counts: BTreeMap<String, usize>,
    decision_probe_source_counts: BTreeMap<String, usize>,
    regime_counts: BTreeMap<String, usize>,
    curriculum_bucket_counts: BTreeMap<String, usize>,
    player_class_counts: BTreeMap<String, usize>,
    screen_type_counts: BTreeMap<String, usize>,
    needs_exact_trigger_target_count: usize,
    screening_activity_target_count: usize,
    terminal_filtered_count: usize,
    duplicate_filtered_count: usize,
    bucket_filtered_count: usize,
}

#[derive(Debug, Serialize)]
struct StateCorpusSplitSummary {
    input_path: String,
    out_dir: String,
    include_bucket_filters: Vec<String>,
    exclude_bucket_filters: Vec<String>,
    preserve_trigger_negative_rows: usize,
    total_records: usize,
    kept_records: usize,
    bucket_filtered_count: usize,
    preserved_trigger_negative_count: usize,
    group_count: usize,
    split_counts: BTreeMap<String, usize>,
    split_group_counts: BTreeMap<String, usize>,
    split_trigger_label_counts: BTreeMap<String, BTreeMap<String, usize>>,
    trigger_coverage_adjustments: Vec<String>,
}

#[derive(Debug, Default, Clone, Copy)]
struct StateCorpusFilterStats {
    candidate_count: usize,
    terminal_filtered_count: usize,
    duplicate_filtered_count: usize,
    bucket_filtered_count: usize,
}

fn artifact_path_for_record(
    manifest_path: &std::path::Path,
    artifact: &Option<sts_simulator::cli::live_comm_admin::LiveArtifactRecord>,
) -> Option<PathBuf> {
    let artifact = artifact.as_ref()?;
    if !artifact.present {
        return None;
    }
    let run_dir = manifest_path.parent()?;
    Some(run_dir.join(&artifact.relative_path))
}

fn manifest_entry_for_run_or_latest(
    paths: &sts_simulator::cli::live_comm_admin::LiveLogPaths,
    run_id: Option<&str>,
    label: Option<&str>,
) -> Option<(
    PathBuf,
    sts_simulator::cli::live_comm_admin::LiveRunManifest,
)> {
    let mut entries =
        sts_simulator::cli::live_comm_admin::list_run_manifests_for_audit(paths).ok()?;
    entries.sort_by(|left, right| right.1.run_id.cmp(&left.1.run_id));
    for (manifest_path, manifest) in entries {
        if let Some(run_id) = run_id {
            if manifest.run_id != run_id {
                continue;
            }
        } else if let Some(label) = label {
            if manifest.classification_label != label {
                continue;
            }
        }
        return Some((manifest_path, manifest));
    }
    None
}

fn manifest_entries_for_corpus(
    paths: &sts_simulator::cli::live_comm_admin::LiveLogPaths,
    run_ids: &[String],
    label: Option<&str>,
    latest_runs: usize,
) -> Result<
    Vec<(
        PathBuf,
        sts_simulator::cli::live_comm_admin::LiveRunManifest,
    )>,
    String,
> {
    let mut entries = sts_simulator::cli::live_comm_admin::list_run_manifests_for_audit(paths)
        .map_err(|err| format!("failed to list run manifests: {err}"))?;
    entries.sort_by(|left, right| right.1.run_id.cmp(&left.1.run_id));
    if !run_ids.is_empty() {
        let requested = run_ids.iter().collect::<BTreeSet<_>>();
        let mut selected = entries
            .into_iter()
            .filter(|(_, manifest)| requested.contains(&manifest.run_id))
            .collect::<Vec<_>>();
        selected.sort_by(|left, right| left.1.run_id.cmp(&right.1.run_id));
        if selected.is_empty() {
            return Err("no matching run manifests found for requested run_ids".to_string());
        }
        return Ok(selected);
    }
    let mut selected = entries
        .into_iter()
        .filter(|(_, manifest)| {
            label
                .map(|expected| manifest.classification_label == expected)
                .unwrap_or(true)
        })
        .take(latest_runs)
        .collect::<Vec<_>>();
    if selected.is_empty() {
        return Err("no matching run manifests found for corpus build".to_string());
    }
    Ok(std::mem::take(&mut selected))
}

fn load_findings_report(path: &PathBuf) -> Result<FindingsReport, String> {
    let text = std::fs::read_to_string(path)
        .map_err(|err| format!("failed to read findings '{}': {err}", path.display()))?;
    serde_json::from_str(&text)
        .map_err(|err| format!("failed to parse findings '{}': {err}", path.display()))
}

fn build_findings_report_from_snapshots(
    manifest_path: &std::path::Path,
    manifest: &sts_simulator::cli::live_comm_admin::LiveRunManifest,
) -> Result<(FindingsReport, PathBuf), String> {
    let snapshots_path =
        artifact_path_for_record(manifest_path, &manifest.artifacts.failure_snapshots).ok_or_else(
            || {
                format!(
                    "run '{}' has neither findings.json nor failure_snapshots.jsonl",
                    manifest.run_id
                )
            },
        )?;
    let report_json = sts_simulator::cli::build_finding_report_json(
        &manifest.run_id,
        &manifest.classification_label,
        &manifest.counts,
        &snapshots_path,
    )?;
    let report: FindingsReport = serde_json::from_value(report_json)
        .map_err(|err| format!("failed to decode synthesized findings report: {err}"))?;
    Ok((report, snapshots_path))
}

fn load_bot_strength_summary(path: &PathBuf) -> Result<DecisionBotStrengthSummary, String> {
    let text = std::fs::read_to_string(path)
        .map_err(|err| format!("failed to read bot_strength '{}': {err}", path.display()))?;
    serde_json::from_str(&text)
        .map_err(|err| format!("failed to parse bot_strength '{}': {err}", path.display()))
}

fn audit_field<'a>(line: &'a str, key: &str) -> Option<&'a str> {
    let needle = format!("{key}=");
    let start = line.find(&needle)? + needle.len();
    let rest = &line[start..];
    let end = rest.find(' ').unwrap_or(rest.len());
    Some(&rest[..end])
}

fn parse_bool_field(line: &str, key: &str) -> Option<bool> {
    match audit_field(line, key)? {
        "true" => Some(true),
        "false" => Some(false),
        _ => None,
    }
}

fn parse_usize_field(line: &str, key: &str) -> Option<usize> {
    audit_field(line, key)?.parse().ok()
}

fn parse_frame_marker(line: &str) -> Option<u64> {
    let rest = line.strip_prefix("[F")?;
    let digits = rest.split_once(']')?.0;
    digits.parse().ok()
}

fn parse_exact_turn_audit_line(
    line_number: usize,
    frame: Option<u64>,
    line: &str,
) -> Option<DecisionAuditLine> {
    if !line.contains("[AUDIT] exact_turn ") {
        return None;
    }
    Some(DecisionAuditLine {
        frame,
        line_number,
        snippet: line.trim().to_string(),
        skipped: parse_bool_field(line, "skipped").unwrap_or(false),
        agrees: parse_bool_field(line, "agrees").unwrap_or(false),
        screened_out_count: parse_usize_field(line, "screened_out").unwrap_or(0),
        regime: audit_field(line, "regime").map(str::to_string),
        frontier_class: audit_field(line, "frontier_class").map(str::to_string),
        dominance: audit_field(line, "dominance").map(str::to_string),
        confidence: audit_field(line, "confidence").map(str::to_string),
        takeover: parse_bool_field(line, "takeover"),
        takeover_reason: audit_field(line, "takeover_reason").map(str::to_string),
        chosen_by: audit_field(line, "chosen_by").map(str::to_string),
        frontier_survival: audit_field(line, "frontier_survival").map(str::to_string),
        exact_survival: audit_field(line, "exact_survival").map(str::to_string),
        alternatives: parse_usize_field(line, "alternatives"),
        rejection_reasons: audit_field(line, "rejection_reasons")
            .unwrap_or_default()
            .split(',')
            .filter(|entry| !entry.is_empty())
            .map(str::to_string)
            .collect(),
    })
}

fn survival_rank(label: Option<&str>) -> i32 {
    match label.unwrap_or_default() {
        "forced_loss" => 0,
        "severe_risk" => 1,
        "risky_but_playable" => 2,
        "stable" => 3,
        "safe" => 4,
        _ => -1,
    }
}

fn classify_audit_cluster(audit: &DecisionAuditLine) -> Option<&'static str> {
    if audit.skipped {
        return Some("exact_unavailable");
    }
    if audit.agrees {
        return None;
    }
    if audit.frontier_class.as_deref() == Some("end_turn") {
        return Some("idle_energy_end_turn");
    }
    let frontier_rank = survival_rank(audit.frontier_survival.as_deref());
    let exact_rank = survival_rank(audit.exact_survival.as_deref());
    let high_threat = audit
        .rejection_reasons
        .iter()
        .any(|reason| reason == "high_threat_disagreement");
    match audit.dominance.as_deref() {
        Some("strictly_better_in_window") if exact_rank > frontier_rank => {
            Some("survival_upgrade_not_taken")
        }
        Some("strictly_better_in_window") if high_threat => {
            Some("high_threat_exact_disagree_not_taken")
        }
        Some("strictly_better_in_window") => Some("strict_better_same_survival"),
        Some("strictly_worse_in_window") if high_threat => Some("high_threat_frontier_kept"),
        Some("strictly_worse_in_window") => Some("strict_worse_frontier_kept"),
        _ if high_threat => Some("high_threat_other_disagreement"),
        _ => Some("other_disagreement"),
    }
}

fn build_cluster_example(category: &str, audit: &DecisionAuditLine) -> DecisionClusterExample {
    DecisionClusterExample {
        category: category.to_string(),
        frame: audit.frame,
        line_number: audit.line_number,
        snippet: audit.snippet.clone(),
        screened_out_count: audit.screened_out_count,
        regime: audit.regime.clone(),
        frontier_class: audit.frontier_class.clone(),
        dominance: audit.dominance.clone(),
        chosen_by: audit.chosen_by.clone(),
        takeover_reason: audit.takeover_reason.clone(),
        frontier_survival: audit.frontier_survival.clone(),
        exact_survival: audit.exact_survival.clone(),
        rejection_reasons: audit.rejection_reasons.clone(),
    }
}

fn screening_cluster_category(audit: &DecisionAuditLine) -> Option<&'static str> {
    if audit.screened_out_count == 0 {
        return None;
    }
    if audit.skipped {
        Some("screening_active_exact_unavailable")
    } else {
        Some("screening_active")
    }
}

fn parse_idle_end_turn_examples(
    lines: &[String],
    last_audit: &mut Option<DecisionAuditLine>,
) -> Vec<DecisionClusterExample> {
    let mut examples = Vec::new();
    let mut current_frame = None;
    for (idx, line) in lines.iter().enumerate() {
        let line_number = idx + 1;
        if let Some(frame) = parse_frame_marker(line) {
            current_frame = Some(frame);
        }
        if let Some(audit) = parse_exact_turn_audit_line(line_number, current_frame, line) {
            *last_audit = Some(audit);
            continue;
        }
        if !line.contains("[END DIAG] END") {
            continue;
        }
        let legal_plays = parse_usize_field(line, "legal_plays").unwrap_or(0);
        if legal_plays == 0 {
            continue;
        }
        let Some(context) = last_audit.as_ref() else {
            continue;
        };
        let mut snippet = vec![line.trim().to_string()];
        for follow in lines.iter().skip(idx + 1).take(6) {
            if !follow.contains("[END DIAG]") {
                break;
            }
            snippet.push(follow.trim().to_string());
        }
        let mut example = build_cluster_example("idle_energy_end_turn", context);
        example.line_number = line_number;
        example.snippet = snippet.join(" | ");
        if !example
            .rejection_reasons
            .iter()
            .any(|reason| reason == "end_diag_kept_end_turn")
        {
            example
                .rejection_reasons
                .push("end_diag_kept_end_turn".to_string());
        }
        examples.push(example);
    }
    examples
}

fn analyze_decision_debug(
    debug_path: &PathBuf,
    run_id: &str,
    classification_label: &str,
    parity_clean: bool,
    bot_strength: Option<DecisionBotStrengthSummary>,
) -> Result<DecisionExperimentReport, String> {
    let text = std::fs::read_to_string(debug_path)
        .map_err(|err| format!("failed to read debug '{}': {err}", debug_path.display()))?;
    let lines = text.lines().map(str::to_string).collect::<Vec<_>>();
    let mut category_counts = BTreeMap::new();
    let mut examples = Vec::new();
    let mut last_audit = None;
    let mut current_frame = None;

    for (idx, line) in lines.iter().enumerate() {
        if let Some(frame) = parse_frame_marker(line) {
            current_frame = Some(frame);
        }
        let Some(audit) = parse_exact_turn_audit_line(idx + 1, current_frame, line) else {
            continue;
        };
        if let Some(category) = classify_audit_cluster(&audit) {
            *category_counts.entry(category.to_string()).or_insert(0) += 1;
            examples.push(build_cluster_example(category, &audit));
        }
        if let Some(category) = screening_cluster_category(&audit) {
            *category_counts.entry(category.to_string()).or_insert(0) += 1;
            examples.push(build_cluster_example(category, &audit));
        }
        last_audit = Some(audit);
    }

    for example in parse_idle_end_turn_examples(&lines, &mut last_audit) {
        *category_counts.entry(example.category.clone()).or_insert(0) += 1;
        examples.push(example);
    }

    examples.sort_by(|left, right| {
        category_counts
            .get(&right.category)
            .unwrap_or(&0)
            .cmp(category_counts.get(&left.category).unwrap_or(&0))
            .then_with(|| left.line_number.cmp(&right.line_number))
    });

    Ok(DecisionExperimentReport {
        run_id: run_id.to_string(),
        classification_label: classification_label.to_string(),
        parity_clean,
        debug_path: debug_path.display().to_string(),
        bot_strength,
        category_counts,
        examples,
    })
}

fn load_raw_records_by_response_id(
    raw_path: &Path,
) -> Result<BTreeMap<i64, serde_json::Value>, String> {
    let text = std::fs::read_to_string(raw_path)
        .map_err(|err| format!("failed to read raw log '{}': {err}", raw_path.display()))?;
    let mut records = BTreeMap::new();
    for (idx, line) in text.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let root: serde_json::Value = serde_json::from_str(trimmed).map_err(|err| {
            format!(
                "failed to parse raw log '{}' line {}: {err}",
                raw_path.display(),
                idx + 1
            )
        })?;
        let response_id = root
            .get("protocol_meta")
            .and_then(|meta| meta.get("response_id"))
            .and_then(|value| value.as_i64())
            .ok_or_else(|| {
                format!(
                    "raw log '{}' line {} is missing protocol_meta.response_id",
                    raw_path.display(),
                    idx + 1
                )
            })?;
        records.insert(response_id, root);
    }
    Ok(records)
}

fn load_combat_shadow_records_by_frame(
    path: &Path,
) -> Result<BTreeMap<u64, serde_json::Value>, String> {
    let text = std::fs::read_to_string(path).map_err(|err| {
        format!(
            "failed to read combat shadow log '{}': {err}",
            path.display()
        )
    })?;
    let mut records = BTreeMap::new();
    for (idx, line) in text.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let root: serde_json::Value = serde_json::from_str(trimmed).map_err(|err| {
            format!(
                "failed to parse combat shadow log '{}' line {}: {err}",
                path.display(),
                idx + 1
            )
        })?;
        if root.get("kind").and_then(|value| value.as_str()) != Some("combat_shadow") {
            continue;
        }
        let Some(frame) = root.get("frame").and_then(|value| value.as_u64()) else {
            continue;
        };
        records.insert(frame, root);
    }
    Ok(records)
}

fn live_combat_shadow_path_for_manifest(
    manifest_path: &Path,
    manifest: &sts_simulator::cli::live_comm_admin::LiveRunManifest,
) -> Option<PathBuf> {
    artifact_path_for_record(manifest_path, &manifest.artifacts.combat_decision_audit)
        .or_else(|| artifact_path_for_record(manifest_path, &manifest.artifacts.sidecar_shadow))
}

fn cluster_example_json(example: &DecisionClusterExample) -> serde_json::Value {
    serde_json::json!({
        "category": example.category,
        "frame": example.frame,
        "line_number": example.line_number,
        "snippet": example.snippet,
        "screened_out_count": example.screened_out_count,
        "regime": example.regime,
        "frontier_class": example.frontier_class,
        "dominance": example.dominance,
        "chosen_by": example.chosen_by,
        "takeover_reason": example.takeover_reason,
        "frontier_survival": example.frontier_survival,
        "exact_survival": example.exact_survival,
        "rejection_reasons": example.rejection_reasons,
    })
}

fn response_id_for_frame(records: &BTreeMap<i64, serde_json::Value>, frame: u64) -> Option<i64> {
    records.iter().find_map(|(response_id, root)| {
        let state_frame = root
            .get("protocol_meta")
            .and_then(|meta| meta.get("state_frame_id"))
            .and_then(|value| value.as_u64())?;
        (state_frame == frame).then_some(*response_id)
    })
}

fn write_scenario_fixture_path(fixture: &ScenarioFixture, path: &Path) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|err| {
            format!(
                "failed to create fixture directory '{}': {err}",
                parent.display()
            )
        })?;
    }
    let text = serde_json::to_string_pretty(fixture).map_err(|err| {
        format!(
            "failed to serialize scenario fixture '{}': {err}",
            path.display()
        )
    })?;
    std::fs::write(path, text).map_err(|err| {
        format!(
            "failed to write scenario fixture '{}': {err}",
            path.display()
        )
    })
}

fn sanitize_category_for_filename(category: &str) -> String {
    category
        .chars()
        .map(|ch| match ch {
            'a'..='z' | 'A'..='Z' | '0'..='9' => ch.to_ascii_lowercase(),
            _ => '_',
        })
        .collect()
}

fn collect_export_examples<'a>(
    report: &'a DecisionExperimentReport,
    categories: &[String],
    limit: usize,
) -> Vec<&'a DecisionClusterExample> {
    let category_filter = if categories.is_empty() {
        None
    } else {
        Some(
            categories
                .iter()
                .map(|entry| entry.to_ascii_lowercase())
                .collect::<BTreeSet<_>>(),
        )
    };
    let mut seen_frames = BTreeSet::new();
    let mut selected = Vec::new();
    for example in &report.examples {
        let Some(frame) = example.frame else {
            continue;
        };
        if category_filter
            .as_ref()
            .is_some_and(|allowed| !allowed.contains(&example.category.to_ascii_lowercase()))
        {
            continue;
        }
        if !seen_frames.insert(frame) {
            continue;
        }
        selected.push(example);
        if selected.len() >= limit {
            break;
        }
    }
    selected
}

fn export_disagreement_fixtures(
    raw_path: &Path,
    report: &DecisionExperimentReport,
    combat_shadows_by_frame: Option<&BTreeMap<u64, serde_json::Value>>,
    categories: &[String],
    limit: usize,
    window_lookback: usize,
    out_dir: &Path,
) -> Result<ExportedDisagreementFixtureReport, String> {
    let records = load_raw_records_by_response_id(raw_path)?;
    let mut exported = Vec::new();
    let mut missing_frames = Vec::new();
    for example in collect_export_examples(report, categories, limit) {
        let Some(frame) = example.frame else {
            continue;
        };
        let Some(response_id) = response_id_for_frame(&records, frame) else {
            missing_frames.push(frame);
            continue;
        };
        let start_response_id = std::cmp::max(1_i64, response_id - window_lookback as i64);
        let fixture_name = format!(
            "live_comm_disagreement_{}_f{}",
            sanitize_category_for_filename(&example.category),
            frame
        );
        let mut debug_context_summary = serde_json::json!({
            "live_cluster": cluster_example_json(example),
        });
        if let Some(shadow) = combat_shadows_by_frame
            .and_then(|entries| entries.get(&frame))
            .cloned()
        {
            debug_context_summary
                .as_object_mut()
                .expect("debug_context_summary should be an object")
                .insert("live_combat_shadow".to_string(), shadow);
        }
        let provenance = Some(ScenarioProvenance {
            source: Some("live_comm_disagreement_export".to_string()),
            source_path: Some(raw_path.display().to_string()),
            response_id_range: Some((start_response_id as u64, response_id as u64)),
            failure_frame: Some(frame),
            assertion_source_frames: vec![frame],
            assertion_source_response_ids: vec![response_id as u64],
            debug_context_summary: Some(debug_context_summary),
            notes: vec![format!(
                "exported from decision category '{}' at debug line {}",
                example.category, example.line_number
            )],
            ..ScenarioProvenance::default()
        });
        let fixture = build_fixture_from_record_window(
            &records,
            start_response_id,
            response_id,
            fixture_name.clone(),
            Vec::new(),
            vec![
                "live_comm_disagreement".to_string(),
                example.category.clone(),
                format!("run:{}", report.run_id),
            ],
            provenance,
        )?;
        let output_path = out_dir.join(format!(
            "{}_f{}_{}.fixture.json",
            report.run_id,
            frame,
            sanitize_category_for_filename(&example.category)
        ));
        write_scenario_fixture_path(&fixture, &output_path)?;
        exported.push(ExportedDisagreementFixture {
            category: example.category.clone(),
            frame,
            response_id: response_id as u64,
            fixture_path: output_path.display().to_string(),
            snippet: example.snippet.clone(),
            regime: example.regime.clone(),
            frontier_class: example.frontier_class.clone(),
            dominance: example.dominance.clone(),
        });
    }
    missing_frames.sort_unstable();
    missing_frames.dedup();
    Ok(ExportedDisagreementFixtureReport {
        run_id: report.run_id.clone(),
        classification_label: report.classification_label.clone(),
        debug_path: report.debug_path.clone(),
        raw_path: raw_path.display().to_string(),
        window_lookback,
        requested_categories: categories.to_vec(),
        exported,
        missing_frames,
    })
}

fn render_decision_experiment_report(report: &DecisionExperimentReport, limit: usize) -> String {
    let mut out = String::new();
    out.push_str(&format!(
        "run={} classification={} parity_clean={} debug={}\n",
        report.run_id, report.classification_label, report.parity_clean, report.debug_path
    ));
    if let Some(bot_strength) = report.bot_strength.as_ref() {
        out.push_str(&format!(
            "progression: floor={} act={} exact_turn_disagree={} strict_dominance={} high_threat={} takeovers={} timeouts={}\n",
            bot_strength.highest_floor,
            bot_strength.highest_act,
            bot_strength.exact_turn_disagree_count,
            bot_strength.strict_dominance_disagreement_count,
            bot_strength.high_threat_disagreement_count,
            bot_strength.exact_turn_takeover_count,
            bot_strength.search_timeout_count
        ));
        if !bot_strength.regime_counts.is_empty() {
            let regimes = bot_strength
                .regime_counts
                .iter()
                .map(|(regime, count)| format!("{regime}={count}"))
                .collect::<Vec<_>>()
                .join(", ");
            out.push_str(&format!("regimes: {regimes}\n"));
        }
    }
    out.push_str("categories:\n");
    let mut counts = report.category_counts.iter().collect::<Vec<_>>();
    counts.sort_by(|left, right| right.1.cmp(left.1).then_with(|| left.0.cmp(right.0)));
    for (category, count) in counts {
        out.push_str(&format!("- {category}={count}\n"));
    }
    out.push_str("examples:\n");
    for example in report.examples.iter().take(limit) {
        let reasons = if example.rejection_reasons.is_empty() {
            "-".to_string()
        } else {
            example.rejection_reasons.join(",")
        };
        out.push_str(&format!(
            "- [{}] line={} regime={} frontier_class={} dominance={} chosen_by={} takeover_reason={} survival={}->{} reasons={} :: {}\n",
            example.category,
            example.line_number,
            example.regime.as_deref().unwrap_or("-"),
            example.frontier_class.as_deref().unwrap_or("-"),
            example.dominance.as_deref().unwrap_or("-"),
            example.chosen_by.as_deref().unwrap_or("-"),
            example.takeover_reason.as_deref().unwrap_or("-"),
            example.frontier_survival.as_deref().unwrap_or("-"),
            example.exact_survival.as_deref().unwrap_or("-"),
            reasons,
            example.snippet
        ));
    }
    out
}

fn render_exported_disagreement_fixture_report(
    report: &ExportedDisagreementFixtureReport,
) -> String {
    let mut out = String::new();
    out.push_str(&format!(
        "run={} classification={} raw={} debug={} exported={} missing_frames={}\n",
        report.run_id,
        report.classification_label,
        report.raw_path,
        report.debug_path,
        report.exported.len(),
        report.missing_frames.len()
    ));
    if !report.requested_categories.is_empty() {
        out.push_str(&format!(
            "categories={}\n",
            report.requested_categories.join(",")
        ));
    }
    out.push_str(&format!("window_lookback={}\n", report.window_lookback));
    for export in &report.exported {
        out.push_str(&format!(
            "- [{}] frame={} response_id={} regime={} frontier_class={} dominance={} fixture={}\n",
            export.category,
            export.frame,
            export.response_id,
            export.regime.as_deref().unwrap_or("-"),
            export.frontier_class.as_deref().unwrap_or("-"),
            export.dominance.as_deref().unwrap_or("-"),
            export.fixture_path
        ));
    }
    if !report.missing_frames.is_empty() {
        out.push_str(&format!(
            "missing_frames={}\n",
            report
                .missing_frames
                .iter()
                .map(u64::to_string)
                .collect::<Vec<_>>()
                .join(",")
        ));
    }
    out
}

fn collect_fixture_paths(
    fixtures: &[PathBuf],
    fixture_dirs: &[PathBuf],
) -> Result<Vec<PathBuf>, String> {
    let mut paths = BTreeSet::new();
    for path in fixtures {
        if path.is_file() {
            paths.insert(path.clone());
        } else {
            return Err(format!("fixture path '{}' is not a file", path.display()));
        }
    }
    for dir in fixture_dirs {
        let entries = std::fs::read_dir(dir)
            .map_err(|err| format!("failed to read fixture dir '{}': {err}", dir.display()))?;
        for entry in entries {
            let entry = entry.map_err(|err| {
                format!(
                    "failed to read fixture dir entry '{}': {err}",
                    dir.display()
                )
            })?;
            let path = entry.path();
            if path.is_file()
                && path
                    .file_name()
                    .and_then(|name| name.to_str())
                    .is_some_and(|name| name.ends_with(".fixture.json"))
            {
                paths.insert(path);
            }
        }
    }
    if paths.is_empty() {
        return Err("no fixture paths found".to_string());
    }
    Ok(paths.into_iter().collect())
}

fn disagreement_category_from_tags(tags: &[String]) -> Option<String> {
    tags.iter()
        .find(|tag| *tag != "live_comm_disagreement" && !tag.starts_with("run:"))
        .cloned()
}

fn move_record_from_stat(
    stat: &sts_simulator::bot::combat::CombatMoveStat,
) -> DecisionTrainingMoveRecord {
    DecisionTrainingMoveRecord {
        input: format!("{:?}", stat.input),
        avg_score: stat.avg_score,
        visits: stat.visits,
        projected_hp: stat.projected_hp,
        projected_block: stat.projected_block,
        projected_unblocked: stat.projected_unblocked,
        projected_enemy_total: stat.projected_enemy_total,
        immediate_incoming: stat.immediate_incoming,
        cluster_size: stat.cluster_size,
    }
}

fn move_record_from_live_top_candidate(value: &serde_json::Value) -> DecisionTrainingMoveRecord {
    DecisionTrainingMoveRecord {
        input: json_string_field(value, "move_label").unwrap_or_default(),
        avg_score: value
            .get("avg_score")
            .and_then(|inner| inner.as_f64())
            .unwrap_or_default() as f32,
        visits: 0,
        projected_hp: 0,
        projected_block: 0,
        projected_unblocked: value
            .get("projected_unblocked")
            .and_then(|inner| inner.as_i64())
            .unwrap_or_default() as i32,
        projected_enemy_total: value
            .get("projected_enemy_total")
            .and_then(|inner| inner.as_i64())
            .unwrap_or_default() as i32,
        immediate_incoming: 0,
        cluster_size: value
            .get("cluster_size")
            .and_then(|inner| inner.as_u64())
            .unwrap_or_default() as usize,
    }
}

fn json_string_field(value: &serde_json::Value, key: &str) -> Option<String> {
    value
        .get(key)
        .and_then(|inner| inner.as_str())
        .map(str::to_string)
}

fn json_string_vec_field(value: &serde_json::Value, key: &str) -> Vec<String> {
    value
        .get(key)
        .and_then(|inner| inner.as_array())
        .into_iter()
        .flatten()
        .filter_map(|entry| entry.as_str().map(str::to_string))
        .collect()
}

fn json_bool_field(value: &serde_json::Value, key: &str) -> Option<bool> {
    value.get(key).and_then(|inner| inner.as_bool())
}

fn build_decision_training_example_from_live_shadow(
    run_id: &str,
    raw_path: &Path,
    fixture_path: Option<&Path>,
    example: &DecisionClusterExample,
    response_id: Option<u64>,
    live_shadow: &serde_json::Value,
) -> DecisionTrainingExample {
    let audit = live_shadow
        .get("decision_audit")
        .unwrap_or(&serde_json::Value::Null);
    let exact_turn_verdict = audit.get("exact_turn_verdict").cloned();
    let decision_trace = audit.get("decision_trace").cloned();
    let root_pipeline = audit.get("root_pipeline").cloned();
    let exact_best_action = exact_turn_verdict
        .as_ref()
        .and_then(|value| value.get("best_first_input"))
        .and_then(|value| value.as_str())
        .map(str::to_string);
    let dominance = exact_turn_verdict
        .as_ref()
        .and_then(|value| value.get("dominance"))
        .and_then(|value| value.as_str())
        .map(str::to_string)
        .or_else(|| example.dominance.clone());
    let confidence = exact_turn_verdict
        .as_ref()
        .and_then(|value| value.get("confidence"))
        .and_then(|value| value.as_str())
        .map(str::to_string);
    let regime = audit
        .get("regime")
        .and_then(|value| value.as_str())
        .map(str::to_string)
        .or_else(|| example.regime.clone());
    let frontier_class = decision_trace
        .as_ref()
        .and_then(|value| value.get("frontier_proposal_class"))
        .and_then(|value| value.as_str())
        .map(str::to_string)
        .or_else(|| example.frontier_class.clone());
    let chosen_by = decision_trace
        .as_ref()
        .and_then(|value| value.get("chosen_by"))
        .and_then(|value| value.as_str())
        .map(str::to_string)
        .or_else(|| example.chosen_by.clone());
    let takeover_reason = audit
        .get("takeover_policy")
        .and_then(|value| value.get("takeover_reason"))
        .and_then(|value| value.as_str())
        .map(str::to_string)
        .or_else(|| example.takeover_reason.clone());
    let frontier_survival = decision_trace
        .as_ref()
        .and_then(|value| value.get("decision_outcomes"))
        .and_then(|value| value.get("frontier"))
        .and_then(|value| value.get("survival"))
        .and_then(|value| value.as_str())
        .map(str::to_string)
        .or_else(|| example.frontier_survival.clone());
    let exact_survival = exact_turn_verdict
        .as_ref()
        .and_then(|value| value.get("survival"))
        .and_then(|value| value.as_str())
        .map(str::to_string)
        .or_else(|| example.exact_survival.clone());
    let screened_out_count = root_pipeline
        .as_ref()
        .and_then(|value| value.get("screened_out"))
        .and_then(|value| value.as_array())
        .map(|entries| entries.len())
        .unwrap_or(example.screened_out_count);
    let bot_chosen_action = live_shadow
        .get("chosen_move")
        .and_then(|value| value.as_str())
        .unwrap_or_default()
        .to_string();
    let observed_command_text = (!bot_chosen_action.is_empty()).then(|| bot_chosen_action.clone());
    let rejection_reasons = decision_trace
        .as_ref()
        .map(|value| json_string_vec_field(value, "rejection_reasons"))
        .filter(|reasons| !reasons.is_empty())
        .unwrap_or_else(|| example.rejection_reasons.clone());
    let disagreement_category = Some(example.category.clone());
    let has_strict_disagreement_target = matches!(
        dominance.as_deref(),
        Some("strictly_better_in_window" | "strictly_worse_in_window")
    );
    let has_high_threat_target = disagreement_category
        .as_deref()
        .map(|category| category.starts_with("high_threat_"))
        .unwrap_or(false)
        || rejection_reasons
            .iter()
            .any(|reason| reason == "high_threat_disagreement");
    let has_screening_activity_target = screened_out_count > 0;
    let needs_exact_trigger_target = has_high_threat_target
        || has_strict_disagreement_target
        || matches!(regime.as_deref(), Some("fragile" | "crisis"));
    let (preferred_action, preferred_action_source) =
        if matches!(dominance.as_deref(), Some("strictly_better_in_window"))
            && !matches!(confidence.as_deref(), Some("unavailable"))
        {
            (
                exact_best_action
                    .clone()
                    .unwrap_or_else(|| bot_chosen_action.clone()),
                "exact_turn_strict_better".to_string(),
            )
        } else if let Some(observed) = observed_command_text.clone() {
            (observed, "observed_command".to_string())
        } else {
            (bot_chosen_action.clone(), "frontier_self".to_string())
        };
    let frontier_self_consistent_target = matches!(dominance.as_deref(), Some("incomparable"))
        || exact_best_action
            .as_deref()
            .map(|action| action == bot_chosen_action)
            .unwrap_or(false)
        || rejection_reasons
            .iter()
            .any(|reason| reason == "frontier_agrees");

    DecisionTrainingExample {
        fixture_name: format!(
            "{}_f{}_{}",
            run_id,
            example.frame.unwrap_or_default(),
            sanitize_category_for_filename(&example.category)
        ),
        fixture_path: fixture_path
            .map(|path| path.display().to_string())
            .unwrap_or_default(),
        disagreement_category,
        tags: vec![
            "live_comm_disagreement".to_string(),
            example.category.clone(),
            format!("run:{run_id}"),
        ],
        source: Some("live_combat_shadow".to_string()),
        source_path: Some(raw_path.display().to_string()),
        response_id,
        frame_id: example.frame,
        observed_command_text,
        audit_source: "live_combat_shadow".to_string(),
        bot_chosen_action,
        exact_best_action,
        preferred_action,
        preferred_action_source,
        needs_exact_trigger_target,
        has_strict_disagreement_target,
        has_high_threat_target,
        has_screening_activity_target,
        screened_out_count,
        frontier_self_consistent_target,
        regime,
        frontier_class,
        dominance,
        confidence,
        takeover_reason,
        frontier_survival,
        exact_survival,
        chosen_by,
        legal_moves: live_shadow
            .get("legal_moves")
            .and_then(|value| value.as_u64())
            .unwrap_or_default() as usize,
        reduced_legal_moves: live_shadow
            .get("reduced_legal_moves")
            .and_then(|value| value.as_u64())
            .unwrap_or_default() as usize,
        timed_out: audit
            .get("exact_turn_shadow")
            .and_then(|value| json_bool_field(value, "timed_out"))
            .unwrap_or(false),
        top_moves: live_shadow
            .get("top_candidates")
            .and_then(|value| value.as_array())
            .into_iter()
            .flatten()
            .map(move_record_from_live_top_candidate)
            .collect(),
        root_pipeline,
        decision_trace,
        exact_turn_verdict,
    }
}

fn build_decision_training_example(
    fixture_path: &Path,
    fixture: &ScenarioFixture,
    depth: u32,
) -> Result<DecisionTrainingExample, String> {
    let initial = initialize_fixture_state(fixture);
    let diagnostics = diagnose_root_search_with_depth_and_runtime(
        &initial.engine_state,
        &initial.combat,
        depth,
        0,
        SearchRuntimeBudget::default(),
    );
    let live_shadow = fixture
        .provenance
        .as_ref()
        .and_then(|provenance| provenance.debug_context_summary.as_ref())
        .and_then(|value| value.get("live_combat_shadow"))
        .filter(|value| !value.is_null());
    let live_cluster = fixture
        .provenance
        .as_ref()
        .and_then(|provenance| provenance.debug_context_summary.as_ref())
        .and_then(|value| value.get("live_cluster"));
    let audit = live_shadow
        .and_then(|value| value.get("decision_audit"))
        .unwrap_or(&diagnostics.decision_audit);
    let exact_turn_verdict = audit.get("exact_turn_verdict").cloned();
    let decision_trace = audit.get("decision_trace").cloned();
    let root_pipeline = audit.get("root_pipeline").cloned();
    let exact_best_action = exact_turn_verdict
        .as_ref()
        .and_then(|value| value.get("best_first_input"))
        .and_then(|value| value.as_str())
        .map(str::to_string);
    let dominance = exact_turn_verdict
        .as_ref()
        .and_then(|value| value.get("dominance"))
        .and_then(|value| value.as_str())
        .map(str::to_string);
    let confidence = exact_turn_verdict
        .as_ref()
        .and_then(|value| value.get("confidence"))
        .and_then(|value| value.as_str())
        .map(str::to_string);
    let regime = audit
        .get("regime")
        .and_then(|value| value.as_str())
        .map(str::to_string)
        .or_else(|| json_string_field(live_cluster.unwrap_or(&serde_json::Value::Null), "regime"));
    let frontier_class = decision_trace
        .as_ref()
        .and_then(|value| value.get("frontier_proposal_class"))
        .and_then(|value| value.as_str())
        .map(str::to_string)
        .or_else(|| {
            json_string_field(
                live_cluster.unwrap_or(&serde_json::Value::Null),
                "frontier_class",
            )
        });
    let chosen_by = decision_trace
        .as_ref()
        .and_then(|value| value.get("chosen_by"))
        .and_then(|value| value.as_str())
        .map(str::to_string)
        .or_else(|| {
            json_string_field(
                live_cluster.unwrap_or(&serde_json::Value::Null),
                "chosen_by",
            )
        });
    let takeover_reason = audit
        .get("takeover_policy")
        .and_then(|value| value.get("takeover_reason"))
        .and_then(|value| value.as_str())
        .map(str::to_string)
        .or_else(|| {
            json_string_field(
                live_cluster.unwrap_or(&serde_json::Value::Null),
                "takeover_reason",
            )
        });
    let frontier_survival = decision_trace
        .as_ref()
        .and_then(|value| value.get("decision_outcomes"))
        .and_then(|value| value.get("frontier"))
        .and_then(|value| value.get("survival"))
        .and_then(|value| value.as_str())
        .map(str::to_string)
        .or_else(|| {
            json_string_field(
                live_cluster.unwrap_or(&serde_json::Value::Null),
                "frontier_survival",
            )
        });
    let exact_survival = exact_turn_verdict
        .as_ref()
        .and_then(|value| value.get("survival"))
        .and_then(|value| value.as_str())
        .map(str::to_string)
        .or_else(|| {
            json_string_field(
                live_cluster.unwrap_or(&serde_json::Value::Null),
                "exact_survival",
            )
        });
    let screened_out_count = root_pipeline
        .as_ref()
        .and_then(|value| value.get("screened_out"))
        .and_then(|value| value.as_array())
        .map(|entries| entries.len())
        .or_else(|| {
            live_cluster
                .and_then(|value| value.get("screened_out_count"))
                .and_then(|value| value.as_u64())
                .map(|value| value as usize)
        })
        .unwrap_or(0);
    let observed_command_text = fixture.steps.first().map(|step| step.command.clone());
    let bot_chosen_action = live_shadow
        .and_then(|value| value.get("chosen_move"))
        .and_then(|value| value.as_str())
        .map(str::to_string)
        .unwrap_or_else(|| format!("{:?}", diagnostics.chosen_move));
    let rejection_reasons = decision_trace
        .as_ref()
        .map(|value| json_string_vec_field(value, "rejection_reasons"))
        .filter(|reasons| !reasons.is_empty())
        .or_else(|| live_cluster.map(|value| json_string_vec_field(value, "rejection_reasons")))
        .unwrap_or_default();
    let disagreement_category = disagreement_category_from_tags(&fixture.tags);
    let has_strict_disagreement_target = matches!(
        dominance.as_deref(),
        Some("strictly_better_in_window" | "strictly_worse_in_window")
    );
    let has_high_threat_target = disagreement_category
        .as_deref()
        .map(|category| category.starts_with("high_threat_"))
        .unwrap_or(false)
        || rejection_reasons
            .iter()
            .any(|reason| reason == "high_threat_disagreement");
    let has_screening_activity_target = screened_out_count > 0;
    let needs_exact_trigger_target = has_high_threat_target
        || has_strict_disagreement_target
        || matches!(regime.as_deref(), Some("fragile" | "crisis"));
    let (preferred_action, preferred_action_source) =
        if matches!(dominance.as_deref(), Some("strictly_better_in_window"))
            && !matches!(confidence.as_deref(), Some("unavailable"))
        {
            (
                exact_best_action
                    .clone()
                    .unwrap_or_else(|| bot_chosen_action.clone()),
                "exact_turn_strict_better".to_string(),
            )
        } else if let Some(observed) = observed_command_text.clone() {
            (observed, "observed_command".to_string())
        } else {
            (bot_chosen_action.clone(), "frontier_self".to_string())
        };
    let frontier_self_consistent_target = matches!(dominance.as_deref(), Some("incomparable"))
        || exact_best_action
            .as_deref()
            .map(|action| action == bot_chosen_action)
            .unwrap_or(false)
        || rejection_reasons
            .iter()
            .any(|reason| reason == "frontier_agrees");

    Ok(DecisionTrainingExample {
        fixture_name: fixture.name.clone(),
        fixture_path: fixture_path.display().to_string(),
        disagreement_category,
        tags: fixture.tags.clone(),
        source: fixture
            .provenance
            .as_ref()
            .and_then(|provenance| provenance.source.clone()),
        source_path: fixture
            .provenance
            .as_ref()
            .and_then(|provenance| provenance.source_path.clone()),
        response_id: initial.response_id,
        frame_id: initial.frame_id,
        observed_command_text,
        audit_source: if live_shadow.is_some() {
            "live_combat_shadow".to_string()
        } else if live_cluster.is_some() {
            "fixture_live_cluster".to_string()
        } else {
            "fixture_rerun".to_string()
        },
        bot_chosen_action,
        exact_best_action,
        preferred_action,
        preferred_action_source,
        needs_exact_trigger_target,
        has_strict_disagreement_target,
        has_high_threat_target,
        has_screening_activity_target,
        screened_out_count,
        frontier_self_consistent_target,
        regime,
        frontier_class,
        dominance,
        confidence,
        takeover_reason,
        frontier_survival,
        exact_survival,
        chosen_by,
        legal_moves: live_shadow
            .and_then(|value| value.get("legal_moves"))
            .and_then(|value| value.as_u64())
            .map(|value| value as usize)
            .unwrap_or(diagnostics.legal_moves),
        reduced_legal_moves: live_shadow
            .and_then(|value| value.get("reduced_legal_moves"))
            .and_then(|value| value.as_u64())
            .map(|value| value as usize)
            .unwrap_or(diagnostics.reduced_legal_moves),
        timed_out: diagnostics.timed_out,
        top_moves: diagnostics
            .top_moves
            .iter()
            .map(move_record_from_stat)
            .collect(),
        root_pipeline,
        decision_trace,
        exact_turn_verdict,
    })
}

fn build_decision_training_set(
    fixture_paths: &[PathBuf],
    depth: u32,
) -> Result<Vec<DecisionTrainingExample>, String> {
    fixture_paths
        .iter()
        .map(|path| {
            let text = std::fs::read_to_string(path)
                .map_err(|err| format!("failed to read fixture '{}': {err}", path.display()))?;
            let fixture: ScenarioFixture = serde_json::from_str(&text)
                .map_err(|err| format!("failed to parse fixture '{}': {err}", path.display()))?;
            build_decision_training_example(path, &fixture, depth)
        })
        .collect()
}

fn summarize_decision_training_set(
    records: &[DecisionTrainingExample],
    out: &Path,
) -> DecisionTrainingSetSummary {
    let mut category_counts = BTreeMap::new();
    let mut audit_source_counts = BTreeMap::new();
    let mut preferred_action_source_counts = BTreeMap::new();
    let mut regime_counts = BTreeMap::new();
    let mut needs_exact_trigger_target_count = 0usize;
    let mut high_threat_target_count = 0usize;
    let mut strict_disagreement_target_count = 0usize;
    let mut screening_activity_target_count = 0usize;
    let mut frontier_self_consistent_target_count = 0usize;
    for record in records {
        if let Some(category) = record.disagreement_category.as_ref() {
            *category_counts.entry(category.clone()).or_insert(0) += 1;
        }
        *audit_source_counts
            .entry(record.audit_source.clone())
            .or_insert(0) += 1;
        *preferred_action_source_counts
            .entry(record.preferred_action_source.clone())
            .or_insert(0) += 1;
        if let Some(regime) = record.regime.as_ref() {
            *regime_counts.entry(regime.clone()).or_insert(0) += 1;
        }
        needs_exact_trigger_target_count += usize::from(record.needs_exact_trigger_target);
        high_threat_target_count += usize::from(record.has_high_threat_target);
        strict_disagreement_target_count += usize::from(record.has_strict_disagreement_target);
        screening_activity_target_count += usize::from(record.has_screening_activity_target);
        frontier_self_consistent_target_count +=
            usize::from(record.frontier_self_consistent_target);
    }
    DecisionTrainingSetSummary {
        fixture_count: records.len(),
        out_path: out.display().to_string(),
        category_counts,
        audit_source_counts,
        preferred_action_source_counts,
        regime_counts,
        needs_exact_trigger_target_count,
        high_threat_target_count,
        strict_disagreement_target_count,
        screening_activity_target_count,
        frontier_self_consistent_target_count,
    }
}

fn build_proposal_training_set(
    records: &[DecisionTrainingExample],
) -> Vec<ProposalTrainingExample> {
    let mut proposals = Vec::new();
    for record in records {
        let Some(decision_trace) = record.decision_trace.as_ref() else {
            continue;
        };
        let mut seen_screened_out = BTreeSet::new();
        if let Some(why_not_others) = decision_trace
            .get("why_not_others")
            .and_then(|value| value.as_array())
        {
            for proposal in why_not_others {
                let proposal_input = json_string_field(proposal, "input").unwrap_or_default();
                let proposal_class = json_string_field(proposal, "proposal_class");
                let disposition = json_string_field(proposal, "disposition")
                    .unwrap_or_else(|| "considered".to_string());
                let exact_confidence = json_string_field(proposal, "exact_confidence");
                let reasons = json_string_vec_field(proposal, "reasons");
                proposals.push(ProposalTrainingExample {
                    fixture_name: record.fixture_name.clone(),
                    fixture_path: record.fixture_path.clone(),
                    disagreement_category: record.disagreement_category.clone(),
                    response_id: record.response_id,
                    frame_id: record.frame_id,
                    audit_source: record.audit_source.clone(),
                    regime: record.regime.clone(),
                    needs_exact_trigger_target: record.needs_exact_trigger_target,
                    has_strict_disagreement_target: record.has_strict_disagreement_target,
                    has_high_threat_target: record.has_high_threat_target,
                    proposal_input: proposal_input.clone(),
                    proposal_class,
                    disposition: disposition.clone(),
                    is_frontier_choice: disposition == "frontier_chosen"
                        || proposal_input == record.bot_chosen_action,
                    is_exact_best: record
                        .exact_best_action
                        .as_deref()
                        .map(|action| action == proposal_input)
                        .unwrap_or(false),
                    veto_target: disposition == "screened_out",
                    exact_confidence,
                    reasons,
                    frontier_outcome: proposal.get("frontier_outcome").cloned(),
                    exact_outcome: proposal.get("exact_outcome").cloned(),
                });
            }
        }
        if let Some(screened_out) = decision_trace
            .get("screened_out")
            .and_then(|value| value.as_array())
        {
            for proposal in screened_out {
                let proposal_input = json_string_field(proposal, "input").unwrap_or_default();
                if !seen_screened_out.insert(proposal_input.clone()) {
                    continue;
                }
                let reason = json_string_field(proposal, "reason")
                    .unwrap_or_else(|| "screened_out".to_string());
                proposals.push(ProposalTrainingExample {
                    fixture_name: record.fixture_name.clone(),
                    fixture_path: record.fixture_path.clone(),
                    disagreement_category: record.disagreement_category.clone(),
                    response_id: record.response_id,
                    frame_id: record.frame_id,
                    audit_source: record.audit_source.clone(),
                    regime: record.regime.clone(),
                    needs_exact_trigger_target: record.needs_exact_trigger_target,
                    has_strict_disagreement_target: record.has_strict_disagreement_target,
                    has_high_threat_target: record.has_high_threat_target,
                    proposal_input,
                    proposal_class: json_string_field(proposal, "proposal_class"),
                    disposition: "screened_out".to_string(),
                    is_frontier_choice: false,
                    is_exact_best: false,
                    veto_target: true,
                    exact_confidence: Some("unavailable".to_string()),
                    reasons: vec![reason],
                    frontier_outcome: proposal.get("frontier_outcome").cloned(),
                    exact_outcome: None,
                });
            }
        }
    }
    proposals
}

fn summarize_proposal_training_set(
    records: &[ProposalTrainingExample],
    out: &Path,
) -> ProposalTrainingSetSummary {
    let mut audit_source_counts = BTreeMap::new();
    let mut disposition_counts = BTreeMap::new();
    let mut proposal_class_counts = BTreeMap::new();
    let mut reason_counts = BTreeMap::new();
    let mut veto_target_count = 0usize;
    let mut exact_best_count = 0usize;
    let mut needs_exact_trigger_target_count = 0usize;
    for record in records {
        *audit_source_counts
            .entry(record.audit_source.clone())
            .or_insert(0) += 1;
        *disposition_counts
            .entry(record.disposition.clone())
            .or_insert(0) += 1;
        if let Some(proposal_class) = record.proposal_class.as_ref() {
            *proposal_class_counts
                .entry(proposal_class.clone())
                .or_insert(0) += 1;
        }
        for reason in &record.reasons {
            *reason_counts.entry(reason.clone()).or_insert(0) += 1;
        }
        veto_target_count += usize::from(record.veto_target);
        exact_best_count += usize::from(record.is_exact_best);
        needs_exact_trigger_target_count += usize::from(record.needs_exact_trigger_target);
    }
    ProposalTrainingSetSummary {
        proposal_count: records.len(),
        out_path: out.display().to_string(),
        audit_source_counts,
        disposition_counts,
        proposal_class_counts,
        reason_counts,
        veto_target_count,
        exact_best_count,
        needs_exact_trigger_target_count,
    }
}

fn collect_json_paths(explicit: &[PathBuf], dirs: &[PathBuf]) -> Result<Vec<PathBuf>, String> {
    let mut paths = explicit.iter().cloned().collect::<BTreeSet<_>>();
    for dir in dirs {
        let entries = std::fs::read_dir(dir)
            .map_err(|err| format!("failed to read directory '{}': {err}", dir.display()))?;
        for entry in entries {
            let entry = entry.map_err(|err| {
                format!(
                    "failed to read directory entry in '{}': {err}",
                    dir.display()
                )
            })?;
            let path = entry.path();
            if path.is_file()
                && path
                    .extension()
                    .and_then(|value| value.to_str())
                    .is_some_and(|ext| ext.eq_ignore_ascii_case("json"))
            {
                paths.insert(path);
            }
        }
    }
    Ok(paths.into_iter().collect())
}

fn engine_state_label(engine_state: &sts_simulator::state::core::EngineState) -> String {
    format!("{engine_state:?}")
}

fn living_monster_count(combat: &sts_simulator::runtime::combat::CombatState) -> usize {
    combat
        .entities
        .monsters
        .iter()
        .filter(|monster| {
            monster.current_hp > 0 && !monster.is_dying && !monster.half_dead && !monster.is_escaped
        })
        .count()
}

fn encounter_signature(combat: &sts_simulator::runtime::combat::CombatState) -> Vec<String> {
    combat
        .entities
        .monsters
        .iter()
        .filter(|monster| monster.current_hp > 0 && !monster.is_dying && !monster.half_dead)
        .map(|monster| {
            sts_simulator::content::monsters::EnemyId::from_id(monster.monster_type)
                .map(|enemy_id| format!("{enemy_id:?}"))
                .unwrap_or_else(|| format!("monster_type_{}", monster.monster_type))
        })
        .collect()
}

fn screen_type_from_game_state(game_state: &Value) -> Option<String> {
    game_state
        .get("screen_type")
        .and_then(|value| value.as_str())
        .map(str::to_string)
}

fn player_class_from_game_state(game_state: &Value) -> Option<String> {
    game_state
        .get("class")
        .and_then(|value| value.as_str())
        .map(str::to_string)
}

fn ascension_from_game_state(game_state: &Value) -> Option<u8> {
    game_state
        .get("ascension_level")
        .and_then(|value| value.as_u64().or_else(|| value.as_i64().map(|v| v as u64)))
        .map(|value| value as u8)
}

fn compact_power_snapshot(
    combat: &sts_simulator::runtime::combat::CombatState,
    entity_id: sts_simulator::EntityId,
) -> Vec<serde_json::Value> {
    combat
        .entities
        .power_db
        .get(&entity_id)
        .into_iter()
        .flat_map(|powers| powers.iter())
        .map(|power| {
            serde_json::json!({
                "id": format!("{:?}", power.power_type),
                "amount": power.amount,
                "extra_data": power.extra_data,
            })
        })
        .collect()
}

fn compact_card_snapshot(card: &sts_simulator::runtime::combat::CombatCard) -> serde_json::Value {
    serde_json::json!({
        "id": format!("{:?}", card.id),
        "uuid": card.uuid,
        "upgrades": card.upgrades,
        "cost": card.get_cost(),
        "cost_for_turn": card.cost_for_turn,
        "free_to_play_once": card.free_to_play_once,
    })
}

fn compact_combat_snapshot(
    combat: &sts_simulator::runtime::combat::CombatState,
) -> serde_json::Value {
    serde_json::json!({
        "player": {
            "current_hp": combat.entities.player.current_hp,
            "max_hp": combat.entities.player.max_hp,
            "block": combat.entities.player.block,
            "energy_master": combat.entities.player.energy_master,
            "gold": combat.entities.player.gold,
            "stance": format!("{:?}", combat.entities.player.stance),
            "relics": combat.entities.player.relics.iter().map(|relic| format!("{:?}", relic.id)).collect::<Vec<_>>(),
            "powers": compact_power_snapshot(combat, combat.entities.player.id),
            "potions": combat.entities.potions.iter().map(|potion| {
                potion.as_ref().map(|p| format!("{:?}", p.id))
            }).collect::<Vec<_>>(),
        },
        "monsters": combat.entities.monsters.iter().map(|monster| {
            serde_json::json!({
                "id": sts_simulator::content::monsters::EnemyId::from_id(monster.monster_type)
                    .map(|enemy_id| format!("{enemy_id:?}"))
                    .unwrap_or_else(|| format!("monster_type_{}", monster.monster_type)),
                "entity_id": monster.id,
                "slot": monster.slot,
                "current_hp": monster.current_hp,
                "max_hp": monster.max_hp,
                "block": monster.block,
                "is_dying": monster.is_dying,
                "is_escaped": monster.is_escaped,
                "half_dead": monster.half_dead,
                "planned_move_id": monster.planned_move_id(),
                "powers": compact_power_snapshot(combat, monster.id),
            })
        }).collect::<Vec<_>>(),
        "zones": {
            "hand": combat.zones.hand.iter().map(compact_card_snapshot).collect::<Vec<_>>(),
            "draw_count": combat.zones.draw_pile.len(),
            "discard_count": combat.zones.discard_pile.len(),
            "exhaust_count": combat.zones.exhaust_pile.len(),
            "limbo_count": combat.zones.limbo.len(),
            "queued_count": combat.zones.queued_cards.len(),
        },
        "turn": {
            "turn_count": combat.turn.turn_count,
            "phase": format!("{:?}", combat.turn.current_phase),
            "energy": combat.turn.energy,
            "cards_played_this_turn": combat.turn.counters.cards_played_this_turn,
            "attacks_played_this_turn": combat.turn.counters.attacks_played_this_turn,
        },
        "runtime": {
            "action_queue_len": combat.action_queue_len(),
            "combat_smoked": combat.runtime.combat_smoked,
            "combat_mugged": combat.runtime.combat_mugged,
        }
    })
}

fn count_status_like_cards(cards: &[sts_simulator::runtime::combat::CombatCard]) -> usize {
    cards
        .iter()
        .filter(|card| {
            matches!(
                sts_simulator::content::cards::get_card_definition(card.id).card_type,
                sts_simulator::content::cards::CardType::Status
                    | sts_simulator::content::cards::CardType::Curse
            )
        })
        .count()
}

fn curriculum_buckets_for_state(
    combat: &sts_simulator::runtime::combat::CombatState,
    regime: Option<&str>,
    audit: &serde_json::Value,
) -> Vec<String> {
    let mut buckets = Vec::new();

    if combat.meta.is_elite_fight {
        buckets.push("elite".to_string());
    }
    if combat.meta.is_boss_fight {
        buckets.push("boss".to_string());
    }
    match regime {
        Some("crisis") => buckets.push("regime_crisis".to_string()),
        Some("fragile") => buckets.push("regime_fragile".to_string()),
        _ => {}
    }

    let hand_status = count_status_like_cards(&combat.zones.hand);
    let total_status = hand_status
        + count_status_like_cards(&combat.zones.draw_pile)
        + count_status_like_cards(&combat.zones.discard_pile)
        + count_status_like_cards(&combat.zones.exhaust_pile);
    if hand_status >= 2 || total_status >= 5 {
        buckets.push("status_heavy".to_string());
    }

    let proposal_class_counts = audit
        .get("root_pipeline")
        .and_then(|value| value.get("proposal_class_counts"))
        .and_then(|value| value.as_object());
    let attack_count = proposal_class_counts
        .and_then(|counts| counts.get("attack"))
        .and_then(|value| value.as_u64())
        .unwrap_or(0);
    let setup_count = proposal_class_counts
        .map(|counts| {
            ["power", "skill_utility"]
                .iter()
                .map(|key| {
                    counts
                        .get(*key)
                        .and_then(|value| value.as_u64())
                        .unwrap_or(0)
                })
                .sum::<u64>()
        })
        .unwrap_or(0);
    if combat.turn.energy > 0 && attack_count > 0 && setup_count > 0 {
        buckets.push("setup_window".to_string());
    }

    buckets.sort();
    buckets.dedup();
    buckets
}

fn build_state_record(
    sample_id: String,
    source_kind: &str,
    source_path: &Path,
    fixture_name: Option<String>,
    combat_case_id: Option<String>,
    run_id: Option<String>,
    response_id: Option<u64>,
    frame_id: Option<u64>,
    player_class: Option<String>,
    ascension_level: Option<u8>,
    screen_type: Option<String>,
    engine_state: &sts_simulator::state::core::EngineState,
    combat: &sts_simulator::runtime::combat::CombatState,
    depth: u32,
) -> Result<StateCorpusRecord, String> {
    let diagnostics = diagnose_root_search_with_depth_and_runtime(
        engine_state,
        combat,
        depth,
        0,
        SearchRuntimeBudget::default(),
    );
    let audit = diagnostics.decision_audit.clone();
    let regime = audit
        .get("regime")
        .and_then(|value| value.as_str())
        .map(str::to_string);
    let screened_out_count = audit
        .get("root_pipeline")
        .and_then(|value| value.get("screened_out"))
        .and_then(|value| value.as_array())
        .map(|entries| entries.len())
        .unwrap_or(0);
    let has_screening_activity_target = screened_out_count > 0;
    let exact_verdict = audit.get("exact_turn_verdict");
    let dominance = exact_verdict
        .and_then(|value| value.get("dominance"))
        .and_then(|value| value.as_str());
    let needs_exact_trigger_target = has_screening_activity_target
        || matches!(regime.as_deref(), Some("fragile" | "crisis"))
        || matches!(
            dominance,
            Some("strictly_better_in_window" | "strictly_worse_in_window")
        );
    let curriculum_buckets = curriculum_buckets_for_state(combat, regime.as_deref(), &audit);

    Ok(StateCorpusRecord {
        sample_id,
        source_kind: source_kind.to_string(),
        source_path: source_path.display().to_string(),
        fixture_name,
        combat_case_id,
        run_id,
        response_id,
        frame_id,
        player_class,
        ascension_level,
        engine_state: engine_state_label(engine_state),
        screen_type,
        regime,
        curriculum_buckets,
        encounter_signature: encounter_signature(combat),
        living_monsters: living_monster_count(combat),
        legal_moves: diagnostics.legal_moves,
        reduced_legal_moves: diagnostics.reduced_legal_moves,
        timed_out: diagnostics.timed_out,
        needs_exact_trigger_target,
        has_screening_activity_target,
        screened_out_count,
        decision_probe_source: "root_search_runtime".to_string(),
        decision_audit: audit,
        combat_snapshot: compact_combat_snapshot(combat),
    })
}

fn build_state_record_from_fixture(path: &Path, depth: u32) -> Result<StateCorpusRecord, String> {
    let text = std::fs::read_to_string(path)
        .map_err(|err| format!("failed to read fixture '{}': {err}", path.display()))?;
    let fixture: ScenarioFixture = serde_json::from_str(&text)
        .map_err(|err| format!("failed to parse fixture '{}': {err}", path.display()))?;
    let initial = initialize_fixture_state(&fixture);
    build_state_record(
        format!(
            "fixture:{}:{}",
            fixture.name,
            initial.frame_id.unwrap_or_default()
        ),
        "scenario_fixture",
        path,
        Some(fixture.name.clone()),
        None,
        fixture
            .tags
            .iter()
            .find_map(|tag| tag.strip_prefix("run:").map(str::to_string)),
        initial.response_id,
        initial.frame_id,
        player_class_from_game_state(&fixture.initial_game_state),
        ascension_from_game_state(&fixture.initial_game_state),
        screen_type_from_game_state(&fixture.initial_game_state),
        &initial.engine_state,
        &initial.combat,
        depth,
    )
}

fn build_state_record_from_combat_case(
    path: &Path,
    depth: u32,
) -> Result<StateCorpusRecord, String> {
    let text = std::fs::read_to_string(path)
        .map_err(|err| format!("failed to read combat case '{}': {err}", path.display()))?;
    let case: CombatCase = serde_json::from_str(&text)
        .map_err(|err| format!("failed to parse combat case '{}': {err}", path.display()))?;
    let lowered = lower_case(&case)?;
    let screen_type = match &case.basis {
        sts_simulator::fixtures::combat_case::CombatCaseBasis::ProtocolSnapshot(protocol) => {
            protocol.root_meta.screen_type.clone()
        }
        sts_simulator::fixtures::combat_case::CombatCaseBasis::EncounterTemplate(_) => None,
        sts_simulator::fixtures::combat_case::CombatCaseBasis::LiveWindow(_) => None,
    };
    build_state_record(
        format!(
            "combat_case:{}:{}",
            case.id,
            lowered.frame_id.unwrap_or_default()
        ),
        "combat_case",
        path,
        None,
        Some(case.id.clone()),
        case.tags
            .iter()
            .find_map(|tag| tag.strip_prefix("run:").map(str::to_string)),
        lowered.response_id,
        lowered.frame_id,
        lowered.player_class,
        lowered.ascension_level,
        screen_type,
        &lowered.engine_state,
        &lowered.combat,
        depth,
    )
}

fn raw_record_is_combat_decision_point(root: &serde_json::Value) -> bool {
    let game_state = root.get("game_state");
    game_state
        .and_then(|value| value.get("combat_truth"))
        .is_some()
        && game_state
            .and_then(|value| value.get("combat_observation"))
            .is_some()
}

fn raw_fixture_from_record(
    root: &serde_json::Value,
    run_id: Option<&str>,
    raw_path: &Path,
) -> Option<ScenarioFixture> {
    let game_state = root.get("game_state")?.clone();
    let protocol_meta = root.get("protocol_meta")?.clone();
    let response_id = protocol_meta
        .get("response_id")
        .and_then(|value| value.as_i64())?;
    let frame_id = protocol_meta
        .get("state_frame_id")
        .and_then(|value| value.as_i64())
        .and_then(|value| u64::try_from(value).ok());
    let mut tags = Vec::new();
    if let Some(run_id) = run_id {
        tags.push(format!("run:{run_id}"));
    }
    Some(ScenarioFixture {
        name: format!(
            "raw_state_{}_{}",
            run_id.unwrap_or("adhoc"),
            frame_id.unwrap_or_default()
        ),
        kind: ScenarioKind::Combat,
        oracle_kind: ScenarioOracleKind::Live,
        initial_game_state: game_state,
        initial_protocol_meta: Some(protocol_meta),
        steps: Vec::new(),
        assertions: Vec::new(),
        provenance: Some(ScenarioProvenance {
            source: Some("raw_state_corpus".to_string()),
            source_path: Some(raw_path.display().to_string()),
            response_id_range: Some((response_id as u64, response_id as u64)),
            failure_frame: frame_id,
            ..ScenarioProvenance::default()
        }),
        tags,
    })
}

fn build_state_records_from_raw(
    raw_path: &Path,
    run_id: Option<&str>,
    limit_per_raw: usize,
    depth: u32,
) -> Result<Vec<StateCorpusRecord>, String> {
    let records = load_raw_records_by_response_id(raw_path)?;
    let mut selected = records
        .iter()
        .rev()
        .filter(|(_, root)| raw_record_is_combat_decision_point(root))
        .filter_map(|(response_id, root)| {
            let fixture = raw_fixture_from_record(root, run_id, raw_path)?;
            let initial = initialize_fixture_state(&fixture);
            if !matches!(
                initial.engine_state,
                sts_simulator::state::core::EngineState::CombatPlayerTurn
                    | sts_simulator::state::core::EngineState::PendingChoice(_)
            ) {
                return None;
            }
            Some((response_id, fixture, initial))
        })
        .take(if limit_per_raw == 0 {
            usize::MAX
        } else {
            limit_per_raw
        })
        .collect::<Vec<_>>();
    selected.sort_by_key(|(response_id, _, _)| *response_id);

    let mut out = Vec::new();
    for (response_id, fixture, initial) in selected {
        out.push(build_state_record(
            format!(
                "raw:{}:{}",
                run_id.unwrap_or("adhoc"),
                initial.frame_id.unwrap_or_default()
            ),
            "live_snapshot",
            raw_path,
            Some(fixture.name.clone()),
            None,
            run_id.map(str::to_string),
            Some(*response_id as u64),
            initial.frame_id,
            player_class_from_game_state(&fixture.initial_game_state),
            ascension_from_game_state(&fixture.initial_game_state),
            screen_type_from_game_state(&fixture.initial_game_state),
            &initial.engine_state,
            &initial.combat,
            depth,
        )?);
    }
    Ok(out)
}

fn state_corpus_source_priority(source_kind: &str) -> u8 {
    match source_kind {
        "combat_case" => 3,
        "scenario_fixture" => 2,
        "live_snapshot" => 1,
        _ => 0,
    }
}

fn state_corpus_terminal_like(record: &StateCorpusRecord) -> bool {
    if matches!(record.screen_type.as_deref(), Some("GAME_OVER")) {
        return true;
    }
    if record.living_monsters == 0 {
        return true;
    }
    record
        .combat_snapshot
        .get("player")
        .and_then(|player| player.get("current_hp"))
        .and_then(|value| value.as_i64())
        .is_some_and(|hp| hp <= 0)
}

fn state_corpus_dedup_key(record: &StateCorpusRecord) -> Option<String> {
    if let (Some(run_id), Some(response_id), Some(frame_id)) =
        (&record.run_id, record.response_id, record.frame_id)
    {
        return Some(format!(
            "run:{run_id}:response:{response_id}:frame:{frame_id}"
        ));
    }
    if let Some(combat_case_id) = &record.combat_case_id {
        return Some(format!("combat_case:{combat_case_id}"));
    }
    if let Some(fixture_name) = &record.fixture_name {
        if let Some(run_id) = &record.run_id {
            if let Some(frame_id) = record.frame_id {
                return Some(format!(
                    "fixture_run:{run_id}:frame:{frame_id}:{fixture_name}"
                ));
            }
        }
        return Some(format!("fixture:{fixture_name}"));
    }
    None
}

fn clean_state_corpus_records(
    records: Vec<StateCorpusRecord>,
) -> (Vec<StateCorpusRecord>, StateCorpusFilterStats) {
    let mut stats = StateCorpusFilterStats {
        candidate_count: records.len(),
        ..StateCorpusFilterStats::default()
    };
    let mut kept: Vec<StateCorpusRecord> = Vec::new();
    let mut seen = BTreeMap::<String, usize>::new();

    for record in records {
        if state_corpus_terminal_like(&record) {
            stats.terminal_filtered_count += 1;
            continue;
        }

        if let Some(key) = state_corpus_dedup_key(&record) {
            if let Some(existing_idx) = seen.get(&key).copied() {
                stats.duplicate_filtered_count += 1;
                if state_corpus_source_priority(&record.source_kind)
                    > state_corpus_source_priority(&kept[existing_idx].source_kind)
                {
                    kept[existing_idx] = record;
                }
                continue;
            }
            seen.insert(key, kept.len());
        }

        kept.push(record);
    }

    (kept, stats)
}

fn filter_state_corpus_by_buckets(
    records: Vec<StateCorpusRecord>,
    include_buckets: &[String],
    exclude_buckets: &[String],
    preserve_trigger_negative_rows: usize,
    stats: &mut StateCorpusFilterStats,
) -> (Vec<StateCorpusRecord>, usize) {
    let include = include_buckets
        .iter()
        .map(|value| value.as_str())
        .collect::<BTreeSet<_>>();
    let exclude = exclude_buckets
        .iter()
        .map(|value| value.as_str())
        .collect::<BTreeSet<_>>();

    let mut kept = Vec::new();
    let mut preserved_candidates = Vec::new();

    for record in records {
        let record_buckets = record
            .curriculum_buckets
            .iter()
            .map(|value| value.as_str())
            .collect::<BTreeSet<_>>();
        let include_ok = include.is_empty() || !record_buckets.is_disjoint(&include);
        let exclude_hit = !exclude.is_empty() && !record_buckets.is_disjoint(&exclude);
        let keep = include_ok && !exclude_hit;
        if keep {
            kept.push(record);
            continue;
        }
        stats.bucket_filtered_count += 1;
        if preserve_trigger_negative_rows > 0
            && !record.needs_exact_trigger_target
            && !exclude_hit
            && !include_ok
        {
            preserved_candidates.push(record);
        }
    }

    preserved_candidates.sort_by(|left, right| {
        state_corpus_split_group_key(left)
            .cmp(&state_corpus_split_group_key(right))
            .then_with(|| left.frame_id.cmp(&right.frame_id))
            .then_with(|| left.response_id.cmp(&right.response_id))
            .then_with(|| left.sample_id.cmp(&right.sample_id))
    });

    let mut preserved_count = 0usize;
    for record in preserved_candidates
        .into_iter()
        .take(preserve_trigger_negative_rows)
    {
        kept.push(record);
        preserved_count += 1;
    }

    (kept, preserved_count)
}

fn summarize_state_corpus(
    records: &[StateCorpusRecord],
    out: &Path,
    include_bucket_filters: &[String],
    exclude_bucket_filters: &[String],
    stats: StateCorpusFilterStats,
) -> StateCorpusSummary {
    let mut source_kind_counts = BTreeMap::new();
    let mut decision_probe_source_counts = BTreeMap::new();
    let mut regime_counts = BTreeMap::new();
    let mut curriculum_bucket_counts = BTreeMap::new();
    let mut player_class_counts = BTreeMap::new();
    let mut screen_type_counts = BTreeMap::new();
    let mut needs_exact_trigger_target_count = 0usize;
    let mut screening_activity_target_count = 0usize;

    for record in records {
        *source_kind_counts
            .entry(record.source_kind.clone())
            .or_insert(0) += 1;
        *decision_probe_source_counts
            .entry(record.decision_probe_source.clone())
            .or_insert(0) += 1;
        if let Some(regime) = record.regime.as_ref() {
            *regime_counts.entry(regime.clone()).or_insert(0) += 1;
        }
        for bucket in &record.curriculum_buckets {
            *curriculum_bucket_counts.entry(bucket.clone()).or_insert(0) += 1;
        }
        if let Some(player_class) = record.player_class.as_ref() {
            *player_class_counts.entry(player_class.clone()).or_insert(0) += 1;
        }
        if let Some(screen_type) = record.screen_type.as_ref() {
            *screen_type_counts.entry(screen_type.clone()).or_insert(0) += 1;
        }
        needs_exact_trigger_target_count += usize::from(record.needs_exact_trigger_target);
        screening_activity_target_count += usize::from(record.has_screening_activity_target);
    }

    StateCorpusSummary {
        candidate_count: stats.candidate_count,
        sample_count: records.len(),
        out_path: out.display().to_string(),
        include_bucket_filters: include_bucket_filters.to_vec(),
        exclude_bucket_filters: exclude_bucket_filters.to_vec(),
        source_kind_counts,
        decision_probe_source_counts,
        regime_counts,
        curriculum_bucket_counts,
        player_class_counts,
        screen_type_counts,
        needs_exact_trigger_target_count,
        screening_activity_target_count,
        terminal_filtered_count: stats.terminal_filtered_count,
        duplicate_filtered_count: stats.duplicate_filtered_count,
        bucket_filtered_count: stats.bucket_filtered_count,
    }
}

fn load_state_corpus_records(path: &Path) -> Result<Vec<StateCorpusRecord>, String> {
    let text = std::fs::read_to_string(path)
        .map_err(|err| format!("failed to read state corpus '{}': {err}", path.display()))?;
    let mut records = Vec::new();
    for (line_idx, line) in text.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let record: StateCorpusRecord = serde_json::from_str(trimmed).map_err(|err| {
            format!(
                "failed to parse state corpus '{}' line {}: {err}",
                path.display(),
                line_idx + 1
            )
        })?;
        records.push(record);
    }
    Ok(records)
}

fn stable_fnv1a_64(input: &str) -> u64 {
    let mut hash = 0xcbf29ce484222325u64;
    for byte in input.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

fn state_corpus_split_group_key(record: &StateCorpusRecord) -> String {
    if let Some(combat_case_id) = &record.combat_case_id {
        return format!("combat_case:{combat_case_id}");
    }
    if let Some(run_id) = &record.run_id {
        if !record.encounter_signature.is_empty() {
            return format!(
                "run:{run_id}:encounter:{}",
                record.encounter_signature.join("+")
            );
        }
        if let Some(frame_id) = record.frame_id {
            return format!("run:{run_id}:frame:{frame_id}");
        }
    }
    if let Some(fixture_name) = &record.fixture_name {
        return format!("fixture:{fixture_name}");
    }
    record.source_path.clone()
}

fn split_name_for_group_key(group_key: &str, train_pct: u8, val_pct: u8) -> &'static str {
    let bucket = (stable_fnv1a_64(group_key) % 100) as u8;
    if bucket < train_pct {
        "train"
    } else if bucket < train_pct.saturating_add(val_pct) {
        "val"
    } else {
        "test"
    }
}

fn split_enabled(split: &str, train_pct: u8, val_pct: u8) -> bool {
    match split {
        "train" => train_pct > 0,
        "val" => val_pct > 0,
        "test" => train_pct as u16 + (val_pct as u16) < 100,
        _ => false,
    }
}

fn group_has_trigger_positive(records: &[StateCorpusRecord]) -> bool {
    records
        .iter()
        .any(|record| record.needs_exact_trigger_target)
}

fn group_has_trigger_negative(records: &[StateCorpusRecord]) -> bool {
    records
        .iter()
        .any(|record| !record.needs_exact_trigger_target)
}

fn count_split_trigger_labels(
    split_groups: &BTreeMap<String, Vec<(String, Vec<StateCorpusRecord>)>>,
) -> BTreeMap<String, BTreeMap<String, usize>> {
    let mut counts = BTreeMap::new();
    for (split, groups) in split_groups {
        let mut positives = 0usize;
        let mut negatives = 0usize;
        for (_, records) in groups {
            for record in records {
                if record.needs_exact_trigger_target {
                    positives += 1;
                } else {
                    negatives += 1;
                }
            }
        }
        counts.insert(
            split.clone(),
            BTreeMap::from([
                ("positive".to_string(), positives),
                ("negative".to_string(), negatives),
            ]),
        );
    }
    counts
}

fn split_groups_with_label(
    split_groups: &BTreeMap<String, Vec<(String, Vec<StateCorpusRecord>)>>,
    split: &str,
    want_positive: bool,
) -> usize {
    let Some(groups) = split_groups.get(split) else {
        return 0;
    };
    groups
        .iter()
        .filter(|(_, records)| {
            if want_positive {
                group_has_trigger_positive(records)
            } else {
                group_has_trigger_negative(records)
            }
        })
        .count()
}

fn move_group_between_splits(
    split_groups: &mut BTreeMap<String, Vec<(String, Vec<StateCorpusRecord>)>>,
    from_split: &str,
    to_split: &str,
    group_key: &str,
) -> bool {
    let Some(source_groups) = split_groups.get_mut(from_split) else {
        return false;
    };
    let Some(index) = source_groups
        .iter()
        .position(|(existing_key, _)| existing_key == group_key)
    else {
        return false;
    };
    let group = source_groups.remove(index);
    split_groups
        .entry(to_split.to_string())
        .or_default()
        .push(group);
    true
}

fn enforce_trigger_label_coverage(
    split_groups: &mut BTreeMap<String, Vec<(String, Vec<StateCorpusRecord>)>>,
    train_pct: u8,
    val_pct: u8,
) -> Vec<String> {
    let mut adjustments = Vec::new();
    let trigger_positive_available = split_groups
        .values()
        .flat_map(|groups| groups.iter())
        .any(|(_, records)| group_has_trigger_positive(records));
    let trigger_negative_available = split_groups
        .values()
        .flat_map(|groups| groups.iter())
        .any(|(_, records)| group_has_trigger_negative(records));

    if !trigger_positive_available && !trigger_negative_available {
        return adjustments;
    }

    for split in ["train", "val", "test"] {
        if !split_enabled(split, train_pct, val_pct) {
            continue;
        }

        let counts = count_split_trigger_labels(split_groups);
        let split_counts = counts.get(split).cloned().unwrap_or_default();
        let split_positive = *split_counts.get("positive").unwrap_or(&0);
        if trigger_positive_available && split_positive == 0 {
            let mut candidate: Option<(String, String, usize)> = None;
            for donor in ["train", "val", "test"] {
                if donor == split {
                    continue;
                }
                let donor_label_groups = split_groups_with_label(split_groups, donor, true);
                let donor_can_spare = if split == "train" {
                    donor_label_groups >= 1
                } else {
                    donor_label_groups > 1
                };
                if !donor_can_spare {
                    continue;
                }
                if let Some(groups) = split_groups.get(donor) {
                    for (group_key, records) in groups {
                        if !group_has_trigger_positive(records) {
                            continue;
                        }
                        let group_len = records.len();
                        let choice = (donor.to_string(), group_key.clone(), group_len);
                        match &candidate {
                            Some((_, _, best_len)) if *best_len <= group_len => {}
                            _ => candidate = Some(choice),
                        }
                    }
                }
            }
            if let Some((from_split, group_key, _)) = candidate {
                if move_group_between_splits(split_groups, &from_split, split, &group_key) {
                    adjustments.push(format!(
                        "moved trigger-positive group '{group_key}' from {from_split} to {split}"
                    ));
                }
            }
        }

        let counts = count_split_trigger_labels(split_groups);
        let split_counts = counts.get(split).cloned().unwrap_or_default();
        let split_negative = *split_counts.get("negative").unwrap_or(&0);
        if trigger_negative_available && split_negative == 0 {
            let mut candidate: Option<(String, String, usize)> = None;
            for donor in ["train", "val", "test"] {
                if donor == split {
                    continue;
                }
                let donor_label_groups = split_groups_with_label(split_groups, donor, false);
                let donor_can_spare = if split == "train" {
                    donor_label_groups >= 1
                } else {
                    donor_label_groups > 1
                };
                if !donor_can_spare {
                    continue;
                }
                if let Some(groups) = split_groups.get(donor) {
                    for (group_key, records) in groups {
                        if !group_has_trigger_negative(records) {
                            continue;
                        }
                        let group_len = records.len();
                        let choice = (donor.to_string(), group_key.clone(), group_len);
                        match &candidate {
                            Some((_, _, best_len)) if *best_len <= group_len => {}
                            _ => candidate = Some(choice),
                        }
                    }
                }
            }
            if let Some((from_split, group_key, _)) = candidate {
                if move_group_between_splits(split_groups, &from_split, split, &group_key) {
                    adjustments.push(format!(
                        "moved trigger-negative group '{group_key}' from {from_split} to {split}"
                    ));
                }
            }
        }
    }

    adjustments
}

fn split_state_corpus_records(
    records: Vec<StateCorpusRecord>,
    include_buckets: &[String],
    exclude_buckets: &[String],
    train_pct: u8,
    val_pct: u8,
    preserve_trigger_negative_rows: usize,
) -> Result<
    (
        BTreeMap<String, Vec<StateCorpusRecord>>,
        StateCorpusSplitSummary,
    ),
    String,
> {
    if train_pct as u16 + val_pct as u16 > 100 {
        return Err(format!(
            "invalid split ratios: train_pct({train_pct}) + val_pct({val_pct}) must be <= 100"
        ));
    }
    let input_count = records.len();
    let mut filter_stats = StateCorpusFilterStats::default();
    let (filtered, preserved_trigger_negative_count) = filter_state_corpus_by_buckets(
        records,
        include_buckets,
        exclude_buckets,
        preserve_trigger_negative_rows,
        &mut filter_stats,
    );
    let mut grouped = BTreeMap::<String, Vec<StateCorpusRecord>>::new();
    for record in filtered {
        grouped
            .entry(state_corpus_split_group_key(&record))
            .or_default()
            .push(record);
    }

    let mut split_groups = BTreeMap::<String, Vec<(String, Vec<StateCorpusRecord>)>>::new();

    for (group_key, group_records) in grouped {
        let split = split_name_for_group_key(&group_key, train_pct, val_pct).to_string();
        split_groups
            .entry(split)
            .or_default()
            .push((group_key, group_records));
    }

    let trigger_coverage_adjustments =
        enforce_trigger_label_coverage(&mut split_groups, train_pct, val_pct);

    let mut split_records = BTreeMap::<String, Vec<StateCorpusRecord>>::new();
    let mut split_counts = BTreeMap::<String, usize>::new();
    let mut split_group_counts = BTreeMap::<String, usize>::new();
    for (split, groups) in &split_groups {
        *split_group_counts.entry(split.clone()).or_insert(0) += groups.len();
        let split_rows = split_records.entry(split.clone()).or_default();
        for (_, group_records) in groups {
            *split_counts.entry(split.clone()).or_insert(0) += group_records.len();
            split_rows.extend(group_records.iter().cloned());
        }
    }
    let split_trigger_label_counts = count_split_trigger_labels(&split_groups);

    let summary = StateCorpusSplitSummary {
        input_path: String::new(),
        out_dir: String::new(),
        include_bucket_filters: include_buckets.to_vec(),
        exclude_bucket_filters: exclude_buckets.to_vec(),
        preserve_trigger_negative_rows,
        total_records: input_count,
        kept_records: split_counts.values().copied().sum(),
        bucket_filtered_count: filter_stats.bucket_filtered_count,
        preserved_trigger_negative_count,
        group_count: split_group_counts.values().copied().sum(),
        split_counts,
        split_group_counts,
        split_trigger_label_counts,
        trigger_coverage_adjustments,
    };
    Ok((split_records, summary))
}

fn build_decision_corpus(
    run_entries: &[(
        PathBuf,
        sts_simulator::cli::live_comm_admin::LiveRunManifest,
    )],
    categories: &[String],
    limit_per_run: usize,
    window_lookback: usize,
    depth: u32,
    out_dir: &Path,
) -> Result<DecisionCorpusSummary, String> {
    let fixtures_root = out_dir.join("fixtures");
    let frame_out = out_dir.join("decision_training.jsonl");
    let frame_summary_out = out_dir.join("decision_training_summary.json");
    let proposal_out = out_dir.join("proposal_training.jsonl");
    let proposal_summary_out = out_dir.join("proposal_training_summary.json");
    let corpus_summary_out = out_dir.join("corpus_summary.json");
    std::fs::create_dir_all(&fixtures_root).map_err(|err| {
        format!(
            "failed to create fixtures root '{}': {err}",
            fixtures_root.display()
        )
    })?;

    let mut all_fixture_paths = Vec::new();
    let mut frame_records = Vec::new();
    let mut run_summaries = Vec::new();

    for (manifest_path, manifest) in run_entries {
        let debug_path = match artifact_path_for_record(manifest_path, &manifest.artifacts.debug) {
            Some(path) => path,
            None => continue,
        };
        let raw_path = match artifact_path_for_record(manifest_path, &manifest.artifacts.raw) {
            Some(path) => path,
            None => continue,
        };
        let bot_strength =
            artifact_path_for_record(manifest_path, &manifest.artifacts.bot_strength)
                .map(|path| load_bot_strength_summary(&path))
                .transpose()?;
        let report = analyze_decision_debug(
            &debug_path,
            &manifest.run_id,
            &manifest.classification_label,
            manifest.counts.engine_bugs == 0
                && manifest.counts.content_gaps == 0
                && manifest.counts.timing_diffs == 0
                && manifest.counts.replay_failures == 0,
            bot_strength,
        )?;
        let selected_examples = collect_export_examples(&report, categories, limit_per_run);
        let combat_shadows_by_frame = live_combat_shadow_path_for_manifest(manifest_path, manifest)
            .map(|path| load_combat_shadow_records_by_frame(&path))
            .transpose()?;
        let run_fixture_dir = fixtures_root.join(&manifest.run_id);
        let export_report = export_disagreement_fixtures(
            &raw_path,
            &report,
            combat_shadows_by_frame.as_ref(),
            categories,
            limit_per_run,
            window_lookback,
            &run_fixture_dir,
        )?;
        let exported_by_frame = export_report
            .exported
            .iter()
            .map(|exported| (exported.frame, PathBuf::from(&exported.fixture_path)))
            .collect::<BTreeMap<_, _>>();
        let raw_records = load_raw_records_by_response_id(&raw_path)?;
        let mut live_shadow_record_count = 0usize;
        let mut fixture_rerun_record_count = 0usize;
        for example in selected_examples {
            let Some(frame) = example.frame else {
                continue;
            };
            let fixture_path = exported_by_frame.get(&frame);
            if let Some(shadow) = combat_shadows_by_frame
                .as_ref()
                .and_then(|entries| entries.get(&frame))
            {
                frame_records.push(build_decision_training_example_from_live_shadow(
                    &manifest.run_id,
                    &raw_path,
                    fixture_path.map(PathBuf::as_path),
                    example,
                    response_id_for_frame(&raw_records, frame).map(|id| id as u64),
                    shadow,
                ));
                live_shadow_record_count += 1;
            } else if let Some(path) = fixture_path {
                all_fixture_paths.push(path.clone());
                fixture_rerun_record_count += 1;
            }
        }
        run_summaries.push(DecisionCorpusRunSummary {
            run_id: manifest.run_id.clone(),
            classification_label: manifest.classification_label.clone(),
            exported_fixture_count: export_report.exported.len(),
            live_shadow_record_count,
            fixture_rerun_record_count,
            missing_frame_count: export_report.missing_frames.len(),
        });
    }

    frame_records.extend(build_decision_training_set(&all_fixture_paths, depth)?);
    let proposal_records = build_proposal_training_set(&frame_records);
    let frame_summary = summarize_decision_training_set(&frame_records, &frame_out);
    let proposal_summary = summarize_proposal_training_set(&proposal_records, &proposal_out);

    if let Some(parent) = frame_out.parent() {
        std::fs::create_dir_all(parent).map_err(|err| {
            format!(
                "failed to create corpus output directory '{}': {err}",
                parent.display()
            )
        })?;
    }

    let mut frame_lines = String::new();
    for record in &frame_records {
        frame_lines.push_str(
            &serde_json::to_string(record)
                .map_err(|err| format!("failed to serialize frame training record: {err}"))?,
        );
        frame_lines.push('\n');
    }
    std::fs::write(&frame_out, frame_lines).map_err(|err| {
        format!(
            "failed to write frame training set '{}': {err}",
            frame_out.display()
        )
    })?;

    let mut proposal_lines = String::new();
    for record in &proposal_records {
        proposal_lines.push_str(
            &serde_json::to_string(record)
                .map_err(|err| format!("failed to serialize proposal training record: {err}"))?,
        );
        proposal_lines.push('\n');
    }
    std::fs::write(&proposal_out, proposal_lines).map_err(|err| {
        format!(
            "failed to write proposal training set '{}': {err}",
            proposal_out.display()
        )
    })?;

    std::fs::write(
        &frame_summary_out,
        serde_json::to_string_pretty(&frame_summary)
            .map_err(|err| format!("failed to serialize frame summary: {err}"))?,
    )
    .map_err(|err| {
        format!(
            "failed to write frame summary '{}': {err}",
            frame_summary_out.display()
        )
    })?;
    std::fs::write(
        &proposal_summary_out,
        serde_json::to_string_pretty(&proposal_summary)
            .map_err(|err| format!("failed to serialize proposal summary: {err}"))?,
    )
    .map_err(|err| {
        format!(
            "failed to write proposal summary '{}': {err}",
            proposal_summary_out.display()
        )
    })?;

    let corpus_summary = DecisionCorpusSummary {
        run_count: run_summaries.len(),
        fixture_count: all_fixture_paths.len(),
        categories: categories.to_vec(),
        out_dir: out_dir.display().to_string(),
        runs: run_summaries,
        frame_summary,
        proposal_summary,
    };
    std::fs::write(
        &corpus_summary_out,
        serde_json::to_string_pretty(&corpus_summary)
            .map_err(|err| format!("failed to serialize corpus summary: {err}"))?,
    )
    .map_err(|err| {
        format!(
            "failed to write corpus summary '{}': {err}",
            corpus_summary_out.display()
        )
    })?;

    Ok(corpus_summary)
}

fn recommended_source_files(family: &FindingsFamily) -> Vec<&'static str> {
    let key_lower = family.key.to_ascii_lowercase();
    let combat_labels = family
        .combat_labels
        .iter()
        .map(|label| label.to_ascii_lowercase())
        .collect::<Vec<_>>();
    let event_labels = family
        .event_labels
        .iter()
        .map(|label| label.to_ascii_lowercase())
        .collect::<Vec<_>>();

    let has_combat_label = |needle: &str| combat_labels.iter().any(|label| label.contains(needle));
    let has_event_label = |needle: &str| event_labels.iter().any(|label| label.contains(needle));

    let mut files = Vec::new();
    let mut push = |path: &'static str| {
        if !files.contains(&path) {
            files.push(path);
        }
    };

    match family.category.as_str() {
        "engine_bug" | "content_gap" | "timing" => {
            push("src/engine/action_handlers/damage.rs");
            push("src/content/powers/mod.rs");
            push("../cardcrawl/powers/");
        }
        "validation_failure" => {
            push("src/cli/live_comm_noncombat.rs");
            push("src/cli/live_comm/combat.rs");
            push("../CommunicationMod/src/main/java/communicationmod/GameStateConverter.java");
        }
        _ => {}
    }

    if key_lower.contains("power[strength]") || key_lower.contains("strength") {
        push("src/engine/action_handlers/damage.rs");
        push("src/content/powers/ironclad/rupture.rs");
        push("src/content/powers/core/lose_strength.rs");
        push("src/content/cards/ironclad/flex.rs");
        push("../cardcrawl/powers/RupturePower.java");
        push("../cardcrawl/powers/LoseStrengthPower.java");
        push("../cardcrawl/cards/red/Flex.java");
    }

    if key_lower.contains("modeshift")
        || key_lower.contains("guardianthreshold")
        || has_combat_label("guardian")
    {
        push("src/content/monsters/exordium/the_guardian.rs");
        push("src/content/powers/core/mode_shift.rs");
        push("../cardcrawl/monsters/exordium/TheGuardian.java");
    }

    if key_lower.contains("stasis")
        || has_combat_label("bronze orb")
        || has_combat_label("bronze automaton")
    {
        push("src/content/monsters/city/bronze_orb.rs");
        push("src/engine/action_handlers/cards.rs");
        push("../cardcrawl/actions/unique/ApplyStasisAction.java");
        push("../cardcrawl/powers/StasisPower.java");
    }

    if key_lower.contains("potion")
        || key_lower.contains("elixir")
        || key_lower.contains("blocked_reason")
        || has_event_label("shop")
    {
        push("src/cli/live_comm_noncombat.rs");
        push("src/bot/card_knowledge.rs");
        push("src/bot/noncombat_families/shop.rs");
        push("../CommunicationMod/src/main/java/communicationmod/GameStateConverter.java");
    }

    files
}

fn render_findings_family(family: &FindingsFamily) -> String {
    let labels = if !family.combat_labels.is_empty() {
        family.combat_labels.join(", ")
    } else if !family.event_labels.is_empty() {
        family.event_labels.join(", ")
    } else {
        "n/a".to_string()
    };
    let examples = if family.example_rust_java_values.is_empty() {
        "n/a".to_string()
    } else {
        family
            .example_rust_java_values
            .iter()
            .take(2)
            .map(|value| format!("Rust={} Java={}", value.rust, value.java))
            .collect::<Vec<_>>()
            .join(" | ")
    };
    let frames = if family.example_frames.is_empty() {
        "n/a".to_string()
    } else {
        family
            .example_frames
            .iter()
            .map(|frame| frame.to_string())
            .collect::<Vec<_>>()
            .join(", ")
    };
    let snapshots = if family.example_snapshot_ids.is_empty() {
        "n/a".to_string()
    } else {
        family.example_snapshot_ids.join(", ")
    };
    let artifacts = if family.suggested_artifacts.is_empty() {
        "n/a".to_string()
    } else {
        family.suggested_artifacts.join(", ")
    };
    let source_files = {
        let files = recommended_source_files(family);
        if files.is_empty() {
            "n/a".to_string()
        } else {
            files.join(", ")
        }
    };

    format!(
        "- [{category}] {key}\n  count={count} frames={first}-{last} labels={labels}\n  example_frames={frames}\n  example_snapshots={snapshots}\n  suggested_artifacts={artifacts}\n  suggested_source_files={source_files}\n  example_values={examples}",
        category = family.category,
        key = family.key,
        count = family.count,
        first = family.first_frame,
        last = family.last_frame,
    )
}

#[derive(Debug, Serialize)]
struct LearningBaselineManifest {
    version: u32,
    generated_at: String,
    source: &'static str,
    selected_runs: Vec<LearningBaselineRun>,
    accepted_run_ids: Vec<String>,
    combat_lab_fixtures: Vec<String>,
    reward_case_run_ids: Vec<String>,
    event_case_run_ids: Vec<String>,
    combat_case_run_ids: Vec<String>,
    failure_snapshot_case_run_ids: Vec<String>,
    shadow_case_run_ids: Vec<String>,
    known_noise: Vec<LearningBaselineNoise>,
}

#[derive(Debug, Serialize)]
struct LearningBaselineRun {
    run_id: String,
    classification_label: String,
    validation_status: Option<String>,
    selection_score: i32,
    engine_bugs: usize,
    content_gaps: usize,
    replay_failures: usize,
    manifest_path: String,
    raw_path: Option<String>,
    reward_audit_path: Option<String>,
    event_audit_path: Option<String>,
    combat_suspects_path: Option<String>,
    failure_snapshots_path: Option<String>,
    validation_path: Option<String>,
    sidecar_shadow_path: Option<String>,
}

#[derive(Debug, Serialize)]
struct LearningBaselineNoise {
    run_id: String,
    classification_label: String,
    engine_bugs: usize,
    content_gaps: usize,
    replay_failures: usize,
}

fn python_wrapper(script_rel_path: &str, args: &[&str]) {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let script_path = PathBuf::from(manifest_dir).join(script_rel_path);

    let status = Command::new("python")
        .arg(&script_path)
        .args(args)
        .status()
        .expect("Failed to execute python script. Make sure python is in PATH.");

    if !status.success() {
        std::process::exit(status.code().unwrap_or(1));
    }
}
