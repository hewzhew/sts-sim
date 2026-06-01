"""LLM provider and response parsing helpers."""

from __future__ import annotations

import json
import random
import re
import sys
import time
import urllib.error
import urllib.request
from typing import Any


def extract_json_object(text: str) -> dict[str, Any]:
    stripped = text.strip()
    if stripped.startswith("```"):
        stripped = re.sub(r"^```(?:json)?\s*", "", stripped)
        stripped = re.sub(r"\s*```$", "", stripped)
    try:
        value = json.loads(stripped)
    except json.JSONDecodeError:
        match = re.search(r"\{.*\}", stripped, flags=re.DOTALL)
        if not match:
            raise
        value = json.loads(match.group(0))
    if not isinstance(value, dict):
        raise ValueError("LLM response JSON was not an object")
    return value

def mock_choice(timestep: dict[str, Any], rng: random.Random) -> dict[str, Any]:
    policy = timestep.get("action_candidate_policy") or {}
    candidates = (
        policy.get("decision_candidates")
        or timestep.get("decision_candidates")
        or timestep.get("candidates")
        or []
    )
    if not candidates:
        raise RuntimeError("no candidates")
    observation = (timestep.get("observation") or {}).get("payload") or {}
    decision_type = observation.get("decision_type")
    chosen = candidates[0]
    if decision_type == "combat":
        non_end = [
            c
            for c in candidates
            if "combat/end_turn" not in str(c.get("action_key"))
        ]
        if non_end:
            chosen = non_end[0]
    elif decision_type == "map":
        chosen = candidates[rng.randrange(len(candidates))]
    return {
        "action_id": chosen["id"],
        "confidence": "low",
        "reason": "mock provider smoke-test choice; not a strategy label",
    }

def call_openai_compatible(
    *,
    base_url: str,
    api_key: str,
    model: str,
    system: str,
    user: str,
    temperature: float,
    timeout: int,
    phase: str | None = None,
) -> tuple[str, dict[str, Any]]:
    url = base_url.rstrip("/") + "/chat/completions"
    payload = {
        "model": model,
        "messages": [
            {"role": "system", "content": system},
            {"role": "user", "content": user},
        ],
        "temperature": temperature,
    }
    req = urllib.request.Request(
        url,
        data=json.dumps(payload).encode("utf-8"),
        headers={
            "Authorization": f"Bearer {api_key}",
            "Content-Type": "application/json",
        },
        method="POST",
    )
    started = time.monotonic()
    if phase:
        print(f"llm_call_start phase={phase} timeout={timeout}s", file=sys.stderr)
    try:
        with urllib.request.urlopen(req, timeout=timeout) as response:
            raw = response.read().decode("utf-8")
    except urllib.error.HTTPError as err:
        body = err.read().decode("utf-8", errors="replace")
        raise RuntimeError(f"LLM HTTP {err.code}: {body}") from err
    finally:
        if phase:
            elapsed = time.monotonic() - started
            print(f"llm_call_end phase={phase} elapsed={elapsed:.1f}s", file=sys.stderr)
    parsed = json.loads(raw)
    content = parsed["choices"][0]["message"]["content"]
    return content, parsed
