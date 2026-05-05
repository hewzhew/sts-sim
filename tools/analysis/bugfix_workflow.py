from __future__ import annotations

import argparse
import json
import re
import subprocess
import sys
from pathlib import Path
from typing import Any


DEFAULT_REPO_ROOT = Path(r"D:\rust\sts_simulator")
DEFAULT_RAW_PATH = DEFAULT_REPO_ROOT / "logs" / "current" / "live_comm_raw.jsonl"
DEFAULT_DEBUG_PATH = DEFAULT_REPO_ROOT / "logs" / "current" / "live_comm_debug.txt"
DEFAULT_OUT_DIR = DEFAULT_REPO_ROOT / "tests" / "live_regressions"
DEFAULT_WINDOW_LOOKBACK = 3
SNAPSHOT_DIFF_RE = re.compile(
    r"^(?P<field>.+?)\s+\[(?P<kind>[A-Z_]+)\]\s+Rust=(?P<rust>.+?)\s+Java=(?P<java>.+)$"
)


def slugify(value: str) -> str:
    value = value.strip().lower()
    value = re.sub(r"[^a-z0-9]+", "_", value)
    return value.strip("_") or "bug"


def run_checked(command: list[str], cwd: Path) -> None:
    print("+", " ".join(str(part) for part in command))
    subprocess.run(command, cwd=cwd, check=True)


def run_capture(command: list[str], cwd: Path) -> subprocess.CompletedProcess[str]:
    print("+", " ".join(str(part) for part in command))
    return subprocess.run(
        command,
        cwd=cwd,
        check=False,
        capture_output=True,
        text=True,
    )


def field_fallbacks(field: str) -> list[str]:
    fallbacks = [field]
    if ".power[" in field and not field.endswith(".amount"):
        fallbacks.append(f"{field}.amount")
    return fallbacks


def scalar_value_from_assertion(assertion: dict[str, Any]) -> dict[str, Any]:
    expected_kind = assertion.get("expected_kind")
    expected_value = assertion.get("expected_value")
    if expected_kind == "missing":
        return {"kind": "missing"}
    if expected_kind == "number":
        return {"kind": "number", "value": int(expected_value)}
    if expected_kind == "string":
        return {"kind": "string", "value": str(expected_value)}
    if expected_kind == "bool":
        return {"kind": "bool", "value": bool(expected_value)}
    raise SystemExit(f"unsupported scenario assertion expected_kind: {expected_kind!r}")


def parse_case_step_from_command(command: str) -> dict[str, Any]:
    parts = command.split()
    if parts == ["END"]:
        return {"kind": "end"}
    if parts == ["CANCEL"]:
        return {"kind": "cancel"}
    if len(parts) == 2 and parts[0] == "PLAY":
        return {"kind": "play", "selector": {"kind": "index", "index": int(parts[1])}}
    if len(parts) == 3 and parts[0] == "PLAY":
        return {
            "kind": "play",
            "selector": {"kind": "index", "index": int(parts[1])},
            "target": int(parts[2]),
        }
    if len(parts) == 3 and parts[0] == "POTION" and parts[1] == "USE":
        return {"kind": "potion_use", "slot": int(parts[2])}
    if len(parts) == 4 and parts[0] == "POTION" and parts[1] == "USE":
        return {"kind": "potion_use", "slot": int(parts[2]), "target": int(parts[3])}
    if parts == ["HUMAN_CARD_REWARD", "SKIP"]:
        return {"kind": "cancel"}
    if len(parts) == 2 and parts[0] == "HUMAN_CARD_REWARD":
        return {"kind": "choose", "index": int(parts[1])}
    raise SystemExit(f"unsupported raw scenario step for CombatCase conversion: {command!r}")


def scenario_step_to_case_step(step: dict[str, Any]) -> dict[str, Any]:
    structured = step.get("structured")
    if isinstance(structured, dict):
        case_step = dict(structured)
    else:
        case_step = parse_case_step_from_command(str(step.get("command", "")))
    result: dict[str, Any] = {"step": case_step}
    for key in ("label", "response_id", "frame_id", "command_kind"):
        value = step.get(key)
        if value is not None:
            result[key] = value
    return result


def scenario_assertion_to_case_expectation(assertion: dict[str, Any]) -> dict[str, Any]:
    result: dict[str, Any] = {
        "check": {
            "kind": "path",
            "field": assertion["field"],
            "expected": scalar_value_from_assertion(assertion),
        }
    }
    for key in ("response_id", "frame_id", "note"):
        value = assertion.get(key)
        if value is not None:
            result[key] = value
    return result


