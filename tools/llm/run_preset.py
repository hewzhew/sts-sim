#!/usr/bin/env python3
"""Preset runner for llm_full_run_controller.py.

This is intentionally a small local experiment-contract runner, not Hydra,
Sacred, or W&B. It reads a YAML preset, applies explicit overrides, creates an
immutable run directory, records the resolved config and command, then runs the
existing controller CLI unchanged.
"""

from __future__ import annotations

import argparse
import json
import os
import re
import subprocess
import sys
import threading
import time
from pathlib import Path
from typing import Any


REPO_ROOT = Path(__file__).resolve().parents[2]
LLM_DIR = Path(__file__).resolve().parent
PRESET_ROOT = LLM_DIR / "presets"
RUN_ROOT = REPO_ROOT / "tools" / "artifacts" / "runs"


class PresetError(RuntimeError):
    pass


def strip_comment(line: str) -> str:
    in_single = False
    in_double = False
    for index, char in enumerate(line):
        if char == "'" and not in_double:
            in_single = not in_single
        elif char == '"' and not in_single:
            in_double = not in_double
        elif char == "#" and not in_single and not in_double:
            if index == 0 or line[index - 1].isspace():
                return line[:index].rstrip()
    return line.rstrip()


def parse_scalar(value: str) -> Any:
    value = value.strip()
    if value == "":
        return ""
    if value in {"null", "Null", "NULL", "~"}:
        return None
    if value in {"true", "True", "TRUE"}:
        return True
    if value in {"false", "False", "FALSE"}:
        return False
    if value == "[]":
        return []
    if value == "{}":
        return {}
    if value.startswith("[") and value.endswith("]"):
        inner = value[1:-1].strip()
        if not inner:
            return []
        return [parse_scalar(part.strip()) for part in inner.split(",")]
    if (
        (value.startswith('"') and value.endswith('"'))
        or (value.startswith("'") and value.endswith("'"))
    ):
        return value[1:-1]
    if re.fullmatch(r"-?\d+", value):
        try:
            return int(value)
        except ValueError:
            pass
    if re.fullmatch(r"-?\d+\.\d+", value):
        try:
            return float(value)
        except ValueError:
            pass
    return value


def line_indent(line: str) -> int:
    return len(line) - len(line.lstrip(" "))


def next_content_line(lines: list[str], index: int) -> tuple[int, str] | None:
    while index < len(lines):
        raw = strip_comment(lines[index])
        if raw.strip():
            return index, raw
        index += 1
    return None


def collect_block_scalar(
    lines: list[str],
    index: int,
    parent_indent: int,
    style: str,
) -> tuple[str, int]:
    collected: list[str] = []
    min_indent: int | None = None
    cursor = index
    while cursor < len(lines):
        raw = lines[cursor].rstrip("\n")
        if raw.strip():
            indent = line_indent(raw)
            if indent <= parent_indent:
                break
            min_indent = indent if min_indent is None else min(min_indent, indent)
        collected.append(raw)
        cursor += 1
    if min_indent is None:
        return "", cursor
    normalized = [
        line[min_indent:] if len(line) >= min_indent else ""
        for line in collected
    ]
    if style == "|":
        return "\n".join(normalized).rstrip("\n"), cursor
    paragraphs: list[str] = []
    current: list[str] = []
    for line in normalized:
        if line.strip():
            current.append(line.strip())
        else:
            if current:
                paragraphs.append(" ".join(current))
                current = []
            paragraphs.append("")
    if current:
        paragraphs.append(" ".join(current))
    return "\n".join(paragraphs).strip(), cursor


