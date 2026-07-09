use std::fs;
use std::io::{BufWriter, Write};
use std::path::PathBuf;

use clap::Parser;
use serde::Serialize;
use serde_json::{json, Value};

#[derive(Parser, Debug)]
#[command(
    about = "Export branch path decisions as learning-oriented JSONL records",
    version
)]
struct Args {
    /// Path to a branch_tiny result.json or path.json file.
    #[arg(long)]
    input: PathBuf,

    /// Optional capsule summary.json. When omitted, result.json status/state is used if present.
    #[arg(long)]
    summary: Option<PathBuf>,

    /// Output JSONL path. Writes to stdout when omitted.
    #[arg(long)]
    out: Option<PathBuf>,

    /// Write path-level observable facts JSON instead of per-decision JSONL.
    #[arg(long)]
    facts: bool,
}

#[derive(Serialize)]
struct DecisionRecordV0 {
    schema: &'static str,
    record_id: String,
    input_path: String,
    branch_id: Option<i64>,
    seed: Option<u64>,
    step: Option<i64>,
    state_before: DecisionStateV0,
    selected: DecisionCandidateV0,
    candidates: Vec<DecisionCandidateV0>,
    decision_delta: Option<Value>,
    outcome: DecisionOutcomeV0,
}

#[derive(Serialize)]
struct PathObservableFactsV0 {
    schema: &'static str,
    input_path: String,
    record_count: usize,
    seed: Option<u64>,
    branch_id: Option<i64>,
    outcome: DecisionOutcomeV0,
    curve: PathCurveFactsV0,
    boundary_counts: BoundaryCountsV0,
    selected_labels: Vec<String>,
    card_reward_picks: Vec<LabelAtStepV0>,
    card_reward_skips: Vec<LabelAtStepV0>,
    shop_buys: Vec<LabelAtStepV0>,
    shop_purges: Vec<LabelAtStepV0>,
    shop_leaves: Vec<LabelAtStepV0>,
    boss_relic_picks: Vec<LabelAtStepV0>,
}

#[derive(Default, Serialize)]
struct PathCurveFactsV0 {
    first_act: Option<i64>,
    first_floor: Option<i64>,
    final_act: Option<i64>,
    final_floor: Option<i64>,
    min_hp: Option<i64>,
    min_hp_max: Option<i64>,
    min_hp_floor: Option<i64>,
    min_hp_act: Option<i64>,
    max_hp_seen: Option<i64>,
    final_hp: Option<i64>,
    final_max_hp: Option<i64>,
    min_deck_size: Option<i64>,
    max_deck_size: Option<i64>,
    final_deck_size: Option<i64>,
    max_gold_seen: Option<i64>,
    final_gold: Option<i64>,
    act1_boss_entry_hp: Option<String>,
    act2_boss_entry_hp: Option<String>,
    act3_boss_entry_hp: Option<String>,
    largest_observed_hp_drop: Option<i64>,
    largest_observed_hp_drop_from: Option<String>,
    largest_observed_hp_drop_to: Option<String>,
}

#[derive(Default, Serialize)]
struct BoundaryCountsV0 {
    card_reward: usize,
    shop: usize,
    boss_relic: usize,
    run_choice_purge: usize,
    neow_bonus: usize,
    other: usize,
}

#[derive(Serialize)]
struct LabelAtStepV0 {
    step: Option<i64>,
    loc: String,
    hp: Option<String>,
    gold: Option<i64>,
    deck_size: Option<i64>,
    label: String,
}

#[derive(Default, Serialize)]
struct DecisionStateV0 {
    act: Option<i64>,
    floor: Option<i64>,
    boundary: Option<String>,
    hp: Option<i64>,
    max_hp: Option<i64>,
    gold: Option<i64>,
    deck_size: Option<i64>,
    deck: Vec<String>,
    relics: Vec<String>,
}

#[derive(Default, Serialize)]
struct DecisionCandidateV0 {
    rank: Option<i64>,
    selected: bool,
    auto_expand: Option<bool>,
    inspect_only: Option<String>,
    label: Option<String>,
    lane: Option<String>,
    detail: Option<String>,
    score: Option<i64>,
    key: Option<Value>,
}