def oracle_from_scenario_fixture(fixture: dict[str, Any]) -> dict[str, Any]:
    oracle_kind = fixture.get("oracle_kind", "live")
    primary = {
        "synthetic": "invariant",
        "java_harness": "live_runtime",
    }.get(str(oracle_kind), "differential")
    return {"primary": primary, "evidence": [primary]}


def provenance_from_scenario_fixture(fixture: dict[str, Any]) -> dict[str, Any]:
    provenance = fixture.get("provenance")
    if isinstance(provenance, dict):
        return dict(provenance)
    return {}


def scenario_fixture_to_combat_case_document(fixture: dict[str, Any]) -> dict[str, Any]:
    initial_game_state = fixture.get("initial_game_state")
    if not isinstance(initial_game_state, dict):
        raise SystemExit("scenario fixture missing initial_game_state object")
    combat_truth = initial_game_state.get("combat_truth")
    combat_observation = initial_game_state.get("combat_observation")
    if combat_truth is None or combat_observation is None:
        raise SystemExit("scenario fixture initial_game_state missing combat_truth/combat_observation")

    root_meta: dict[str, Any] = {}
    mapping = {
        "class": "player_class",
        "ascension_level": "ascension_level",
        "seed": "seed_hint",
        "screen_type": "screen_type",
        "screen_state": "screen_state",
    }
    for src, dst in mapping.items():
        value = initial_game_state.get(src)
        if value is not None:
            root_meta[dst] = value

    return {
        "id": fixture.get("name", "combat_case"),
        "domain": "combat",
        "basis": {
            "kind": "protocol_snapshot",
            "combat_truth": combat_truth,
            "combat_observation": combat_observation,
            "relics": initial_game_state.get("relics", []),
            "protocol_meta": fixture.get("initial_protocol_meta"),
            "root_meta": root_meta,
        },
        "delta": {},
        "program": [
            scenario_step_to_case_step(step)
            for step in fixture.get("steps", [])
            if isinstance(step, dict)
        ],
        "oracle": oracle_from_scenario_fixture(fixture),
        "expectations": [
            scenario_assertion_to_case_expectation(assertion)
            for assertion in fixture.get("assertions", [])
            if isinstance(assertion, dict)
        ],
        "provenance": provenance_from_scenario_fixture(fixture),
        "tags": list(fixture.get("tags", [])),
    }


def fixture_response_range(
    fixture: dict[str, Any],
    *,
    fallback_from_response_id: int,
    fallback_to_response_id: int,
) -> tuple[int, int]:
    provenance = fixture.get("provenance")
    if isinstance(provenance, dict):
        value = provenance.get("response_id_range")
        if (
            isinstance(value, list | tuple)
            and len(value) == 2
            and isinstance(value[0], int)
            and isinstance(value[1], int)
        ):
            return int(value[0]), int(value[1])
    return fallback_from_response_id, fallback_to_response_id


def scenario_fixture_to_live_window_case_document(
    fixture: dict[str, Any],
    *,
    raw_path: Path,
    debug_path: Path,
    fallback_from_response_id: int,
    fallback_to_response_id: int,
    failure_frame: int | None,
    target_field: str,
) -> dict[str, Any]:
    from_response_id, to_response_id = fixture_response_range(
        fixture,
        fallback_from_response_id=fallback_from_response_id,
        fallback_to_response_id=fallback_to_response_id,
    )
    case_doc = scenario_fixture_to_combat_case_document(fixture)
    case_doc["basis"] = {
        "kind": "live_window",
        "raw_path": str(raw_path),
        "debug_path": str(debug_path),
        "from_response_id": from_response_id,
        "to_response_id": to_response_id,
        "failure_frame": failure_frame,
        "target_field": target_field,
    }
    provenance = dict(case_doc.get("provenance", {}))
    provenance.setdefault("source", "live_comm")
    provenance.setdefault("source_path", str(raw_path))
    provenance.setdefault("response_id_range", [from_response_id, to_response_id])
    if failure_frame is not None:
        provenance.setdefault("failure_frame", failure_frame)
    notes = list(provenance.get("notes", []))
    notes.append("materialize with `cargo run --bin combat_case -- reduce --case <witness> --out <case>`")
    provenance["notes"] = notes
    case_doc["provenance"] = provenance
    return case_doc


