#!/usr/bin/env python3
"""Portfolio demo wrapper for the LLM full-run controller.

This script runs the safe demo sequence:

1. dry_run, to inspect the prompt without an API call;
2. mock, to smoke-test the controller loop and action validation;
3. openai_compatible, only when requested or when API credentials are present.

It writes controller JSON artifacts plus a short markdown report suitable for
human review. The report is descriptive only; it does not claim policy quality.
"""

from __future__ import annotations

import argparse
import datetime as dt
import json
import os
import subprocess
import sys
import textwrap
import uuid
from pathlib import Path
from typing import Any


REPO_ROOT = Path(__file__).resolve().parents[2]
CONTROLLER = REPO_ROOT / "tools" / "llm" / "llm_full_run_controller.py"
DEFAULT_OUT_DIR = REPO_ROOT / "tools" / "artifacts" / "runs"


def has_openai_compatible_config() -> bool:
    return bool(
        os.environ.get("LLM_API_KEY")
        or os.environ.get("OPENAI_API_KEY")
        or os.environ.get("DEEPSEEK_API_KEY")
    )


def compact(value: Any, *, width: int = 160) -> str:
    if value is None:
        return ""
    if not isinstance(value, str):
        value = json.dumps(value, ensure_ascii=False, separators=(",", ":"))
    value = " ".join(value.split())
    return textwrap.shorten(value, width=width, placeholder="...")


def utc_now() -> dt.datetime:
    return dt.datetime.now(dt.timezone.utc)


def make_run_id(args: argparse.Namespace) -> str:
    timestamp = utc_now().strftime("%Y%m%dT%H%M%SZ")
    suffix = uuid.uuid4().hex[:8]
    return f"{timestamp}_llm_demo_seed{args.seed}_{suffix}"


def atomic_write_text(path: Path, text: str) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    tmp = path.with_name(path.name + ".tmp")
    tmp.write_text(text, encoding="utf-8")
    tmp.replace(path)


def atomic_write_json(path: Path, value: Any) -> None:
    atomic_write_text(path, json.dumps(value, ensure_ascii=False, indent=2) + "\n")


def markdown_cell(value: Any, *, width: int = 120) -> str:
    text = compact(value, width=width)
    return text.replace("|", "\\|")


def path_for_report(path: Path) -> str:
    try:
        return path.resolve().relative_to(REPO_ROOT).as_posix()
    except ValueError:
        return str(path)


def path_relative_to(path: Path, root: Path) -> str:
    try:
        return path.resolve().relative_to(root.resolve()).as_posix()
    except ValueError:
        return path_for_report(path)


def run_controller(
    *,
    provider: str,
    out_path: Path,
    steps: int,
    args: argparse.Namespace,
) -> dict[str, Any]:
    if out_path.exists():
        out_path.unlink()
    command = [
        sys.executable,
        str(CONTROLLER),
        "--provider",
        provider,
        "--seed",
        str(args.seed),
        "--ascension",
        str(args.ascension),
        "--class",
        args.player_class,
        "--max-steps",
        str(args.max_steps),
        "--steps",
        str(steps),
        "--max-candidates",
        str(args.max_candidates),
        "--out",
        str(out_path),
        "--trace-level",
        "full",
    ]
    if args.final_act:
        command.append("--final-act")
    if provider == "openai_compatible":
        command.extend(
            [
                "--temperature",
                str(args.temperature),
                "--timeout",
                str(args.timeout),
            ]
        )
    completed = subprocess.run(
        command,
        cwd=REPO_ROOT,
        text=True,
        encoding="utf-8",
        capture_output=True,
        check=False,
    )
    parsed = None
    if completed.returncode == 0 and out_path.exists():
        try:
            parsed = json.loads(out_path.read_text(encoding="utf-8"))
        except json.JSONDecodeError:
            parsed = None
    return {
        "provider": provider,
        "out_path": out_path,
        "returncode": completed.returncode,
        "stdout_tail": compact(completed.stdout, width=600),
        "stderr_tail": compact(completed.stderr, width=600),
        "report": parsed,
    }


def summarize_info(info: Any) -> str:
    if not isinstance(info, dict):
        return ""
    keys = [
        "result",
        "floor",
        "act",
        "current_hp",
        "max_hp",
        "gold",
        "step_count",
    ]
    present = {key: info.get(key) for key in keys if key in info}
    if present:
        return compact(present, width=160)
    return compact(info, width=160)


