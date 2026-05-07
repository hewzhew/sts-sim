#!/usr/bin/env python3
from __future__ import annotations

import hashlib
import json
import math
import subprocess
from collections import Counter, defaultdict
from pathlib import Path
from typing import Any

REPO_ROOT = Path(__file__).resolve().parents[2]
_TORCH_MODEL_CACHE: dict[str, Any] = {}
SKIP_FEATURE_KEYS = {
    "action_id",
    "card_id_hash",
    "estimated_role_scores",
    "plan_delta",
    "reward_structure",
    "rule_score",
}


def binary_path(explicit: str | Path | None, name: str) -> Path:
    if explicit:
        path = Path(explicit)
        return path if path.is_absolute() else REPO_ROOT / path
    suffix = ".exe" if (REPO_ROOT / "target").exists() else ""
    candidates = [
        REPO_ROOT / "target" / "release" / f"{name}{suffix}",
        REPO_ROOT / "target" / "debug" / f"{name}{suffix}",
        REPO_ROOT / "target" / "release" / name,
        REPO_ROOT / "target" / "debug" / name,
    ]
    for candidate in candidates:
        if candidate.exists():
            return candidate
    raise SystemExit(
        f"could not find {name}; run `cargo build --bin {name}` or pass --binary"
    )


class FullRunDriver:
    def __init__(self, binary: str | Path | None = None) -> None:
        self.binary = binary_path(binary, "full_run_env_driver")
        self.proc: subprocess.Popen[str] | None = None

    def start(self) -> None:
        if self.proc is not None:
            return
        self.proc = subprocess.Popen(
            [str(self.binary)],
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
            encoding="utf-8",
            cwd=str(REPO_ROOT),
            bufsize=1,
        )
        self.request({"cmd": "ping"})

    def close(self) -> None:
        if self.proc is None:
            return
        try:
            self.request({"cmd": "close"})
        except Exception:
            pass
        proc = self.proc
        self.proc = None
        try:
            if proc.poll() is None:
                proc.terminate()
                proc.wait(timeout=1.0)
        except Exception:
            pass

    def request(self, payload: dict[str, Any]) -> dict[str, Any]:
        self.start()
        assert self.proc and self.proc.stdin and self.proc.stdout
        try:
            self.proc.stdin.write(json.dumps(payload, ensure_ascii=False) + "\n")
            self.proc.stdin.flush()
        except OSError as err:
            stderr = self._take_stderr()
            self.proc = None
            raise RuntimeError(f"full_run_env_driver pipe write failed: {err}; stderr={stderr}") from err
        line = self.proc.stdout.readline()
        if not line:
            stderr = self._take_stderr()
            self.proc = None
            raise RuntimeError(f"full_run_env_driver exited unexpectedly: {stderr}")
        response = json.loads(line)
        if not response.get("ok"):
            raise RuntimeError(str(response.get("error") or "unknown driver error"))
        return response

    def _take_stderr(self) -> str:
        if self.proc is None or self.proc.stderr is None:
            return ""
        try:
            return self.proc.stderr.read()
        except Exception:
            return ""


def stable_group_split(group_key: str) -> str:
    digest = hashlib.sha256(group_key.encode("utf-8")).digest()
    bucket = int.from_bytes(digest[:8], "big") % 100
    if bucket < 80:
        return "train"
    if bucket < 90:
        return "valid"
    return "test"


def read_jsonl(path: Path) -> list[dict[str, Any]]:
    rows = []
    with path.open("r", encoding="utf-8") as handle:
        for line in handle:
            line = line.strip()
            if line:
                rows.append(json.loads(line))
    return rows