def write_live_window_case_from_fixture(
    *,
    fixture_path: Path,
    case_path: Path,
    raw_path: Path,
    debug_path: Path,
    fallback_from_response_id: int,
    fallback_to_response_id: int,
    failure_frame: int | None,
    target_field: str,
) -> None:
    fixture = json.loads(fixture_path.read_text(encoding="utf-8"))
    case_doc = scenario_fixture_to_live_window_case_document(
        fixture,
        raw_path=raw_path,
        debug_path=debug_path,
        fallback_from_response_id=fallback_from_response_id,
        fallback_to_response_id=fallback_to_response_id,
        failure_frame=failure_frame,
        target_field=target_field,
    )
    case_path.write_text(json.dumps(case_doc, indent=2), encoding="utf-8")


def reduce_combat_case(
    *,
    repo_root: Path,
    in_path: Path,
    out_path: Path,
) -> None:
    run_checked(
        [
            "cargo",
            "run",
            "--quiet",
            "--bin",
            "combat_case",
            "--",
            "reduce",
            "--case",
            str(in_path),
            "--out",
            str(out_path),
        ],
        repo_root,
    )


def load_jsonl_rows(path: Path) -> list[dict[str, Any]]:
    rows: list[dict[str, Any]] = []
    with path.open("r", encoding="utf-8") as fh:
        for line in fh:
            line = line.strip()
            if not line:
                continue
            row = json.loads(line)
            if isinstance(row, dict):
                rows.append(row)
    return rows


def snapshot_selector_label(args: argparse.Namespace) -> str:
    if args.snapshot_id:
        return f"snapshot_id={args.snapshot_id}"
    if args.response_id is not None:
        return f"response_id={args.response_id}"
    if args.frame is not None:
        return f"frame={args.frame}"
    raise SystemExit("expected snapshot selector")


def select_failure_snapshot(
    snapshots: list[dict[str, Any]],
    *,
    snapshot_id: str | None,
    response_id: int | None,
    frame: int | None,
) -> dict[str, Any]:
    matches: list[dict[str, Any]] = []
    for snapshot in snapshots:
        if snapshot_id and snapshot.get("snapshot_id") == snapshot_id:
            matches.append(snapshot)
            continue
        if response_id is not None and snapshot.get("response_id") == response_id:
            matches.append(snapshot)
            continue
        if frame is not None and snapshot.get("frame") == frame:
            matches.append(snapshot)

    if not matches:
        raise SystemExit("no failure snapshot matched the requested selector")
    if len(matches) > 1:
        candidates = ", ".join(
            str(snapshot.get("snapshot_id", "<missing>")) for snapshot in matches
        )
        raise SystemExit(
            f"snapshot selector is ambiguous; matched {len(matches)} rows: {candidates}"
        )
    return matches[0]


def diff_strings_from_snapshot(snapshot: dict[str, Any]) -> list[str]:
    context = snapshot.get("decision_context")
    if not isinstance(context, dict):
        return []
    diffs = context.get("diffs")
    if not isinstance(diffs, list):
        return []
    return [str(diff) for diff in diffs]


def infer_field_from_diff(diff: str) -> str:
    match = SNAPSHOT_DIFF_RE.match(diff.strip())
    if match:
        return match.group("field").strip()
    field, _, _rest = diff.partition(" [")
    if field.strip():
        return field.strip()
    raise SystemExit(f"could not infer field from diff: {diff!r}")


def infer_field_from_snapshot(snapshot: dict[str, Any], diff_index: int) -> tuple[str, str]:
    diffs = diff_strings_from_snapshot(snapshot)
    if not diffs:
        raise SystemExit("selected failure snapshot does not contain decision_context.diffs")
    if diff_index < 0 or diff_index >= len(diffs):
        raise SystemExit(
            f"diff_index {diff_index} is out of range for snapshot with {len(diffs)} diff(s)"
        )
    chosen_diff = diffs[diff_index]
    return infer_field_from_diff(chosen_diff), chosen_diff