def summarize_public_state(state: Any) -> str:
    if not isinstance(state, dict):
        return ""
    summary = {
        key: state.get(key)
        for key in [
            "act",
            "floor",
            "current_hp",
            "max_hp",
            "gold",
            "deck_size",
            "act_boss",
        ]
        if key in state
    }
    combat = state.get("combat")
    if isinstance(combat, dict):
        summary["combat"] = {
            key: combat.get(key)
            for key in [
                "player_hp",
                "player_block",
                "energy",
                "turn_count",
                "visible_incoming_damage",
                "total_monster_hp",
                "alive_monster_count",
            ]
            if key in combat
        }
    return compact(summary, width=220)


def final_outcome(records: list[dict[str, Any]]) -> str:
    if not records:
        return ""
    last = records[-1]
    info = last.get("info") if isinstance(last.get("info"), dict) else {}
    pre_info = last.get("pre_info") if isinstance(last.get("pre_info"), dict) else {}
    public_state = (
        last.get("public_state_before")
        if isinstance(last.get("public_state_before"), dict)
        else {}
    )
    outcome: dict[str, Any] = {}
    for key in [
        "result",
        "floor",
        "act",
        "current_hp",
        "max_hp",
        "gold",
        "step_count",
    ]:
        if key in info:
            outcome[key] = info.get(key)
        elif key in pre_info:
            outcome[key] = pre_info.get(key)
        elif key in public_state:
            outcome[key] = public_state.get(key)
    if "combat" in public_state:
        outcome["last_public_combat"] = public_state.get("combat")
    outcome["last_decision_type"] = last.get("decision_type")
    outcome["last_action"] = last.get("selected_action_key")
    outcome["last_reason"] = (last.get("llm_choice") or {}).get("reason")
    outcome["done"] = last.get("done")
    return compact(outcome, width=420)


def record_rows(records: list[dict[str, Any]]) -> list[str]:
    rows = [
        "| step | decision | public state before | candidates | selected action | legal | reward | done | reason |",
        "| --- | --- | --- | ---: | --- | --- | ---: | --- | --- |",
    ]
    for record in records:
        choice = record.get("llm_choice") or {}
        rows.append(
            "| "
            + " | ".join(
                [
                    markdown_cell(record.get("step_index")),
                    markdown_cell(record.get("decision_type")),
                    markdown_cell(
                        summarize_public_state(record.get("public_state_before")),
                        width=260,
                    ),
                    markdown_cell(record.get("candidate_count")),
                    markdown_cell(record.get("selected_action_key")),
                    markdown_cell(record.get("choice_was_legal")),
                    markdown_cell(record.get("reward")),
                    markdown_cell(record.get("done")),
                    markdown_cell(choice.get("reason")),
                ]
            )
            + " |"
        )
    return rows


def dry_run_section(result: dict[str, Any]) -> list[str]:
    if result.get("skipped"):
        return skipped_section(result)
    report = result.get("report") or {}
    records = report.get("records") or []
    failed = result["returncode"] != 0
    lines = [
        f"### {result['provider']}",
        "",
        f"- status: `exit {result['returncode']}`",
        f"- result: `{'failed' if failed else 'ok'}`",
        f"- json: `{path_for_report(result['out_path'])}`",
    ]
    if result.get("stderr_tail"):
        lines.append(f"- stderr: `{markdown_cell(result['stderr_tail'], width=500)}`")
    if failed:
        lines.extend(
            [
                "",
                "The controller exited non-zero. This report intentionally did not",
                "read JSON output for this provider, so stale artifacts cannot be",
                "reported as current results.",
            ]
        )
        return lines
    if records:
        first = records[0]
        lines.extend(
            [
                f"- candidate count: `{first.get('candidate_count')}`",
                "",
                "Prompt preview:",
                "",
                "```text",
                compact(first.get("prompt"), width=1600),
                "```",
            ]
        )
    return lines