def parse_yaml_block(lines: list[str], index: int, indent: int) -> tuple[Any, int]:
    probe = next_content_line(lines, index)
    if probe is None:
        return {}, index
    _, first = probe
    if line_indent(first) < indent:
        return {}, index
    is_list = first.lstrip().startswith("- ") and line_indent(first) == indent
    if is_list:
        items: list[Any] = []
        cursor = index
        while cursor < len(lines):
            raw = strip_comment(lines[cursor])
            if not raw.strip():
                cursor += 1
                continue
            current_indent = line_indent(raw)
            if current_indent < indent:
                break
            if current_indent > indent:
                raise PresetError(f"unexpected nested list indentation near: {raw}")
            stripped = raw.lstrip()
            if not stripped.startswith("- "):
                break
            rest = stripped[2:].strip()
            if not rest:
                item, cursor = parse_yaml_block(lines, cursor + 1, indent + 2)
                items.append(item)
                continue
            if ":" in rest and not rest.startswith(("'", '"')):
                key, value = rest.split(":", 1)
                item: dict[str, Any] = {key.strip(): parse_scalar(value.strip())}
                cursor += 1
                items.append(item)
            else:
                items.append(parse_scalar(rest))
                cursor += 1
        return items, cursor

    mapping: dict[str, Any] = {}
    cursor = index
    while cursor < len(lines):
        raw = strip_comment(lines[cursor])
        if not raw.strip():
            cursor += 1
            continue
        current_indent = line_indent(raw)
        if current_indent < indent:
            break
        if current_indent > indent:
            raise PresetError(f"unexpected indentation near: {raw}")
        stripped = raw.strip()
        if ":" not in stripped:
            raise PresetError(f"expected key: value line, got: {raw}")
        key, rest = stripped.split(":", 1)
        key = key.strip()
        rest = rest.strip()
        cursor += 1
        if rest in {">", "|"}:
            value, cursor = collect_block_scalar(lines, cursor, current_indent, rest)
            mapping[key] = value
        elif rest:
            mapping[key] = parse_scalar(rest)
        else:
            nested_probe = next_content_line(lines, cursor)
            if nested_probe is None or line_indent(nested_probe[1]) <= current_indent:
                mapping[key] = {}
            else:
                mapping[key], cursor = parse_yaml_block(
                    lines,
                    cursor,
                    line_indent(nested_probe[1]),
                )
    return mapping, cursor


def load_yaml(path: Path) -> dict[str, Any]:
    text = path.read_text(encoding="utf-8")
    value, index = parse_yaml_block(text.splitlines(), 0, 0)
    if not isinstance(value, dict):
        raise PresetError(f"top-level YAML must be a mapping: {path}")
    trailing = next_content_line(text.splitlines(), index)
    if trailing is not None:
        raise PresetError(f"could not parse complete YAML near line {trailing[0] + 1}")
    return value


def yaml_scalar(value: Any) -> str:
    if value is None:
        return "null"
    if value is True:
        return "true"
    if value is False:
        return "false"
    if isinstance(value, (int, float)):
        return str(value)
    text = str(value)
    if text == "":
        return '""'
    if re.fullmatch(r"[A-Za-z0-9_./:\\-]+", text):
        return text
    return json.dumps(text, ensure_ascii=False)


def dump_yaml(value: Any, indent: int = 0) -> str:
    pad = " " * indent
    if isinstance(value, dict):
        lines: list[str] = []
        for key, item in value.items():
            if isinstance(item, dict) and not item:
                lines.append(f"{pad}{key}: {{}}")
            elif isinstance(item, list) and not item:
                lines.append(f"{pad}{key}: []")
            elif isinstance(item, (dict, list)):
                lines.append(f"{pad}{key}:")
                nested = dump_yaml(item, indent + 2)
                if nested:
                    lines.append(nested)
            elif isinstance(item, str) and ("\n" in item or len(item) > 100):
                lines.append(f"{pad}{key}: >")
                for block_line in item.splitlines() or [""]:
                    lines.append(" " * (indent + 2) + block_line)
            else:
                lines.append(f"{pad}{key}: {yaml_scalar(item)}")
        return "\n".join(lines)
    if isinstance(value, list):
        if not value:
            return f"{pad}[]"
        lines = []
        for item in value:
            if isinstance(item, (dict, list)):
                lines.append(f"{pad}-")
                lines.append(dump_yaml(item, indent + 2))
            else:
                lines.append(f"{pad}- {yaml_scalar(item)}")
        return "\n".join(lines)
    return f"{pad}{yaml_scalar(value)}"