def run_live_workflow(
    *,
    repo_root: Path,
    out_dir: Path,
    raw_path: Path,
    debug_path: Path,
    field: str,
    from_response_id: int,
    to_response_id: int,
    name: str | None,
    mode: str,
    failure_frame: int | None = None,
) -> tuple[Path, Path, Path, str, Path, Path]:
    repo_root = repo_root.resolve()
    out_dir = out_dir.resolve()
    out_dir.mkdir(parents=True, exist_ok=True)

    live_regression = repo_root / "tools" / "analysis" / "live_regression.py"
    slug = name or f"{slugify(field)}_r{from_response_id}_{to_response_id}"
    extracted_path = out_dir / f"{slug}.json"
    minimized_path = out_dir / f"{slug}.min.json"
    extracted_witness_case_path = out_dir / f"{slug}.case.witness.json"
    minimized_witness_case_path = out_dir / f"{slug}.case.min.witness.json"
    extracted_case_path = out_dir / f"{slug}.case.json"
    minimized_case_path = out_dir / f"{slug}.case.min.json"
    notes_path = out_dir / f"{slug}.notes.md"

    common = [
        sys.executable,
        str(live_regression),
    ]

    chosen_field: str | None = None
    last_error: subprocess.CompletedProcess[str] | None = None
    for candidate_field in field_fallbacks(field):
        extract_cmd = common + [
            "extract",
            "--raw-path",
            str(raw_path),
            "--debug-path",
            str(debug_path),
            "--from-response-id",
            str(from_response_id),
            "--to-response-id",
            str(to_response_id),
            "--field",
            candidate_field,
            "--out",
            str(extracted_path),
        ]
        if failure_frame is not None:
            extract_cmd.extend(["--failure-frame", str(failure_frame)])
        result = run_capture(extract_cmd, repo_root)
        if result.stdout:
            print(result.stdout, end="")
        if result.returncode == 0:
            if result.stderr:
                print(result.stderr, end="", file=sys.stderr)
            chosen_field = candidate_field
            break
        last_error = result

    if chosen_field is None:
        assert last_error is not None
        if last_error.stdout:
            print(last_error.stdout, end="")
        if last_error.stderr:
            print(last_error.stderr, end="", file=sys.stderr)
        raise SystemExit(last_error.returncode)

    minimize_cmd = common + [
        "minimize",
        "--fixture",
        str(extracted_path),
        "--raw-path",
        str(raw_path),
        "--debug-path",
        str(debug_path),
        "--repo-root",
        str(repo_root),
        "--field",
        chosen_field,
        "--mode",
        mode,
        "--out",
        str(minimized_path),
    ]
    if failure_frame is not None:
        minimize_cmd.extend(["--failure-frame", str(failure_frame)])
    run_checked(minimize_cmd, repo_root)

    write_notes_template(
        notes_path=notes_path,
        field=chosen_field,
        from_response_id=from_response_id,
        to_response_id=to_response_id,
        extracted_path=extracted_path,
        minimized_path=minimized_path,
        extracted_witness_case_path=extracted_witness_case_path,
        minimized_witness_case_path=minimized_witness_case_path,
        extracted_case_path=extracted_case_path,
        minimized_case_path=minimized_case_path,
        raw_path=raw_path,
        debug_path=debug_path,
    )

    write_live_window_case_from_fixture(
        fixture_path=extracted_path,
        case_path=extracted_witness_case_path,
        raw_path=raw_path,
        debug_path=debug_path,
        fallback_from_response_id=from_response_id,
        fallback_to_response_id=to_response_id,
        failure_frame=failure_frame,
        target_field=chosen_field,
    )
    write_live_window_case_from_fixture(
        fixture_path=minimized_path,
        case_path=minimized_witness_case_path,
        raw_path=raw_path,
        debug_path=debug_path,
        fallback_from_response_id=from_response_id,
        fallback_to_response_id=to_response_id,
        failure_frame=failure_frame,
        target_field=chosen_field,
    )
    reduce_combat_case(
        repo_root=repo_root,
        in_path=extracted_witness_case_path,
        out_path=extracted_case_path,
    )
    reduce_combat_case(
        repo_root=repo_root,
        in_path=minimized_witness_case_path,
        out_path=minimized_case_path,
    )

    return (
        extracted_path,
        minimized_path,
        extracted_witness_case_path,
        minimized_witness_case_path,
        notes_path,
        chosen_field,
        extracted_case_path,
        minimized_case_path,
    )