#[derive(Clone, Default, Serialize)]
struct DecisionOutcomeV0 {
    status_kind: Option<String>,
    blocker_kind: Option<String>,
    reason: Option<String>,
    subject: Option<String>,
    combat_case: Option<String>,
    final_act: Option<i64>,
    final_floor: Option<i64>,
    final_hp: Option<i64>,
    final_max_hp: Option<i64>,
    final_gold: Option<i64>,
    final_deck_size: Option<i64>,
    terminal_victory: bool,
}

fn main() -> Result<(), String> {
    let args = Args::parse();
    let input = read_json(&args.input)?;
    let summary = args.summary.as_ref().map(read_json).transpose()?;
    let outcome = outcome_from(summary.as_ref(), &input);
    let seed = outcome_seed(summary.as_ref(), &input);
    let paths = path_values(&input)?;

    let mut records = Vec::new();
    for path in paths {
        let branch_id = path.get("branch_id").and_then(Value::as_i64);
        let Some(steps) = path.get("steps").and_then(Value::as_array) else {
            continue;
        };
        for step in steps {
            let step_index = step.get("step").and_then(Value::as_i64);
            let selected = selected_candidate(step);
            let record_id = format!(
                "{}:branch{}:step{}",
                args.input.display(),
                branch_id
                    .map(|id| id.to_string())
                    .unwrap_or_else(|| "unknown".to_string()),
                step_index
                    .map(|id| id.to_string())
                    .unwrap_or_else(|| "unknown".to_string())
            );
            records.push(DecisionRecordV0 {
                schema: "learning_decision_record_v0",
                record_id,
                input_path: args.input.display().to_string(),
                branch_id,
                seed,
                step: step_index,
                state_before: decision_state(step.get("state_before")),
                selected,
                candidates: candidates(step),
                decision_delta: step.get("decision_delta").cloned().filter(|v| !v.is_null()),
                outcome: outcome.clone(),
            });
        }
    }

    if args.facts {
        let facts = path_observable_facts(&args.input, seed, &records);
        write_json_value(args.out, &facts)?;
        return Ok(());
    }

    if let Some(path) = args.out {
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                fs::create_dir_all(parent).map_err(|err| {
                    format!(
                        "failed to create output directory {}: {err}",
                        parent.display()
                    )
                })?;
            }
        }
        let file = fs::File::create(&path)
            .map_err(|err| format!("failed to create {}: {err}", path.display()))?;
        write_records(BufWriter::new(file), &records)?;
        eprintln!(
            "wrote {} DecisionRecordV0 row(s) to {}",
            records.len(),
            path.display()
        );
    } else {
        let stdout = std::io::stdout();
        write_records(BufWriter::new(stdout.lock()), &records)?;
    }
    Ok(())
}

fn write_json_value<T: Serialize>(out: Option<PathBuf>, value: &T) -> Result<(), String> {
    let text = serde_json::to_string_pretty(value)
        .map_err(|err| format!("failed to serialize facts: {err}"))?;
    if let Some(path) = out {
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                fs::create_dir_all(parent).map_err(|err| {
                    format!(
                        "failed to create output directory {}: {err}",
                        parent.display()
                    )
                })?;
            }
        }
        fs::write(&path, format!("{text}\n"))
            .map_err(|err| format!("failed to write {}: {err}", path.display()))?;
        eprintln!("wrote PathObservableFactsV0 to {}", path.display());
    } else {
        println!("{text}");
    }
    Ok(())
}

fn read_json(path: &PathBuf) -> Result<Value, String> {
    let text = fs::read_to_string(path)
        .map_err(|err| format!("failed to read {}: {err}", path.display()))?;
    serde_json::from_str(&text).map_err(|err| format!("failed to parse {}: {err}", path.display()))
}

fn write_records<W: Write>(mut writer: W, records: &[DecisionRecordV0]) -> Result<(), String> {
    for record in records {
        serde_json::to_writer(&mut writer, record)
            .map_err(|err| format!("failed to serialize decision record: {err}"))?;
        writer
            .write_all(b"\n")
            .map_err(|err| format!("failed to write decision record: {err}"))?;
    }
    writer
        .flush()
        .map_err(|err| format!("failed to flush decision records: {err}"))
}