def write_json(path: Path, payload: dict[str, Any]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("w", encoding="utf-8") as handle:
        json.dump(payload, handle, indent=2, sort_keys=True)
        handle.write("\n")


def action_key(candidate: dict[str, Any]) -> str:
    return str(candidate.get("action_key") or "")


def candidate_allowed(
    candidate: dict[str, Any],
    scope: str = "all",
    observation: dict[str, Any] | None = None,
) -> bool:
    if scope in {"", "all"}:
        return True
    key = action_key(candidate)
    if scope == "controlled_v0":
        return key.startswith("combat/play_card") or key.startswith("combat/end_turn")
    if scope == "controlled_v1":
        decision_type = str((observation or {}).get("decision_type") or "")
        if decision_type == "combat":
            return key.startswith("combat/play_card") or key.startswith("combat/end_turn")
        if not decision_type.startswith("combat_"):
            return False
        if decision_type == "combat_card_reward":
            return False
        if key.startswith("combat/use_potion") or key.startswith("combat/discard_potion"):
            return False
        return (
            key.startswith("choice/")
            or key.startswith("combat/hand_select")
            or key.startswith("combat/grid_select")
            or key.startswith("combat/scry_discard")
            or key.startswith("combat/card_choice")
            or key.startswith("selection/")
            or key in {"proceed", "cancel"}
        )
    raise ValueError(f"unknown candidate scope {scope}")


def legal_candidate_indices(response: dict[str, Any], scope: str = "all") -> list[int]:
    payload = response.get("payload") or {}
    observation = payload.get("observation") or {}
    candidates = payload.get("action_candidates") or []
    return [
        idx
        for idx, legal in enumerate(payload.get("action_mask") or [])
        if legal
        and idx < len(candidates)
        and candidate_allowed(candidates[idx], scope, observation)
    ]


def model_scores_are_rank_only(model: dict[str, Any]) -> bool:
    target_mode = str(model.get("target_mode") or model.get("config", {}).get("target_mode") or "")
    return target_mode in {"group_centered_return", "pairwise_return", "listwise_return"}


def action_only_features(candidate: dict[str, Any]) -> tuple[Counter[str], list[str]]:
    key = action_key(candidate)
    features: Counter[str] = Counter()
    tokens = []
    if key.startswith("combat/end_turn"):
        tokens.append("action:end_turn")
    elif key.startswith("combat/play_card"):
        tokens.append("action:play_card")
        card = extract_segment(key, "card")
        if card:
            tokens.append(f"card:{card}")
    elif key.startswith("combat/use_potion"):
        tokens.append("action:use_potion")
    else:
        head = key.split("/", 1)[0] if key else "unknown"
        tokens.append(f"action:{head}")
    for token in tokens:
        features[token] += 1.0
    return features, tokens


def extract_segment(key: str, name: str) -> str:
    marker = f"{name}:"
    for part in key.split("/"):
        if part.startswith(marker):
            return part[len(marker) :]
    return ""


def flatten_json_features(
    value: Any,
    prefix: str,
    *,
    max_depth: int = 5,
    max_items: int = 48,
) -> tuple[Counter[str], list[str]]:
    features: Counter[str] = Counter()
    cats: list[str] = []

    def visit(node: Any, path: str, depth: int) -> None:
        if depth > max_depth:
            return
        if isinstance(node, dict):
            for key in sorted(node.keys())[:max_items]:
                if key in SKIP_FEATURE_KEYS:
                    continue
                visit(node[key], f"{path}.{key}", depth + 1)
            return
        if isinstance(node, list):
            features[f"{path}.len"] += math.tanh(len(node) / 20.0)
            cats.append(f"{path}.len_bucket:{min(len(node), 20)}")
            features[cats[-1]] += 1.0
            for idx, item in enumerate(node[:max_items]):
                visit(item, f"{path}[{idx}]", depth + 1)
            return
        if isinstance(node, bool):
            token = f"{path}:{str(node).lower()}"
            features[token] += 1.0
            cats.append(token)
            return
        if isinstance(node, (int, float)) and not isinstance(node, bool):
            if not math.isfinite(float(node)):
                return
            value_f = float(node)
            features[f"{path}.num"] += math.tanh(value_f / 50.0)
            token = f"{path}.bucket:{math.floor(value_f / 5.0)}"
            features[token] += 1.0
            cats.append(token)
            return
        if node is None:
            token = f"{path}:none"
            features[token] += 1.0
            cats.append(token)
            return
        text = str(node)
        if len(text) > 96:
            text = text[:96]
        token = f"{path}:{text}"
        features[token] += 1.0
        cats.append(token)

    visit(value, prefix, 0)
    return features, cats


def row_features(row: dict[str, Any], feature_set: str) -> dict[int, float]:
    candidate = row.get("candidate") or {}
    observation = row.get("observation") or {}
    features: Counter[str] = Counter()
    state_cats: list[str] = []
    candidate_cats: list[str] = []

    if feature_set == "action_only":
        action_features, candidate_cats = action_only_features(candidate)
        features.update(action_features)
    elif feature_set == "candidate_only":
        candidate_features, candidate_cats = compact_candidate_features(candidate)
        action_features, action_cats = action_only_features(candidate)
        features.update(candidate_features)
        features.update(action_features)
        candidate_cats.extend(action_cats)
    elif feature_set == "state_only":
        state_features, state_cats = compact_state_features(observation)
        features.update(state_features)
    elif feature_set == "full_state_plus_candidate":
        state_features, state_cats = compact_state_features(observation)
        candidate_features, candidate_cats = compact_candidate_features(candidate)
        action_features, action_cats = action_only_features(candidate)
        features.update(state_features)
        features.update(candidate_features)
        features.update(action_features)
        candidate_cats.extend(action_cats)
        for state_token in state_cats[:96]:
            for candidate_token in candidate_cats[:32]:
                features[f"cross:{state_token}|{candidate_token}"] += 1.0
    else:
        raise ValueError(f"unknown feature set {feature_set}")

    sparse: defaultdict[int, float] = defaultdict(float)
    for token, value in features.items():
        sparse[hash_feature(token)] += float(value)
    return {idx: value for idx, value in sparse.items() if value}


ADV_OVERRIDE_FEATURE_SETS = [
    "action_only",
    "candidate_only",
    "candidate_plus_cheap",
    "state_only",
    "full_decision_plus_choice",
]


def adv_override_features(row: dict[str, Any], feature_set: str) -> dict[int, float]:
    candidate = row.get("candidate") or {}
    rule_candidate = row.get("rule_candidate") or row.get("reference_candidate") or {}
    observation = row.get("observation") or {}
    features: Counter[str] = Counter()
    state_cats: list[str] = []
    candidate_cats: list[str] = []
    rule_cats: list[str] = []
    delta_cats: list[str] = []

    if feature_set == "action_only":
        candidate_features, candidate_cats = action_only_features(candidate)
        rule_features, rule_cats = action_only_features(rule_candidate)
        prefixed_rule = Counter({f"rule.{key}": value for key, value in rule_features.items()})
        features.update(candidate_features)
        features.update(prefixed_rule)
    elif feature_set == "candidate_only":
        candidate_features, candidate_cats = compact_candidate_features(candidate)
        candidate_action_features, candidate_action_cats = action_only_features(candidate)
        rule_features, rule_cats = compact_candidate_features(rule_candidate)
        rule_action_features, rule_action_cats = action_only_features(rule_candidate)
        features.update(candidate_features)
        features.update(candidate_action_features)
        for key, value in rule_features.items():
            features[f"rule.{key}"] += value
        for key, value in rule_action_features.items():
            features[f"rule.{key}"] += value
        candidate_cats.extend(candidate_action_cats)
        rule_cats.extend(rule_action_cats)
    elif feature_set == "candidate_plus_cheap":
        candidate_features, candidate_cats = compact_candidate_features(candidate)
        candidate_action_features, candidate_action_cats = action_only_features(candidate)
        rule_features, rule_cats = compact_candidate_features(rule_candidate)
        rule_action_features, rule_action_cats = action_only_features(rule_candidate)
        cheap_features, _cheap_cats = flatten_json_features(
            row.get("cheap_return_features") or {},
            "cheap",
            max_depth=2,
            max_items=32,
        )
        features.update(candidate_features)
        features.update(candidate_action_features)
        features.update(cheap_features)
        for key, value in rule_features.items():
            features[f"rule.{key}"] += value
        for key, value in rule_action_features.items():
            features[f"rule.{key}"] += value
        candidate_cats.extend(candidate_action_cats)
        rule_cats.extend(rule_action_cats)
    elif feature_set == "state_only":
        state_features, state_cats = compact_state_features(observation)
        features.update(state_features)
    elif feature_set == "full_decision_plus_choice":
        state_features, state_cats = compact_state_features(observation)
        candidate_features, candidate_cats = compact_candidate_features(candidate)
        candidate_action_features, candidate_action_cats = action_only_features(candidate)
        rule_features, rule_cats = compact_candidate_features(rule_candidate)
        rule_action_features, rule_action_cats = action_only_features(rule_candidate)
        delta_features, delta_cats = flatten_json_features(
            {
                "candidate_delta": row.get("candidate_delta_vs_start") or {},
                "rule_delta": row.get("rule_delta_vs_start") or {},
                "delta_vs_rule": row.get("delta_vs_rule_features") or {},
                "cheap": row.get("cheap_return_features") or {},
                "decision_kind": row.get("decision_kind") or observation.get("decision_type"),
            },
            "adv",
            max_depth=4,
            max_items=64,
        )
        features.update(state_features)
        features.update(candidate_features)
        features.update(candidate_action_features)
        features.update(delta_features)
        for key, value in rule_features.items():
            features[f"rule.{key}"] += value
        for key, value in rule_action_features.items():
            features[f"rule.{key}"] += value
        candidate_cats.extend(candidate_action_cats)
        rule_cats.extend(rule_action_cats)
        for state_token in state_cats[:96]:
            for candidate_token in candidate_cats[:32]:
                features[f"cross.state_candidate:{state_token}|{candidate_token}"] += 1.0
            for rule_token in rule_cats[:24]:
                features[f"cross.state_rule:{state_token}|{rule_token}"] += 1.0
        for candidate_token in candidate_cats[:48]:
            for rule_token in rule_cats[:24]:
                features[f"cross.candidate_rule:{candidate_token}|{rule_token}"] += 1.0
        for delta_token in delta_cats[:64]:
            features[f"cross.delta_decision:{delta_token}|{observation.get('decision_type', 'unknown')}"] += 1.0
    else:
        raise ValueError(f"unknown advantage override feature set {feature_set}")

    sparse: defaultdict[int, float] = defaultdict(float)
    for token, value in features.items():
        sparse[hash_feature(token)] += float(value)
    return {idx: value for idx, value in sparse.items() if value}


def compact_state_features(observation: dict[str, Any]) -> tuple[Counter[str], list[str]]:
    features: Counter[str] = Counter()
    cats: list[str] = []

    def cat(token: str) -> None:
        features[token] += 1.0
        cats.append(token)

    def num(name: str, value: Any, width: float) -> None:
        try:
            value_f = float(value)
        except (TypeError, ValueError):
            return
        if not math.isfinite(value_f):
            return
        features[f"state.{name}.num"] += math.tanh(value_f / max(width * 10.0, 1.0))
        cat(f"state.{name}.bucket:{math.floor(value_f / max(width, 1.0))}")

    combat = observation.get("combat") or {}
    deck = observation.get("deck") or {}
    cat(f"state.decision_type:{observation.get('decision_type', 'unknown')}")
    cat(f"state.engine_state:{observation.get('engine_state', 'unknown')}")
    cat(f"state.room:{observation.get('current_room', 'unknown')}")
    cat(f"state.act_boss:{observation.get('act_boss', 'unknown')}")
    num("act", observation.get("act"), 1)
    num("floor", observation.get("floor"), 5)
    num("current_hp", observation.get("current_hp"), 10)
    num("hp_ratio_milli", observation.get("hp_ratio_milli"), 100)
    num("gold", observation.get("gold"), 50)
    num("deck_size", observation.get("deck_size"), 5)
    num("relic_count", observation.get("relic_count"), 2)

    for key, width in [
        ("energy", 1),
        ("turn_count", 1),
        ("hand_count", 1),
        ("draw_count", 5),
        ("discard_count", 5),
        ("exhaust_count", 3),
        ("player_block", 5),
        ("visible_incoming_damage", 5),
        ("total_monster_hp", 10),
        ("alive_monster_count", 1),
    ]:
        num(f"combat_{key}", combat.get(key), width)

    for key in [
        "attack_count",
        "skill_count",
        "power_count",
        "damage_card_count",
        "block_card_count",
        "draw_card_count",
        "scaling_card_count",
        "exhaust_card_count",
        "starter_basic_count",
    ]:
        num(f"deck_{key}", deck.get(key), 2)

    for card in (combat.get("hand_cards") or [])[:12]:
        card_id = card.get("card_id")
        if card_id:
            cat(f"state.hand_card:{card_id}")
        if card.get("playable"):
            cat("state.hand_playable:true")
        num("hand_card_cost", card.get("cost_for_turn"), 1)
        for semantic in card.get("base_semantics") or []:
            cat(f"state.hand_semantic:{semantic}")

    return features, cats


def compact_candidate_features(candidate: dict[str, Any]) -> tuple[Counter[str], list[str]]:
    features: Counter[str] = Counter()
    cats: list[str] = []

    def cat(token: str) -> None:
        features[token] += 1.0
        cats.append(token)

    def num(name: str, value: Any, width: float) -> None:
        try:
            value_f = float(value)
        except (TypeError, ValueError):
            return
        if not math.isfinite(value_f):
            return
        features[f"candidate.{name}.num"] += math.tanh(value_f / max(width * 10.0, 1.0))
        cat(f"candidate.{name}.bucket:{math.floor(value_f / max(width, 1.0))}")

    action = candidate.get("action") or {}
    card = candidate.get("card") or {}
    key = action_key(candidate)
    cat(f"candidate.action_type:{action.get('type', key.split('/')[0] if key else 'unknown')}")
    target = extract_segment(key, "target")
    if target:
        cat(f"candidate.target:{target}")
    if candidate.get("dominated"):
        cat("candidate.dominated:true")
    num("action_index", candidate.get("action_index"), 4)

    if card:
        for key_name in ["card_id", "card_type_id", "rarity_id", "cost", "upgrades"]:
            value = card.get(key_name)
            if value is not None:
                cat(f"candidate.card_{key_name}:{value}")
        for key_name in [
            "starter_basic",
            "aoe",
            "multi_damage",
            "draws_cards",
            "gains_energy",
            "exhaust",
            "ethereal",
            "applies_vulnerable",
            "applies_weak",
            "scaling_piece",
        ]:
            if card.get(key_name):
                cat(f"candidate.card_{key_name}:true")
        for key_name, width in [
            ("base_damage", 4),
            ("base_block", 4),
            ("base_magic", 2),
            ("deck_copies", 1),
        ]:
            num(f"card_{key_name}", card.get(key_name), width)

    return features, cats


def hash_feature(token: str, dim: int = 32768) -> int:
    digest = hashlib.blake2b(token.encode("utf-8"), digest_size=8).digest()
    return int.from_bytes(digest, "big") % dim


def dot(weights: dict[int, float], sparse: dict[int, float]) -> float:
    return sum(weights.get(idx, 0.0) * value for idx, value in sparse.items())


def predict_model(model: dict[str, Any], observation: dict[str, Any], candidate: dict[str, Any]) -> float:
    if str(model.get("model_type") or "") == "torch_embedding_mlp":
        return predict_torch_embedding_mlp(model, observation, candidate)
    row = {"observation": observation, "candidate": candidate}
    sparse = row_features(row, str(model["feature_set"]))
    weights = {int(idx): float(value) for idx, value in model.get("weights", [])}
    raw = float(model.get("bias", 0.0)) + dot(weights, sparse)
    return raw * float(model.get("target_std", 1.0)) + float(model.get("target_mean", 0.0))


def predict_adv_override_probability(
    model: dict[str, Any],
    observation: dict[str, Any],
    candidate: dict[str, Any],
    rule_candidate: dict[str, Any],
    cheap_return_features: dict[str, Any] | None = None,
    candidate_delta_vs_start: dict[str, Any] | None = None,
    rule_delta_vs_start: dict[str, Any] | None = None,
    delta_vs_rule_features: dict[str, Any] | None = None,
) -> float:
    model_type = str(model.get("model_type") or "")
    if model_type == "verified_proposer_sklearn_pickle_v0" or str(model.get("schema_version") or "") == "verified_proposer_sklearn_pickle_v0":
        return predict_sklearn_verified_proposer(
            model,
            observation,
            candidate,
            rule_candidate,
            cheap_return_features,
            candidate_delta_vs_start,
            rule_delta_vs_start,
            delta_vs_rule_features,
        )
    if model_type == "verified_proposer_torch_embedding_mlp_v0" or str(model.get("schema_version") or "") == "verified_proposer_torch_embedding_mlp_v0":
        return predict_torch_verified_proposer(
            model,
            observation,
            candidate,
            rule_candidate,
            cheap_return_features,
            candidate_delta_vs_start,
            rule_delta_vs_start,
            delta_vs_rule_features,
        )
    if model_type != "verified_proposer_linear":
        raise ValueError(f"unsupported verified proposer model_type {model_type!r}")
    row = {
        "observation": observation,
        "candidate": candidate,
        "rule_candidate": rule_candidate,
        "decision_kind": observation.get("decision_type"),
        "cheap_return_features": cheap_return_features or {},
        "candidate_delta_vs_start": candidate_delta_vs_start or {},
        "rule_delta_vs_start": rule_delta_vs_start or {},
        "delta_vs_rule_features": delta_vs_rule_features or {},
    }
    sparse = adv_override_features(row, str(model["feature_set"]))
    weights = {int(idx): float(value) for idx, value in model.get("weights", [])}
    raw = float(model.get("bias", 0.0)) + dot(weights, sparse)
    return sigmoid(raw)


def predict_sklearn_verified_proposer(
    model: dict[str, Any],
    observation: dict[str, Any],
    candidate: dict[str, Any],
    rule_candidate: dict[str, Any],
    cheap_return_features: dict[str, Any] | None = None,
    candidate_delta_vs_start: dict[str, Any] | None = None,
    rule_delta_vs_start: dict[str, Any] | None = None,
    delta_vs_rule_features: dict[str, Any] | None = None,
) -> float:
    import numpy as np
    from scipy.sparse import csr_matrix

    row = {
        "observation": observation,
        "candidate": candidate,
        "rule_candidate": rule_candidate,
        "decision_kind": observation.get("decision_type"),
        "cheap_return_features": cheap_return_features or {},
        "candidate_delta_vs_start": candidate_delta_vs_start or {},
        "rule_delta_vs_start": rule_delta_vs_start or {},
        "delta_vs_rule_features": delta_vs_rule_features or {},
    }
    sparse = adv_override_features(row, str(model["feature_set"]))
    feature_dim = int(model.get("feature_dim") or 32768)
    indices = []
    data = []
    for idx, value in sparse.items():
        idx_i = int(idx)
        if 0 <= idx_i < feature_dim and value:
            indices.append(idx_i)
            data.append(float(value))
    matrix = csr_matrix(
        (
            np.asarray(data, dtype=np.float32),
            np.asarray(indices, dtype=np.int32),
            np.asarray([0, len(indices)], dtype=np.int32),
        ),
        shape=(1, feature_dim),
    )
    sklearn_model = model["model"]
    if hasattr(sklearn_model, "predict_proba"):
        probs = sklearn_model.predict_proba(matrix)
        classes = list(getattr(sklearn_model, "classes_", [0, 1]))
        pos_idx = classes.index(1) if 1 in classes else len(classes) - 1
        return float(probs[0, pos_idx])
    if hasattr(sklearn_model, "decision_function"):
        score = float(sklearn_model.decision_function(matrix)[0])
        return sigmoid(score)
    return float(sklearn_model.predict(matrix)[0])


def predict_torch_verified_proposer(
    model: dict[str, Any],
    observation: dict[str, Any],
    candidate: dict[str, Any],
    rule_candidate: dict[str, Any],
    cheap_return_features: dict[str, Any] | None = None,
    candidate_delta_vs_start: dict[str, Any] | None = None,
    rule_delta_vs_start: dict[str, Any] | None = None,
    delta_vs_rule_features: dict[str, Any] | None = None,
) -> float:
    torch_model, torch = load_torch_embedding_mlp(model)
    row = {
        "observation": observation,
        "candidate": candidate,
        "rule_candidate": rule_candidate,
        "decision_kind": observation.get("decision_type"),
        "cheap_return_features": cheap_return_features or {},
        "candidate_delta_vs_start": candidate_delta_vs_start or {},
        "rule_delta_vs_start": rule_delta_vs_start or {},
        "delta_vs_rule_features": delta_vs_rule_features or {},
    }
    sparse = adv_override_features(row, str(model["feature_set"]))
    if sparse:
        indices = torch.tensor(list(sparse.keys()), dtype=torch.long)
        weights = torch.tensor(list(sparse.values()), dtype=torch.float32)
    else:
        indices = torch.tensor([0], dtype=torch.long)
        weights = torch.tensor([0.0], dtype=torch.float32)
    offsets = torch.tensor([0, len(indices)], dtype=torch.long)
    with torch.no_grad():
        logit = float(torch_model(indices, offsets, weights).item())
    return sigmoid(logit)


def sigmoid(value: float) -> float:
    if value >= 0:
        z = math.exp(-value)
        return 1.0 / (1.0 + z)
    z = math.exp(value)
    return z / (1.0 + z)


def predict_torch_embedding_mlp(
    model: dict[str, Any],
    observation: dict[str, Any],
    candidate: dict[str, Any],
) -> float:
    torch_model, torch = load_torch_embedding_mlp(model)
    sparse = row_features(
        {"observation": observation, "candidate": candidate},
        str(model["feature_set"]),
    )
    if sparse:
        indices = torch.tensor(list(sparse.keys()), dtype=torch.long)
        weights = torch.tensor(list(sparse.values()), dtype=torch.float32)
    else:
        indices = torch.tensor([0], dtype=torch.long)
        weights = torch.tensor([0.0], dtype=torch.float32)
    offsets = torch.tensor([0, len(indices)], dtype=torch.long)
    with torch.no_grad():
        score = torch_model(indices, offsets, weights).item()
    return float(score) * float(model.get("target_std", 1.0)) + float(model.get("target_mean", 0.0))


def load_torch_embedding_mlp(model: dict[str, Any]) -> tuple[Any, Any]:
    state_path = Path(str(model["state_dict_path"]))
    if not state_path.is_absolute():
        state_path = REPO_ROOT / state_path
    config = model.get("config") or {}
    cache_key = "|".join(
        [
            str(state_path.resolve()),
            str(model.get("model_type") or ""),
            str(config.get("feature_dim") or 32768),
            str(config.get("hidden_dim") or 64),
        ]
    )
    cached = _TORCH_MODEL_CACHE.get(cache_key)
    if cached is not None:
        return cached
    import torch
    import torch.nn as nn

    class TorchEmbeddingMlp(nn.Module):
        def __init__(self, feature_dim: int, hidden_dim: int) -> None:
            super().__init__()
            dropout_p = float(config.get("dropout_p") or 0.0)
            layers: list[Any] = [
                nn.LayerNorm(hidden_dim),
                nn.Linear(hidden_dim, hidden_dim),
                nn.ReLU(),
            ]
            if dropout_p > 0.0:
                layers.append(nn.Dropout(p=dropout_p))
            layers.append(nn.Linear(hidden_dim, 1))
            self.embedding = nn.EmbeddingBag(
                feature_dim,
                hidden_dim,
                mode="sum",
                include_last_offset=True,
            )
            self.net = nn.Sequential(*layers)

        def forward(self, indices: Any, offsets: Any, weights: Any) -> Any:
            embedded = self.embedding(indices, offsets, per_sample_weights=weights)
            return self.net(embedded).squeeze(-1)

    torch_model = TorchEmbeddingMlp(
        int(config.get("feature_dim") or 32768),
        int(config.get("hidden_dim") or 64),
    )
    state = torch.load(state_path, map_location="cpu")
    torch_model.load_state_dict(state)
    torch_model.eval()
    cached = (torch_model, torch)
    _TORCH_MODEL_CACHE[cache_key] = cached
    return cached