def run_section(result: dict[str, Any]) -> list[str]:
    if result.get("skipped"):
        return skipped_section(result)
    report = result.get("report") or {}
    records = report.get("records") or []
    failed = result["returncode"] != 0
    lines = [
        f"### {result['provider']}",
        "",
        f"- status: `exit {result['returncode']}`",
        f"- result: `{'failed' if failed else 'ok'}`",
        f"- json: `{path_for_report(result['out_path'])}`",
    ]
    if report.get("model"):
        lines.append(f"- model: `{report.get('model')}`")
    if result.get("stderr_tail"):
        lines.append(f"- stderr: `{markdown_cell(result['stderr_tail'], width=500)}`")
    if failed:
        lines.extend(
            [
                "",
                "The controller exited non-zero. This report intentionally did not",
                "read JSON output for this provider, so stale artifacts cannot be",
                "reported as current results.",
            ]
        )
        return lines
    if not records:
        lines.extend(["", "No records were produced."])
        return lines
    illegal = sum(1 for record in records if record.get("choice_was_legal") is False)
    done = any(bool(record.get("done")) for record in records)
    lines.extend(
        [
            f"- records: `{len(records)}`",
            f"- illegal choices after validation: `{illegal}`",
            f"- run ended: `{done}`",
            f"- final state: `{markdown_cell(final_outcome(records), width=500)}`",
            "",
            *record_rows(records),
        ]
    )
    return lines


def skipped_section(result: dict[str, Any]) -> list[str]:
    return [
        f"### {result['provider']}",
        "",
        "- status: `skipped`",
        f"- reason: `{markdown_cell(result.get('skip_reason'), width=240)}`",
    ]