fn path_observable_facts(
    input_path: &PathBuf,
    seed: Option<u64>,
    records: &[DecisionRecordV0],
) -> PathObservableFactsV0 {
    let branch_id = records.iter().find_map(|record| record.branch_id);
    let outcome = records
        .first()
        .map(|record| record.outcome.clone())
        .unwrap_or_default();
    let mut selected_labels = Vec::new();
    let mut card_reward_picks = Vec::new();
    let mut card_reward_skips = Vec::new();
    let mut shop_buys = Vec::new();
    let mut shop_purges = Vec::new();
    let mut shop_leaves = Vec::new();
    let mut boss_relic_picks = Vec::new();
    let mut counts = BoundaryCountsV0::default();

    for record in records {
        if let Some(label) = record.selected.label.as_ref() {
            selected_labels.push(label.clone());
        }
        match record.state_before.boundary.as_deref() {
            Some("Card Reward") => {
                counts.card_reward += 1;
                if selected_label_is_skip(&record.selected) {
                    card_reward_skips.push(label_at_step(record));
                } else {
                    card_reward_picks.push(label_at_step(record));
                }
            }
            Some("Shop") => {
                counts.shop += 1;
                if selected_label_is_shop_leave(&record.selected) {
                    shop_leaves.push(label_at_step(record));
                } else if selected_label_is_purge(&record.selected) {
                    shop_purges.push(label_at_step(record));
                } else {
                    shop_buys.push(label_at_step(record));
                }
            }
            Some("Boss Relic") => {
                counts.boss_relic += 1;
                boss_relic_picks.push(label_at_step(record));
            }
            Some("Run Choice Purge") => counts.run_choice_purge += 1,
            Some("Neow Bonus") => counts.neow_bonus += 1,
            _ => counts.other += 1,
        }
    }

    PathObservableFactsV0 {
        schema: "path_observable_facts_v0",
        input_path: input_path.display().to_string(),
        record_count: records.len(),
        seed,
        branch_id,
        outcome,
        curve: path_curve_facts(records),
        boundary_counts: counts,
        selected_labels,
        card_reward_picks,
        card_reward_skips,
        shop_buys,
        shop_purges,
        shop_leaves,
        boss_relic_picks,
    }
}

fn path_curve_facts(records: &[DecisionRecordV0]) -> PathCurveFactsV0 {
    let mut facts = PathCurveFactsV0::default();
    let first = records.first();
    let last = records.last();
    facts.first_act = first.and_then(|record| record.state_before.act);
    facts.first_floor = first.and_then(|record| record.state_before.floor);
    facts.final_act = last
        .and_then(|record| record.outcome.final_act)
        .or_else(|| last.and_then(|record| record.state_before.act));
    facts.final_floor = last
        .and_then(|record| record.outcome.final_floor)
        .or_else(|| last.and_then(|record| record.state_before.floor));
    facts.final_hp = last
        .and_then(|record| record.outcome.final_hp)
        .or_else(|| last.and_then(|record| record.state_before.hp));
    facts.final_max_hp = last
        .and_then(|record| record.outcome.final_max_hp)
        .or_else(|| last.and_then(|record| record.state_before.max_hp));
    facts.final_deck_size = last
        .and_then(|record| record.outcome.final_deck_size)
        .or_else(|| last.and_then(|record| record.state_before.deck_size));
    facts.final_gold = last
        .and_then(|record| record.outcome.final_gold)
        .or_else(|| last.and_then(|record| record.state_before.gold));

    let mut previous_hp: Option<(i64, String)> = None;
    for record in records {
        let state = &record.state_before;
        update_min(&mut facts.min_deck_size, state.deck_size);
        update_max(&mut facts.max_deck_size, state.deck_size);
        update_max(&mut facts.max_hp_seen, state.max_hp);
        update_max(&mut facts.max_gold_seen, state.gold);

        if let (Some(hp), Some(max_hp)) = (state.hp, state.max_hp) {
            let replace = facts.min_hp.is_none_or(|current| hp < current);
            if replace {
                facts.min_hp = Some(hp);
                facts.min_hp_max = Some(max_hp);
                facts.min_hp_act = state.act;
                facts.min_hp_floor = state.floor;
            }
            let loc = loc_string(state);
            if let Some((prev_hp, prev_loc)) = previous_hp.as_ref() {
                let drop = prev_hp.saturating_sub(hp);
                if drop > 0
                    && facts
                        .largest_observed_hp_drop
                        .is_none_or(|current| drop > current)
                {
                    facts.largest_observed_hp_drop = Some(drop);
                    facts.largest_observed_hp_drop_from = Some(prev_loc.clone());
                    facts.largest_observed_hp_drop_to = Some(loc.clone());
                }
            }
            previous_hp = Some((hp, loc));
        }

        if state.boundary.as_deref() == Some("Boss Relic") {
            match state.act {
                Some(1) => facts.act1_boss_entry_hp = hp_string(state),
                Some(2) => facts.act2_boss_entry_hp = hp_string(state),
                Some(3) => facts.act3_boss_entry_hp = hp_string(state),
                _ => {}
            }
        }
    }
    facts
}