def deep_copy(value: Any) -> Any:
    return json.loads(json.dumps(value, ensure_ascii=False))


def parse_override_value(value: str) -> Any:
    return parse_scalar(value)


def set_deep(config: dict[str, Any], path: str, value: Any) -> None:
    parts = path.split(".")
    cursor: Any = config
    for part in parts[:-1]:
        if not isinstance(cursor, dict):
            raise PresetError(f"cannot set {path}: {part} is not a mapping")
        cursor = cursor.setdefault(part, {})
    if not isinstance(cursor, dict):
        raise PresetError(f"cannot set {path}: parent is not a mapping")
    cursor[parts[-1]] = value


def resolve_override_path(config: dict[str, Any], raw_key: str) -> str:
    if "." in raw_key:
        return raw_key
    matches = []
    for section in ("args", "limits", "claims", "artifacts"):
        value = config.get(section)
        if isinstance(value, dict) and raw_key in value:
            matches.append(f"{section}.{raw_key}")
    if len(matches) == 1:
        return matches[0]
    if len(matches) > 1:
        raise PresetError(
            f"ambiguous override '{raw_key}', use one of: {', '.join(matches)}"
        )
    return f"args.{raw_key}"


def apply_overrides(config: dict[str, Any], overrides: list[str]) -> None:
    for override in overrides:
        if "=" not in override:
            raise PresetError(f"--set must use key=value, got: {override}")
        key, raw_value = override.split("=", 1)
        path = resolve_override_path(config, key.strip())
        set_deep(config, path, parse_override_value(raw_value.strip()))


def preset_path(name: str) -> Path:
    direct = Path(name)
    if direct.suffix in {".yaml", ".yml"}:
        candidate = direct if direct.is_absolute() else REPO_ROOT / direct
        if candidate.exists():
            return candidate
    normalized = name.replace("\\", "/").strip("/")
    candidate = PRESET_ROOT / f"{normalized}.yaml"
    if candidate.exists():
        return candidate
    raise PresetError(f"preset not found: {name} ({candidate})")


def preset_slug(name: str) -> str:
    return re.sub(r"[^A-Za-z0-9_]+", "_", name.replace("\\", "/")).strip("_")


def unique_run_dir(config: dict[str, Any], preset_name: str) -> Path:
    artifacts = ensure_mapping(config, "artifacts")
    configured = artifacts.get("run_dir", "auto")
    if configured and configured != "auto":
        path = Path(str(configured))
        return path if path.is_absolute() else REPO_ROOT / path
    stamp = time.strftime("%Y%m%d_%H%M%S")
    base = RUN_ROOT / f"{stamp}_{preset_slug(preset_name)}"
    if not base.exists():
        return base
    for index in range(2, 1000):
        candidate = RUN_ROOT / f"{stamp}_{preset_slug(preset_name)}_{index}"
        if not candidate.exists():
            return candidate
    raise PresetError("could not allocate unique run directory")


def ensure_mapping(config: dict[str, Any], key: str) -> dict[str, Any]:
    value = config.get(key)
    if not isinstance(value, dict):
        raise PresetError(f"preset field '{key}' must be a mapping")
    return value


def ensure_list(config: dict[str, Any], key: str) -> list[Any]:
    value = config.get(key)
    if value is None:
        return []
    if not isinstance(value, list):
        raise PresetError(f"preset field '{key}' must be a list")
    return value


