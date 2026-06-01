"""DecisionCaseV1 capture and persistence."""

from __future__ import annotations

import json
import re
import time
from pathlib import Path
from typing import Any

from sts_agent.context.observation_context import public_state_snapshot
from sts_agent.utils.llm_utils import find_candidate, json_safe


REPO_ROOT = Path(__file__).resolve().parents[4]
CASES_DIR = REPO_ROOT / "tools" / "artifacts" / "cases"


def safe_case_slug(value: Any) -> str:
    text = str(value or "unknown").strip().lower()
    text = re.sub(r"[^a-z0-9_+-]+", "_", text)
    text = re.sub(r"_+", "_", text).strip("_")
    return text or "unknown"

def save_decision_case_v1(
    *,
    step_index: int,
    decision_type: str,
    public_payload: dict[str, Any],
    candidates: list[dict[str, Any]],
    parsed: dict[str, Any],
    action_id: int,
    validation: dict[str, Any],
    action_candidate_policy: dict[str, Any] | None,
    combat_search_report: dict[str, Any] | None,
    decision_brief: dict[str, Any] | None,
    case_metadata: dict[str, Any] | None,
    llm_raw_text: str | None,
    human_note: str,
    suspected_issue: str,
) -> Path:
    selected = find_candidate(candidates, action_id) or {}
    policy = action_candidate_policy or {}
    timestamp = time.strftime("%Y%m%d_%H%M%S")
    decision_slug = safe_case_slug(decision_type)
    case_id = f"{timestamp}_{decision_slug}_step{step_index}"
    case = {
        "schema_name": "DecisionCaseV1",
        "schema_version": 1,
        "case_id": case_id,
        "case_role": "diagnostic_case",
        "label_role": "diagnostic_not_teacher_label",
        "trainable_as_action_label": False,
        "policy_quality_claim": False,
        "created_at_local": timestamp,
        "run_metadata": case_metadata or {},
        "step": step_index,
        "decision_type": decision_type,
        "decision_class": decision_type,
        "public_state_before": public_state_snapshot(public_payload),
        "public_payload_snapshot": public_payload,
        "legal_actions_all": candidates,
        "decision_candidates": policy.get("decision_candidates") or candidates,
        "locked_actions": policy.get("locked_actions") or [],
        "discouraged_actions": policy.get("dominated_or_discouraged_actions") or [],
        "action_candidate_policy": policy,
        "action_descriptors": [
            {
                "action_id": candidate.get("id"),
                "action_key": candidate.get("action_key"),
                "descriptor": candidate.get("action_descriptor")
                or (candidate.get("payload") or {}).get("semantic_descriptor"),
            }
            for candidate in candidates
            if isinstance(candidate, dict)
        ],
        "reward_card_choices": ((public_payload.get("screen") or {}).get("reward_card_choices") or []),
        "map_route_context": public_payload.get("map_route_context"),
        "combat_context": ((public_payload.get("combat") or {}).get("combat_context")),
        "llm_prompt_or_request": {
            "available": False,
            "reason": "watch case capture records parsed response and public/candidate context; raw prompt is not retained in this controller path yet",
        },
        "llm_response": parsed,
        "llm_raw_text": llm_raw_text,
        "selected_action": {
            "action_id": action_id,
            "action_key": selected.get("action_key"),
            "candidate": selected,
        },
        "validation": validation,
        "combat_search_report": combat_search_report,
        "decision_brief": decision_brief,
        "human_note": human_note,
        "suspected_issue": suspected_issue,
        "replay_handle": {
            "seed": (case_metadata or {}).get("seed"),
            "step": step_index,
            "status": "capture_only_no_replay_state_snapshot",
        },
        "truth_warnings": [
            "diagnostic case only",
            "not a teacher label",
            "not a policy-quality claim",
            "counterfactual tools must declare patch, scenario suite, continuation policy, and metrics before using this case",
        ],
    }
    CASES_DIR.mkdir(parents=True, exist_ok=True)
    path = CASES_DIR / f"{case_id}.json"
    path.write_text(json.dumps(json_safe(case), ensure_ascii=False, indent=2), encoding="utf-8")
    return path