fn update_min(target: &mut Option<i64>, value: Option<i64>) {
    if let Some(value) = value {
        if target.is_none_or(|current| value < current) {
            *target = Some(value);
        }
    }
}

fn update_max(target: &mut Option<i64>, value: Option<i64>) {
    if let Some(value) = value {
        if target.is_none_or(|current| value > current) {
            *target = Some(value);
        }
    }
}

fn selected_label_is_skip(candidate: &DecisionCandidateV0) -> bool {
    candidate
        .label
        .as_deref()
        .is_some_and(|label| label == "Skip card reward")
}

fn selected_label_is_shop_leave(candidate: &DecisionCandidateV0) -> bool {
    candidate
        .label
        .as_deref()
        .is_some_and(|label| label == "Leave shop")
}

fn selected_label_is_purge(candidate: &DecisionCandidateV0) -> bool {
    candidate
        .label
        .as_deref()
        .is_some_and(|label| label.starts_with("Remove "))
}

fn label_at_step(record: &DecisionRecordV0) -> LabelAtStepV0 {
    LabelAtStepV0 {
        step: record.step,
        loc: loc_string(&record.state_before),
        hp: hp_string(&record.state_before),
        gold: record.state_before.gold,
        deck_size: record.state_before.deck_size,
        label: record.selected.label.clone().unwrap_or_default(),
    }
}

fn loc_string(state: &DecisionStateV0) -> String {
    match (state.act, state.floor) {
        (Some(act), Some(floor)) => format!("A{act}F{floor}"),
        _ => "unknown".to_string(),
    }
}

fn hp_string(state: &DecisionStateV0) -> Option<String> {
    Some(format!("{}/{}", state.hp?, state.max_hp?))
}

fn path_values(input: &Value) -> Result<Vec<&Value>, String> {
    if let Some(path) = input.get("path") {
        return Ok(vec![path]);
    }
    if input.get("steps").is_some() {
        return Ok(vec![input]);
    }
    if let Some(paths) = input.as_array() {
        return Ok(paths.iter().collect());
    }
    Err("input is not a result.json, path.json object, or path array".to_string())
}

fn selected_candidate(step: &Value) -> DecisionCandidateV0 {
    let selected_from_pool = step
        .get("candidate_pool")
        .and_then(Value::as_array)
        .and_then(|pool| {
            pool.iter()
                .find(|candidate| candidate.get("selected").and_then(Value::as_bool) == Some(true))
        });
    if let Some(candidate) = selected_from_pool {
        return candidate_record(candidate);
    }
    let mut selected = DecisionCandidateV0::default();
    selected.selected = true;
    selected.label = string_field(step, "label");
    selected.key = step.get("key").cloned().filter(|v| !v.is_null());
    if let Some(annotation) = step.get("annotation") {
        selected.lane = string_field(annotation, "lane");
        selected.detail = string_field(annotation, "detail");
        selected.score = annotation.get("score").and_then(Value::as_i64);
    }
    selected
}

fn candidates(step: &Value) -> Vec<DecisionCandidateV0> {
    step.get("candidate_pool")
        .and_then(Value::as_array)
        .map(|pool| pool.iter().map(candidate_record).collect())
        .unwrap_or_default()
}

fn candidate_record(candidate: &Value) -> DecisionCandidateV0 {
    let annotation = candidate.get("annotation");
    DecisionCandidateV0 {
        rank: candidate.get("rank").and_then(Value::as_i64),
        selected: candidate
            .get("selected")
            .and_then(Value::as_bool)
            .unwrap_or(false),
        auto_expand: candidate.get("auto_expand").and_then(Value::as_bool),
        inspect_only: string_field(candidate, "inspect_only"),
        label: string_field(candidate, "label"),
        lane: annotation.and_then(|a| string_field(a, "lane")),
        detail: annotation.and_then(|a| string_field(a, "detail")),
        score: annotation
            .and_then(|a| a.get("score"))
            .and_then(Value::as_i64),
        key: candidate.get("key").cloned().filter(|v| !v.is_null()),
    }
}