def validate_config(config: dict[str, Any]) -> None:
    for key in ("name", "purpose", "script", "args", "limits", "claims", "artifacts"):
        if key not in config:
            raise PresetError(f"missing required preset field: {key}")
    purpose = config["purpose"]
    if purpose not in {"smoke", "watch", "eval", "replay"}:
        raise PresetError("purpose must be one of: smoke, watch, eval, replay")
    args = ensure_mapping(config, "args")
    limits = ensure_mapping(config, "limits")
    claims = ensure_mapping(config, "claims")
    if claims.get("benchmark_claim") is not False:
        raise PresetError("claims.benchmark_claim must be false")
    if claims.get("policy_quality_claim") is not False:
        raise PresetError("claims.policy_quality_claim must be false")
    if purpose == "watch":
        if "max_llm_calls" not in limits:
            raise PresetError("purpose=watch requires limits.max_llm_calls")
        if limits.get("require_interactive_watch") and args.get("watch_decisions") is not True:
            raise PresetError(
                "purpose=watch with require_interactive_watch requires args.watch_decisions=true"
            )
    if purpose == "eval":
        if not config.get("hypothesis"):
            raise PresetError("purpose=eval requires hypothesis")
        if not ensure_list(config, "acceptance_criteria"):
            raise PresetError("purpose=eval requires non-empty acceptance_criteria")
        if "seed" not in args and "seed_suite" not in args:
            raise PresetError("purpose=eval requires args.seed or args.seed_suite")
    if purpose == "smoke":
        if int(limits.get("max_llm_calls", 0)) > 10 and not limits.get("allow_long_run"):
            raise PresetError("purpose=smoke requires max_llm_calls <= 10 unless allow_long_run=true")
    if purpose == "replay":
        if not (config.get("replay_source") or args.get("replay_source") or args.get("state_source")):
            raise PresetError("purpose=replay requires replay_source or state_source")


def resolve_artifacts(config: dict[str, Any], run_dir: Path) -> None:
    artifacts = ensure_mapping(config, "artifacts")
    args = ensure_mapping(config, "args")
    artifacts["run_dir"] = str(run_dir.relative_to(REPO_ROOT))
    defaults = {
        "events": "events.jsonl",
        "summary": "summary.json",
        "stdout": "stdout.log",
        "stderr": "stderr.log",
        "command": "command.txt",
        "resolved_config": "resolved_config.yaml",
        "act1_eval_summary": "act1_eval_summary.json",
    }
    for key, value in defaults.items():
        artifacts.setdefault(key, value)
    if args.get("out") == "auto":
        args["out"] = str((run_dir / str(artifacts["events"])).relative_to(REPO_ROOT))
    if args.get("act1_eval_out") == "auto":
        args["act1_eval_out"] = str(
            (run_dir / str(artifacts["act1_eval_summary"])).relative_to(REPO_ROOT)
        )


def flag_name(key: str) -> str:
    return "--" + key.replace("_", "-")


def command_from_config(config: dict[str, Any]) -> list[str]:
    script = Path(str(config["script"]))
    script_path = script if script.is_absolute() else REPO_ROOT / script
    if not script_path.exists():
        raise PresetError(f"script not found: {script_path}")
    try:
        script_arg = str(script_path.relative_to(REPO_ROOT))
    except ValueError:
        script_arg = str(script_path)
    command = [sys.executable, script_arg]
    args = ensure_mapping(config, "args")
    for key, value in args.items():
        if value is None or value is False:
            continue
        if value is True:
            command.append(flag_name(key))
        elif isinstance(value, list):
            for item in value:
                command.extend([flag_name(key), str(item)])
        else:
            command.extend([flag_name(key), str(value)])
    return command


def write_text(path: Path, text: str) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(text, encoding="utf-8")


def tee_stream(
    stream: Any,
    log_path: Path,
    console: Any,
    monitor: dict[str, Any],
    *,
    line_mode: bool = False,
) -> None:
    line_buffer: list[str] = []
    with log_path.open("w", encoding="utf-8", buffering=1) as log:
        if line_mode:
            while True:
                line = stream.readline()
                if line == "":
                    break
                log.write(line)
                console.write(line)
                console.flush()
                if "llm_call_start" not in line:
                    continue
                monitor["observed_llm_calls"] = int(
                    monitor.get("observed_llm_calls", 0)
                ) + 1
                max_llm_calls = monitor.get("max_llm_calls")
                proc = monitor.get("proc")
                if (
                    max_llm_calls is not None
                    and monitor["observed_llm_calls"] > int(max_llm_calls)
                    and not monitor.get("limit_exceeded")
                    and proc is not None
                ):
                    monitor["limit_exceeded"] = "max_llm_calls"
                    try:
                        proc.terminate()
                    except Exception:
                        pass
            return
        while True:
            chunk = stream.read(1)
            if chunk == "":
                break
            log.write(chunk)
            console.write(chunk)
            console.flush()
            line_buffer.append(chunk)
            if chunk == "\n":
                line = "".join(line_buffer)
                line_buffer = []
                if "llm_call_start" not in line:
                    continue
                monitor["observed_llm_calls"] = int(
                    monitor.get("observed_llm_calls", 0)
                ) + 1
                max_llm_calls = monitor.get("max_llm_calls")
                proc = monitor.get("proc")
                if (
                    max_llm_calls is not None
                    and monitor["observed_llm_calls"] > int(max_llm_calls)
                    and not monitor.get("limit_exceeded")
                    and proc is not None
                ):
                    monitor["limit_exceeded"] = "max_llm_calls"
                    try:
                        proc.terminate()
                    except Exception:
                        pass