def write_notes_template(
    *,
    notes_path: Path,
    field: str,
    from_response_id: int,
    to_response_id: int,
    extracted_path: Path,
    minimized_path: Path,
    extracted_witness_case_path: Path,
    minimized_witness_case_path: Path,
    extracted_case_path: Path,
    minimized_case_path: Path,
    raw_path: Path,
    debug_path: Path,
) -> None:
    notes_path.write_text(
        f"""# Bugfix Note: {field}

## Scope
- Target field: `{field}`
- Response window: `{from_response_id}..{to_response_id}`
- Raw log: `{raw_path}`
- Debug log: `{debug_path}`
- Extracted fixture: `{extracted_path}`
- Minimized fixture: `{minimized_path}`
- Extracted witness: `{extracted_witness_case_path}`
- Minimized witness: `{minimized_witness_case_path}`
- Extracted reduced case: `{extracted_case_path}`
- Minimized reduced case: `{minimized_case_path}`

## Required Workflow
- [ ] Reproduce with minimized reduced case first
- [ ] Read Java source before editing Rust
- [ ] Write or update a targeted regression test before/with the fix
- [ ] Re-run targeted test, then `cargo test -q`
- [ ] Re-run `live_comm` after the fix

## Java Source To Inspect
- [ ] Primary class:
- [ ] Related powers / relics / actions:
- [ ] Hidden internal state involved:

## Rust Touchpoints
- [ ] state sync / snapshot build:
- [ ] engine action handler:
- [ ] content entity implementation:
- [ ] comparator / replay support:

## Root Cause
- Pending

## Regression Target
- Pending

## Notes
- This note was generated by `tools/analysis/bugfix_workflow.py`.
- Do not start broad edits from the raw log alone; use the minimized reduced case first.
""",
        encoding="utf-8",
    )


def cmd_from_live(args: argparse.Namespace) -> int:
    (
        extracted_path,
        minimized_path,
        extracted_witness_case_path,
        minimized_witness_case_path,
        notes_path,
        chosen_field,
        extracted_case_path,
        minimized_case_path,
    ) = run_live_workflow(
        repo_root=args.repo_root,
        out_dir=args.out_dir,
        raw_path=args.raw_path,
        debug_path=args.debug_path,
        field=args.field,
        from_response_id=args.from_response_id,
        to_response_id=args.to_response_id,
        name=args.name,
        mode=args.mode,
    )

    print()
    print("Bugfix workflow artifacts written:")
    print(f"- extracted: {extracted_path}")
    print(f"- minimized: {minimized_path}")
    print(f"- extracted witness: {extracted_witness_case_path}")
    print(f"- minimized witness: {minimized_witness_case_path}")
    print(f"- extracted case: {extracted_case_path}")
    print(f"- minimized case: {minimized_case_path}")
    print(f"- notes:     {notes_path}")
    print()
    print("Next steps:")
    print(f"1. Reproduce with: set COMBAT_CASE={minimized_case_path}")
    print("2. Fill the Java/Rust touchpoints in the notes file")
    print("3. Add a targeted regression before broad changes")
    return 0