def build_report(
    results: list[dict[str, Any]],
    args: argparse.Namespace,
    *,
    run_id: str,
    run_dir: Path,
    manifest_path: Path,
    status_path: Path,
) -> str:
    now = dt.datetime.now().astimezone().isoformat(timespec="seconds")
    lines = [
        "---",
        "schema_name: HumanReadableReport",
        "schema_version: 1",
        "artifact_role: human_readable_view",
        f"run_id: {run_id}",
        "source_artifacts:",
        f"  - {path_relative_to(manifest_path, run_dir)}",
        f"  - {path_relative_to(status_path, run_dir)}",
        "descriptive_only: true",
        "not_policy_evaluation: true",
        "not_training_data: true",
        "---",
        "",
        "# LLM Controller Demo Report",
        "",
        f"Generated: `{now}`",
        f"Run id: `{run_id}`",
        "",
        "This report demonstrates the public-observation controller loop. It is",
        "not a gameplay-strength claim and does not treat LLM choices as teacher",
        "labels.",
        "",
        "## Run Configuration",
        "",
        f"- seed: `{args.seed}`",
        f"- ascension: `{args.ascension}`",
        f"- class: `{args.player_class}`",
        f"- final act: `{args.final_act}`",
        f"- mock / LLM steps: `{args.steps}`",
        f"- max candidates in prompt: `{args.max_candidates}`",
        "",
        "## Contract",
        "",
        "- The controller reads only public `decision_env_observation` payloads.",
        "- The prompt includes legal candidate ids and action keys.",
        "- The model must return strict JSON with `action_id`, `confidence`, and `reason`.",
        "- The harness validates `action_id` before stepping the simulator.",
        "",
        "## Runs",
        "",
    ]
    for result in results:
        if result["provider"] == "dry_run":
            lines.extend(dry_run_section(result))
        else:
            lines.extend(run_section(result))
        lines.append("")
    return "\n".join(lines).rstrip() + "\n"


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--seed", type=int, default=42)
    parser.add_argument("--ascension", type=int, default=0)
    parser.add_argument("--final-act", action="store_true")
    parser.add_argument("--class", dest="player_class", default="ironclad")
    parser.add_argument("--max-steps", type=int, default=500)
    parser.add_argument("--steps", type=int, default=5)
    parser.add_argument("--max-candidates", type=int, default=24)
    parser.add_argument("--out-dir", type=Path, default=DEFAULT_OUT_DIR)
    parser.add_argument("--run-id", default=None)
    parser.add_argument(
        "--openai-compatible",
        choices=["auto", "always", "never"],
        default="auto",
        help="auto runs only when API credentials are present",
    )
    parser.add_argument("--temperature", type=float, default=0.0)
    parser.add_argument("--timeout", type=int, default=60)
    parser.add_argument(
        "--report",
        type=Path,
        default=None,
        help="markdown report path; defaults to OUT_DIR/llm_demo_report.md",
    )
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    run_id = args.run_id or make_run_id(args)
    run_dir = args.out_dir / run_id
    raw_dir = run_dir / "raw"
    reports_dir = run_dir / "reports"
    inputs_dir = run_dir / "inputs"
    manifest_path = run_dir / "manifest.json"
    status_path = run_dir / "status.json"
    report_path = args.report or reports_dir / "demo_report.md"

    raw_dir.mkdir(parents=True, exist_ok=True)
    reports_dir.mkdir(parents=True, exist_ok=True)
    inputs_dir.mkdir(parents=True, exist_ok=True)

    run_config = {
        "schema_name": "RunConfig",
        "schema_version": 1,
        "run_id": run_id,
        "seed": args.seed,
        "ascension": args.ascension,
        "class": args.player_class,
        "final_act": args.final_act,
        "max_steps": args.max_steps,
        "steps": args.steps,
        "max_candidates": args.max_candidates,
        "openai_compatible": args.openai_compatible,
        "temperature": args.temperature,
        "timeout": args.timeout,
    }
    atomic_write_json(inputs_dir / "run_config.json", run_config)
    atomic_write_json(
        status_path,
        {
            "schema_name": "RunStatus",
            "schema_version": 1,
            "run_id": run_id,
            "state": "running",
            "report_generated": False,
        },
    )

    results = [
        run_controller(
            provider="dry_run",
            out_path=raw_dir / "dry_run.json",
            steps=1,
            args=args,
        ),
        run_controller(
            provider="mock",
            out_path=raw_dir / "mock_run.json",
            steps=args.steps,
            args=args,
        ),
    ]

    should_run_llm = args.openai_compatible == "always" or (
        args.openai_compatible == "auto" and has_openai_compatible_config()
    )
    if should_run_llm:
        results.append(
            run_controller(
                provider="openai_compatible",
                out_path=raw_dir / "llm_run.json",
                steps=args.steps,
                args=args,
            )
        )
    else:
        results.append(
            {
                "provider": "openai_compatible",
                "out_path": raw_dir / "llm_run.json",
                "returncode": None,
                "skipped": True,
                "skip_reason": "no API key configured and --openai-compatible auto",
                "report": None,
            }
        )

    failed = any(
        result.get("returncode") not in (0, None)
        for result in results
        if not result.get("skipped")
    )
    status = {
        "schema_name": "RunStatus",
        "schema_version": 1,
        "run_id": run_id,
        "state": "failed" if failed else "completed",
        "provider_status": [
            {
                "provider": result["provider"],
                "state": "skipped"
                if result.get("skipped")
                else ("ok" if result.get("returncode") == 0 else "failed"),
                "returncode": result.get("returncode"),
                "artifact": path_relative_to(result["out_path"], run_dir),
            }
            for result in results
        ],
        "report_generated": True,
    }

    report = build_report(
        results,
        args,
        run_id=run_id,
        run_dir=run_dir,
        manifest_path=manifest_path,
        status_path=status_path,
    )
    atomic_write_text(report_path, report)

    manifest = {
        "schema_name": "RunManifest",
        "schema_version": 1,
        "artifact_role": "llm_controller_demo_run",
        "run_id": run_id,
        "created_at_utc": utc_now().isoformat(timespec="seconds").replace("+00:00", "Z"),
        "purpose": "llm_controller_descriptive_demo",
        "claim_level": "demo_only",
        "information_boundary": "public_observation_only",
        "label_role": "not_a_label",
        "trainable_as_action_label": False,
        "eligible_uses": ["demo_report", "behavior_audit", "replay_debug"],
        "forbidden_uses": [
            "teacher_label",
            "supervised_policy_training",
            "policy_quality_claim",
        ],
        "producer": {
            "component": "tools/llm/run_llm_demo.py",
            "schema_contract": "minimal_run_manifest_v1",
        },
        "code": {
            "repo": "sts_simulator",
            "git_commit": None,
            "git_dirty": None,
            "note": "git metadata intentionally not collected by this wrapper",
        },
        "inputs": {
            "run_config": path_relative_to(inputs_dir / "run_config.json", run_dir),
        },
        "outputs": {
            "dry_run": path_relative_to(raw_dir / "dry_run.json", run_dir),
            "mock_run": path_relative_to(raw_dir / "mock_run.json", run_dir),
            "llm_run": path_relative_to(raw_dir / "llm_run.json", run_dir),
            "report": path_relative_to(report_path, run_dir),
            "status": path_relative_to(status_path, run_dir),
        },
        "status": status,
    }

    atomic_write_json(manifest_path, manifest)
    atomic_write_json(status_path, status)
    print(f"run_id {run_id}")
    print(f"run_dir {run_dir}")
    for result in results:
        if result.get("skipped"):
            print(f"{result['provider']}: skipped ({result.get('skip_reason')})")
        else:
            print(f"{result['provider']}: exit {result.get('returncode')}")
    print(f"wrote {report_path}")

    return 1 if failed else 0


if __name__ == "__main__":
    raise SystemExit(main())
