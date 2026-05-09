// Log analysis helpers for sts_dev_tool.

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

fn python_wrapper(script_rel_path: &str, args: &[&str]) {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let script_path = PathBuf::from(manifest_dir).join(script_rel_path);

    let status = Command::new("python")
        .arg(&script_path)
        .args(args)
        .status()
        .expect("failed to execute python script");

    if !status.success() {
        std::process::exit(status.code().unwrap_or(1));
    }
}