def cmd_from_snapshot(args: argparse.Namespace) -> int:
    run_dir = args.run_dir.resolve()
    failure_snapshots_path = run_dir / "failure_snapshots.jsonl"
    raw_path = run_dir / "raw.jsonl"
    debug_path = run_dir / "debug.txt"
    if not failure_snapshots_path.exists():
        raise SystemExit(f"missing failure snapshots: {failure_snapshots_path}")
    if not raw_path.exists():
        raise SystemExit(f"missing raw log: {raw_path}")
    if not debug_path.exists():
        raise SystemExit(f"missing debug log: {debug_path}")

    snapshots = load_jsonl_rows(failure_snapshots_path)
    snapshot = select_failure_snapshot(
        snapshots,
        snapshot_id=args.snapshot_id,
        response_id=args.response_id,
        frame=args.frame,
    )

    field = args.field
    chosen_diff: str | None = None
    if not field:
        field, chosen_diff = infer_field_from_snapshot(snapshot, args.diff_index)

    response_id = snapshot.get("response_id")
    frame = snapshot.get("frame")
    if not isinstance(response_id, int):
        raise SystemExit("selected failure snapshot is missing an integer response_id")
    failure_frame = frame if isinstance(frame, int) else None
    from_response_id = args.from_response_id
    if from_response_id is None:
        from_response_id = max(1, response_id - max(args.window_lookback, 0))

    snapshot_label = str(snapshot.get("snapshot_id", "snapshot"))
    name = args.name or f"{slugify(field)}_{slugify(snapshot_label)}"

    print(
        f"selected {snapshot_selector_label(args)} -> "
        f"{snapshot_label} (response_id={response_id}, frame={frame})"
    )
    if chosen_diff is not None:
        print(f"inferred diff[{args.diff_index}]: {chosen_diff}")
    print(f"requested field: {field}")
    print(f"initial response window: {from_response_id}..{response_id}")

    (
        extracted_path,
        minimized_path,
        extracted_witness_case_path,
        minimized_witness_case_path,
        notes_path,
        chosen_field,
        extracted_case_path,
        minimized_case_path,
    ) = run_live_workflow(
        repo_root=args.repo_root,
        out_dir=args.out_dir,
        raw_path=raw_path,
        debug_path=debug_path,
        field=field,
        from_response_id=from_response_id,
        to_response_id=response_id,
        name=name,
        mode=args.mode,
        failure_frame=failure_frame,
    )

    print()
    print("Bugfix workflow artifacts written:")
    print(f"- extracted: {extracted_path}")
    print(f"- minimized: {minimized_path}")
    print(f"- extracted witness: {extracted_witness_case_path}")
    print(f"- minimized witness: {minimized_witness_case_path}")
    print(f"- extracted case: {extracted_case_path}")
    print(f"- minimized case: {minimized_case_path}")
    print(f"- notes:     {notes_path}")
    print()
    print("Next steps:")
    print(f"1. Reproduce with: set COMBAT_CASE={minimized_case_path}")
    print("2. Fill the Java/Rust touchpoints in the notes file")
    print("3. Add a targeted regression before broad changes")
    print()
    print("Resolved workflow inputs:")
    print(f"- snapshot:  {snapshot_label}")
    print(f"- field:     {chosen_field}")
    print(f"- raw:       {raw_path}")
    print(f"- debug:     {debug_path}")
    return 0


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        description="Opinionated bugfix workflow wrapper for live_comm parity issues."
    )
    sub = parser.add_subparsers(dest="command", required=True)

    from_live = sub.add_parser(
        "from-live",
        help="extract + minimize a field-specific live_comm bug and generate a notes template",
    )
    from_live.add_argument("--field", required=True, help="field to preserve, e.g. player.energy")
    from_live.add_argument("--from-response-id", type=int, required=True)
    from_live.add_argument("--to-response-id", type=int, required=True)
    from_live.add_argument(
        "--name",
        help="optional artifact stem; defaults to '<field>_r<from>_<to>'",
    )
    from_live.add_argument("--raw-path", type=Path, default=DEFAULT_RAW_PATH)
    from_live.add_argument("--debug-path", type=Path, default=DEFAULT_DEBUG_PATH)
    from_live.add_argument("--repo-root", type=Path, default=DEFAULT_REPO_ROOT)
    from_live.add_argument("--out-dir", type=Path, default=DEFAULT_OUT_DIR)
    from_live.add_argument(
        "--mode",
        choices=("same-outcome", "pass-only", "fail-only"),
        default="same-outcome",
        help="fixture minimization mode",
    )
    from_live.set_defaults(func=cmd_from_live)

    from_snapshot = sub.add_parser(
        "from-snapshot",
        help="extract + minimize a live_comm bug directly from a failure_snapshots row",
    )
    from_snapshot.add_argument(
        "--run-dir",
        type=Path,
        required=True,
        help="run directory containing raw.jsonl, debug.txt, and failure_snapshots.jsonl",
    )
    selector = from_snapshot.add_mutually_exclusive_group(required=True)
    selector.add_argument("--snapshot-id")
    selector.add_argument("--response-id", type=int)
    selector.add_argument("--frame", type=int)
    from_snapshot.add_argument(
        "--diff-index",
        type=int,
        default=0,
        help="which diff entry in decision_context.diffs to use when inferring the field",
    )
    from_snapshot.add_argument(
        "--field",
        help="optional explicit field override; skips diff-based inference",
    )
    from_snapshot.add_argument(
        "--from-response-id",
        type=int,
        help="optional manual response window start override",
    )
    from_snapshot.add_argument(
        "--window-lookback",
        type=int,
        default=DEFAULT_WINDOW_LOOKBACK,
        help="when inferring the response window, start at response_id - lookback",
    )
    from_snapshot.add_argument(
        "--name",
        help="optional artifact stem; defaults to '<field>_<snapshot_id>'",
    )
    from_snapshot.add_argument("--repo-root", type=Path, default=DEFAULT_REPO_ROOT)
    from_snapshot.add_argument("--out-dir", type=Path, default=DEFAULT_OUT_DIR)
    from_snapshot.add_argument(
        "--mode",
        choices=("same-outcome", "pass-only", "fail-only"),
        default="same-outcome",
        help="fixture minimization mode",
    )
    from_snapshot.set_defaults(func=cmd_from_snapshot)
    return parser


def main() -> int:
    parser = build_parser()
    args = parser.parse_args()
    return args.func(args)


if __name__ == "__main__":
    raise SystemExit(main())