def extract_first_json_object(text: str) -> dict[str, Any] | None:
    start = text.find("{")
    if start < 0:
        return None
    depth = 0
    in_string = False
    escape = False
    for index in range(start, len(text)):
        char = text[index]
        if in_string:
            if escape:
                escape = False
            elif char == "\\":
                escape = True
            elif char == '"':
                in_string = False
            continue
        if char == '"':
            in_string = True
        elif char == "{":
            depth += 1
        elif char == "}":
            depth -= 1
            if depth == 0:
                try:
                    value = json.loads(text[start : index + 1])
                except json.JSONDecodeError:
                    return None
                return value if isinstance(value, dict) else None
    return None


def print_summary(config: dict[str, Any], preset_ref: str, run_dir: Path) -> None:
    args = ensure_mapping(config, "args")
    limits = ensure_mapping(config, "limits")
    claims = ensure_mapping(config, "claims")
    print("Preset:", preset_ref)
    print("Purpose:", config.get("purpose"))
    print("Provider:", args.get("provider"))
    model = args.get("model")
    if args.get("provider") == "openai_compatible":
        model = model or os.environ.get("LLM_MODEL") or os.environ.get("OPENAI_MODEL")
    print("Model:", model)
    print("Run mode:", args.get("run_mode"))
    print("Seed:", args.get("seed"))
    print("Combat search:", args.get("combat_search_engine"))
    print("Watch decisions:", args.get("watch_decisions"))
    print("Max LLM calls:", limits.get("max_llm_calls"))
    print("Benchmark claim:", claims.get("benchmark_claim"))
    print("Policy quality claim:", claims.get("policy_quality_claim"))
    print("Run dir:", str(run_dir.relative_to(REPO_ROOT)))
    if args.get("provider") == "openai_compatible":
        print("External LLM provider enabled. Ctrl+C before first decision if unintended.")


def run_command(
    command: list[str],
    run_dir: Path,
    config: dict[str, Any],
) -> tuple[int, bool, bool, str | None, int]:
    artifacts = ensure_mapping(config, "artifacts")
    limits = ensure_mapping(config, "limits")
    stdout_path = run_dir / str(artifacts["stdout"])
    stderr_path = run_dir / str(artifacts["stderr"])
    max_wall_seconds = limits.get("max_wall_seconds")
    proc = subprocess.Popen(
        command,
        cwd=REPO_ROOT,
        stdin=None,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
        encoding="utf-8",
        errors="replace",
        bufsize=0,
    )
    assert proc.stdout is not None
    assert proc.stderr is not None
    monitor: dict[str, Any] = {
        "proc": proc,
        "max_llm_calls": limits.get("max_llm_calls"),
        "observed_llm_calls": 0,
        "limit_exceeded": None,
    }
    stdout_thread = threading.Thread(
        target=tee_stream,
        args=(proc.stdout, stdout_path, sys.stdout, monitor),
        daemon=True,
    )
    stderr_thread = threading.Thread(
        target=tee_stream,
        args=(proc.stderr, stderr_path, sys.stderr, monitor),
        kwargs={"line_mode": True},
        daemon=True,
    )
    stdout_thread.start()
    stderr_thread.start()
    started = time.time()
    timed_out = False
    interrupted = False
    try:
        while proc.poll() is None:
            if max_wall_seconds and time.time() - started > float(max_wall_seconds):
                timed_out = True
                proc.terminate()
                try:
                    proc.wait(timeout=10)
                except subprocess.TimeoutExpired:
                    proc.kill()
                break
            time.sleep(0.25)
    except KeyboardInterrupt:
        interrupted = True
        proc.terminate()
        try:
            proc.wait(timeout=10)
        except subprocess.TimeoutExpired:
            proc.kill()
    finally:
        stdout_thread.join(timeout=5)
        stderr_thread.join(timeout=5)
    return (
        130 if interrupted else int(proc.returncode or 0),
        timed_out,
        interrupted,
        monitor.get("limit_exceeded"),
        int(monitor.get("observed_llm_calls", 0)),
    )