fn decision_state(state: Option<&Value>) -> DecisionStateV0 {
    let Some(state) = state else {
        return DecisionStateV0::default();
    };
    DecisionStateV0 {
        act: state.get("act").and_then(Value::as_i64),
        floor: state.get("floor").and_then(Value::as_i64),
        boundary: string_field(state, "boundary"),
        hp: state.get("hp").and_then(Value::as_i64),
        max_hp: state.get("max_hp").and_then(Value::as_i64),
        gold: state.get("gold").and_then(Value::as_i64),
        deck_size: state.get("deck_size").and_then(Value::as_i64),
        deck: state
            .get("deck")
            .and_then(Value::as_array)
            .map(|cards| cards.iter().filter_map(card_label).collect())
            .unwrap_or_default(),
        relics: state
            .get("relics")
            .and_then(Value::as_array)
            .map(|relics| relics.iter().filter_map(relic_label).collect())
            .unwrap_or_default(),
    }
}

fn card_label(card: &Value) -> Option<String> {
    let id = string_field(card, "id")?;
    let upgrades = card.get("upgrades").and_then(Value::as_i64).unwrap_or(0);
    if upgrades > 0 {
        Some(format!("{id}+{upgrades}"))
    } else {
        Some(id)
    }
}

fn relic_label(relic: &Value) -> Option<String> {
    let id = string_field(relic, "id")?;
    let used_up = relic
        .get("used_up")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    if used_up {
        Some(format!("{id}(used)"))
    } else {
        Some(id)
    }
}

fn outcome_from(summary: Option<&Value>, input: &Value) -> DecisionOutcomeV0 {
    let source = summary.unwrap_or(input);
    let status_kind = source
        .get("status")
        .and_then(|status| string_field(status, "kind"))
        .or_else(|| {
            source
                .get("status")
                .and_then(|status| status.as_str().map(str::to_string))
        });
    let reason = string_field(source, "reason").or_else(|| {
        source
            .get("status")
            .and_then(|status| string_field(status, "reason"))
    });
    let terminal_victory = reason.as_deref() == Some("victory_found")
        || status_kind.as_deref() == Some("terminal")
            && source
                .get("blocker_kind")
                .and_then(Value::as_str)
                .is_some_and(|kind| kind == "terminal");
    DecisionOutcomeV0 {
        status_kind,
        blocker_kind: string_field(source, "blocker_kind"),
        reason,
        subject: string_field(source, "subject"),
        combat_case: string_field(source, "combat_case"),
        final_act: source.get("act").and_then(Value::as_i64).or_else(|| {
            source
                .get("state")
                .and_then(|s| s.get("act"))
                .and_then(Value::as_i64)
        }),
        final_floor: source.get("floor").and_then(Value::as_i64).or_else(|| {
            source
                .get("state")
                .and_then(|s| s.get("floor"))
                .and_then(Value::as_i64)
        }),
        final_hp: source.get("hp").and_then(Value::as_i64).or_else(|| {
            source
                .get("state")
                .and_then(|s| s.get("hp"))
                .and_then(Value::as_i64)
        }),
        final_max_hp: source.get("max_hp").and_then(Value::as_i64).or_else(|| {
            source
                .get("state")
                .and_then(|s| s.get("max_hp"))
                .and_then(Value::as_i64)
        }),
        final_gold: source.get("gold").and_then(Value::as_i64).or_else(|| {
            source
                .get("state")
                .and_then(|s| s.get("gold"))
                .and_then(Value::as_i64)
        }),
        final_deck_size: source.get("deck_size").and_then(Value::as_i64).or_else(|| {
            source
                .get("state")
                .and_then(|s| s.get("deck_size"))
                .and_then(Value::as_i64)
        }),
        terminal_victory,
    }
}

fn outcome_seed(summary: Option<&Value>, input: &Value) -> Option<u64> {
    summary
        .and_then(|summary| summary.get("seed"))
        .and_then(Value::as_u64)
        .or_else(|| input.get("seed").and_then(Value::as_u64))
}

fn string_field(value: &Value, field: &str) -> Option<String> {
    match value.get(field)? {
        Value::String(text) => Some(text.clone()),
        Value::Null => None,
        other => Some(render_json_scalar(other)),
    }
}

fn render_json_scalar(value: &Value) -> String {
    match value {
        Value::String(text) => text.clone(),
        Value::Number(number) => number.to_string(),
        Value::Bool(flag) => flag.to_string(),
        Value::Null => String::new(),
        other => json!(other).to_string(),
    }
}