def write_runner_summary(
    *,
    config: dict[str, Any],
    preset_ref: str,
    run_dir: Path,
    command: list[str],
    started_at: str,
    finished_at: str,
    exit_code: int,
    timed_out: bool,
    interrupted: bool,
    limit_exceeded: str | None,
    observed_llm_calls: int,
) -> None:
    artifacts = ensure_mapping(config, "artifacts")
    stdout_path = run_dir / str(artifacts["stdout"])
    controller_summary = None
    if stdout_path.exists():
        controller_summary = extract_first_json_object(stdout_path.read_text(encoding="utf-8"))
    summary = {
        "preset": preset_ref,
        "name": config.get("name"),
        "purpose": config.get("purpose"),
        "started_at": started_at,
        "finished_at": finished_at,
        "exit_code": exit_code,
        "timed_out": timed_out,
        "interrupted": interrupted,
        "exit_reason": "user_interrupt"
        if interrupted
        else ("timeout" if timed_out else (limit_exceeded or "process_exit")),
        "limit_exceeded": limit_exceeded,
        "observed_llm_calls": observed_llm_calls,
        "run_dir": str(run_dir.relative_to(REPO_ROOT)),
        "command": command,
        "claims": config.get("claims"),
        "limits": config.get("limits"),
        "artifacts": config.get("artifacts"),
        "controller_summary": controller_summary,
    }
    write_text(
        run_dir / str(artifacts["summary"]),
        json.dumps(summary, ensure_ascii=False, indent=2),
    )


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("preset", help="Preset name such as watch/deepseek_seed42")
    parser.add_argument("--set", dest="overrides", action="append", default=[])
    parser.add_argument("--dry-run", action="store_true")
    args = parser.parse_args()
    path = preset_path(args.preset)
    config = deep_copy(load_yaml(path))
    apply_overrides(config, args.overrides)
    validate_config(config)
    run_dir = unique_run_dir(config, args.preset)
    resolve_artifacts(config, run_dir)
    validate_config(config)
    command = command_from_config(config)
    run_dir.mkdir(parents=True, exist_ok=True)
    artifacts = ensure_mapping(config, "artifacts")
    write_text(run_dir / str(artifacts["resolved_config"]), dump_yaml(config) + "\n")
    write_text(run_dir / str(artifacts["command"]), " ".join(command) + "\n")
    print_summary(config, args.preset, run_dir)
    if args.dry_run:
        return 0
    started_at = time.strftime("%Y-%m-%dT%H:%M:%S%z")
    exit_code, timed_out, interrupted, limit_exceeded, observed_llm_calls = run_command(
        command,
        run_dir,
        config,
    )
    finished_at = time.strftime("%Y-%m-%dT%H:%M:%S%z")
    write_runner_summary(
        config=config,
        preset_ref=args.preset,
        run_dir=run_dir,
        command=command,
        started_at=started_at,
        finished_at=finished_at,
        exit_code=exit_code,
        timed_out=timed_out,
        interrupted=interrupted,
        limit_exceeded=limit_exceeded,
        observed_llm_calls=observed_llm_calls,
    )
    return exit_code


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except PresetError as err:
        print(f"preset error: {err}", file=sys.stderr)
        raise SystemExit(2)
