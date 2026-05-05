#!/usr/bin/env python3
from __future__ import annotations

import argparse
import html
import json
from pathlib import Path
from typing import Any

import numpy as np
import torch

from combat_rl_common import REPO_ROOT, write_json
from structured_combat_env import (
    ACTION_TYPE_CANCEL,
    ACTION_TYPE_CHOICE,
    ACTION_TYPE_END_TURN,
    ACTION_TYPE_PLAY_CARD,
    ACTION_TYPE_PROCEED,
    ACTION_TYPE_USE_POTION,
    StructuredGymCombatEnv,
)
from structured_policy import StructuredPolicyNet, to_device_obs

ACTION_TYPE_LABELS = {
    ACTION_TYPE_END_TURN: "end_turn",
    ACTION_TYPE_PLAY_CARD: "play_card",
    ACTION_TYPE_USE_POTION: "use_potion",
    ACTION_TYPE_CHOICE: "choice",
    ACTION_TYPE_PROCEED: "proceed",
    ACTION_TYPE_CANCEL: "cancel",
}

PROBE_LABELS = [
    "urgent_survival",
    "lethal_exists",
    "setup_window",
    "aoe_balance_pressure",
]


def _masked_probabilities(logits: torch.Tensor, mask: torch.Tensor) -> torch.Tensor:
    bool_mask = mask > 0.5
    if not bool_mask.any():
        fallback = torch.zeros_like(bool_mask)
        fallback[..., 0] = True
        bool_mask = fallback
    masked_logits = logits.masked_fill(~bool_mask, -1e9)
    return torch.softmax(masked_logits, dim=-1)


def _topk_named(values: np.ndarray, names: list[str], k: int) -> list[dict[str, Any]]:
    if values.size == 0:
        return []
    order = np.argsort(values)[::-1][:k]
    rows = []
    for index in order:
        rows.append(
            {
                "index": int(index),
                "name": names[index] if index < len(names) else str(index),
                "prob": float(values[index]),
            }
        )
    return rows


def _monster_names(raw_observation: dict[str, Any]) -> list[str]:
    names = ["none"]
    for monster in raw_observation.get("monsters") or []:
        names.append(str(monster.get("name") or monster.get("monster_id") or "monster"))
    return names


def _hand_names(raw_observation: dict[str, Any], max_hand: int) -> list[str]:
    names = []
    hand = list(raw_observation.get("hand") or [])
    for index in range(max_hand):
        if index < len(hand):
            card = hand[index]
            names.append(f"{index}: {card.get('name') or card.get('card_id') or 'card'}")
        else:
            names.append(f"{index}: <empty>")
    return names


def _potion_names(raw_observation: dict[str, Any], max_potions: int) -> list[str]:
    names = []
    potions = list(raw_observation.get("potions") or [])
    for index in range(max_potions):
        if index < len(potions):
            potion = potions[index]
            names.append(f"{index}: {potion.get('name') or potion.get('potion_id') or 'potion'}")
        else:
            names.append(f"{index}: <empty>")
    return names


def _choice_names(raw_observation: dict[str, Any], max_choices: int) -> list[str]:
    names = []
    pending_choice = raw_observation.get("pending_choice") or {}
    options = list(pending_choice.get("options") or [])
    for index in range(max_choices):
        if index < len(options):
            option = options[index]
            label = option.get("label") or option.get("card_id") or option.get("option_index") or "option"
            names.append(f"{index}: {label}")
        else:
            names.append(f"{index}: <empty>")
    return names

def _raw_observation_summary(raw_observation: dict[str, Any]) -> dict[str, Any]:
    return {
        "turn_count": raw_observation.get("turn_count"),
        "phase": raw_observation.get("phase"),
        "engine_state": raw_observation.get("engine_state"),
        "player": {
            "hp": raw_observation.get("player_hp"),
            "max_hp": raw_observation.get("player_max_hp"),
            "block": raw_observation.get("player_block"),
            "energy": raw_observation.get("energy"),
        },
        "player_powers": raw_observation.get("player_powers") or [],
        "draw_count": raw_observation.get("draw_count"),
        "discard_count": raw_observation.get("discard_count"),
        "exhaust_count": raw_observation.get("exhaust_count"),
        "turn_prefix": raw_observation.get("turn_prefix"),
        "pressure": raw_observation.get("pressure"),
        "belief": raw_observation.get("belief"),
        "monsters": [
            {
                "slot": monster.get("slot"),
                "entity_id": monster.get("entity_id"),
                "name": monster.get("name"),
                "monster_id": monster.get("monster_id"),
                "hp": monster.get("current_hp"),
                "max_hp": monster.get("max_hp"),
                "block": monster.get("block"),
                "alive": monster.get("alive"),
                "targetable": monster.get("targetable"),
                "intent": monster.get("intent_payload"),
                "powers": monster.get("powers"),
                "mechanic_state": monster.get("mechanic_state"),
            }
            for monster in (raw_observation.get("monsters") or [])
        ],
        "hand": [
            {
                "slot": card.get("index"),
                "uuid": card.get("uuid"),
                "name": card.get("name"),
                "card_id": card.get("card_id"),
                "cost": card.get("cost_for_turn"),
                "playable": card.get("playable"),
                "target_mode": card.get("target_mode"),
                "upgraded": card.get("upgraded"),
                "exhausts_when_played": card.get("exhausts_when_played"),
                "ethereal": card.get("ethereal"),
                "retain": card.get("retain"),
            }
            for card in (raw_observation.get("hand") or [])
        ],
        "potions": [
            {
                "slot": potion.get("slot"),
                "uuid": potion.get("uuid"),
                "name": potion.get("name"),
                "potion_id": potion.get("potion_id"),
                "target_mode": potion.get("target_mode"),
                "usable": potion.get("usable"),
            }
            for potion in (raw_observation.get("potions") or [])
        ],
        "pending_choice_kind": raw_observation.get("pending_choice_kind"),
        "pending_choice": raw_observation.get("pending_choice"),
    }


def _candidate_probability(
    candidate: dict[str, Any],
    canonical: dict[str, Any],
    distributions: dict[str, Any],
) -> float:
    action_type = int(canonical.get("action_type") or 0)
    action_type_probs = distributions["action_type_probs"]
    if action_type >= len(action_type_probs):
        return 0.0
    probability = float(action_type_probs[action_type])
    if action_type == ACTION_TYPE_PLAY_CARD:
        slot = int(canonical.get("card_slot") or 0)
        target_slot = int(canonical.get("target_slot") or 0)
        probability *= float(distributions["card_probs"][slot])
        probability *= float(distributions["card_target_probs"].get(slot, [0.0])[target_slot])
    elif action_type == ACTION_TYPE_USE_POTION:
        slot = int(canonical.get("potion_slot") or 0)
        target_slot = int(canonical.get("target_slot") or 0)
        probability *= float(distributions["potion_probs"][slot])
        probability *= float(distributions["potion_target_probs"].get(slot, [0.0])[target_slot])
    elif action_type == ACTION_TYPE_CHOICE:
        choice_index = int(canonical.get("choice_index") or 0)
        probability *= float(distributions["choice_probs"][choice_index])
    return probability


def analyze_policy_step(
    model: StructuredPolicyNet,
    env: StructuredGymCombatEnv,
    obs: dict[str, np.ndarray],
    raw_observation: dict[str, Any],
    *,
    deterministic: bool,
    device: torch.device,
    top_k: int,
) -> tuple[dict[str, int], dict[str, Any]]:
    obs_tensor = to_device_obs({key: value[None, ...] for key, value in obs.items()}, device)
    with torch.no_grad():
        state = model.encode(obs_tensor)

        action_type_probs_t = _masked_probabilities(state.action_type_logits[0], obs_tensor["action_type_mask"][0])
        action_type_probs = action_type_probs_t.detach().cpu().numpy()
        if deterministic:
            action_type = int(action_type_probs.argmax())
        else:
            action_type = int(torch.distributions.Categorical(probs=action_type_probs_t).sample().item())

        card_probs = np.zeros_like(obs["play_card_mask"], dtype=np.float32)
        potion_probs = np.zeros_like(obs["use_potion_mask"], dtype=np.float32)
        choice_probs = np.zeros_like(obs["choice_option_mask"], dtype=np.float32)
        card_target_probs: dict[int, np.ndarray] = {}
        potion_target_probs: dict[int, np.ndarray] = {}

        action: dict[str, int] = {
            "action_type": action_type,
            "card_slot": 0,
            "target_slot": 0,
            "potion_slot": 0,
            "choice_index": 0,
        }

        if float(obs["play_card_mask"].sum()) > 0:
            card_probs_t = _masked_probabilities(state.card_logits[0], obs_tensor["play_card_mask"][0])
            card_probs = card_probs_t.detach().cpu().numpy()
            for slot in np.where(obs["play_card_mask"] > 0.5)[0]:
                chosen_card = state.hand_repr[0, int(slot)].unsqueeze(0)
                target_logits = model.card_target_logits(
                    state.__class__(
                        tactical=state.tactical[:1],
                        value=state.value[:1],
                        probes=state.probes[:1],
                        hand_repr=state.hand_repr[:1],
                        potion_repr=state.potion_repr[:1],
                        monster_repr=state.monster_repr[:1],
                        choice_repr=state.choice_repr[:1],
                        action_type_logits=state.action_type_logits[:1],
                        card_logits=state.card_logits[:1],
                        potion_logits=state.potion_logits[:1],
                        choice_logits=state.choice_logits[:1],
                    ),
                    chosen_card,
                )[0]
                target_probs = _masked_probabilities(
                    target_logits,
                    obs_tensor["play_card_target_mask"][0, int(slot)],
                ).detach().cpu().numpy()
                card_target_probs[int(slot)] = target_probs
            if action_type == ACTION_TYPE_PLAY_CARD:
                chosen_slot = int(card_probs.argmax()) if deterministic else int(torch.distributions.Categorical(probs=card_probs_t).sample().item())
                action["card_slot"] = chosen_slot
                target_probs = card_target_probs.get(chosen_slot)
                if target_probs is not None:
                    action["target_slot"] = int(target_probs.argmax())

        if float(obs["use_potion_mask"].sum()) > 0:
            potion_probs_t = _masked_probabilities(state.potion_logits[0], obs_tensor["use_potion_mask"][0])
            potion_probs = potion_probs_t.detach().cpu().numpy()
            for slot in np.where(obs["use_potion_mask"] > 0.5)[0]:
                chosen_potion = state.potion_repr[0, int(slot)].unsqueeze(0)
                target_logits = model.potion_target_logits(
                    state.__class__(
                        tactical=state.tactical[:1],
                        value=state.value[:1],
                        probes=state.probes[:1],
                        hand_repr=state.hand_repr[:1],
                        potion_repr=state.potion_repr[:1],
                        monster_repr=state.monster_repr[:1],
                        choice_repr=state.choice_repr[:1],
                        action_type_logits=state.action_type_logits[:1],
                        card_logits=state.card_logits[:1],
                        potion_logits=state.potion_logits[:1],
                        choice_logits=state.choice_logits[:1],
                    ),
                    chosen_potion,
                )[0]
                target_probs = _masked_probabilities(
                    target_logits,
                    obs_tensor["use_potion_target_mask"][0, int(slot)],
                ).detach().cpu().numpy()
                potion_target_probs[int(slot)] = target_probs
            if action_type == ACTION_TYPE_USE_POTION:
                chosen_slot = int(potion_probs.argmax()) if deterministic else int(torch.distributions.Categorical(probs=potion_probs_t).sample().item())
                action["potion_slot"] = chosen_slot
                target_probs = potion_target_probs.get(chosen_slot)
                if target_probs is not None:
                    action["target_slot"] = int(target_probs.argmax())

        if float(obs["choice_option_mask"].sum()) > 0:
            choice_probs_t = _masked_probabilities(state.choice_logits[0], obs_tensor["choice_option_mask"][0])
            choice_probs = choice_probs_t.detach().cpu().numpy()
            if action_type == ACTION_TYPE_CHOICE:
                action["choice_index"] = int(choice_probs.argmax()) if deterministic else int(torch.distributions.Categorical(probs=choice_probs_t).sample().item())

        probe_probs = torch.sigmoid(state.probes[0]).detach().cpu().numpy()
        value = float(state.value[0].item())

    payload = (env._last_response or {}).get("payload") or {}
    legal_candidates = [
        candidate
        for candidate, legal in zip(payload.get("action_candidates") or [], payload.get("action_mask") or [])
        if legal
    ]

    distributions = {
        "action_type_probs": action_type_probs,
        "card_probs": card_probs,
        "potion_probs": potion_probs,
        "choice_probs": choice_probs,
        "card_target_probs": card_target_probs,
        "potion_target_probs": potion_target_probs,
    }
    legal_candidate_rows = []
    for candidate in legal_candidates:
        canonical = env.candidate_to_canonical(candidate)
        legal_candidate_rows.append(
            {
                "label": candidate.get("label"),
                "action_family": candidate.get("action_family"),
                "canonical": canonical,
                "derived_probability": _candidate_probability(candidate, canonical, distributions),
            }
        )
    legal_candidate_rows.sort(key=lambda row: row["derived_probability"], reverse=True)

    head_summary = {
        "value": value,
        "probe_probs": [
            {"name": PROBE_LABELS[index], "prob": float(prob)}
            for index, prob in enumerate(probe_probs)
        ],
        "action_type_topk": _topk_named(
            action_type_probs,
            [ACTION_TYPE_LABELS[index] for index in range(len(action_type_probs))],
            top_k,
        ),
        "card_topk": _topk_named(card_probs, _hand_names(raw_observation, len(card_probs)), top_k),
        "potion_topk": _topk_named(
            potion_probs,
            _potion_names(raw_observation, len(potion_probs)),
            top_k,
        ),
        "choice_topk": _topk_named(
            choice_probs,
            _choice_names(raw_observation, len(choice_probs)),
            top_k,
        ),
        "card_target_topk": {
            str(slot): _topk_named(probs, _monster_names(raw_observation), top_k)
            for slot, probs in card_target_probs.items()
        },
        "potion_target_topk": {
            str(slot): _topk_named(probs, _monster_names(raw_observation), top_k)
            for slot, probs in potion_target_probs.items()
        },
        "legal_candidate_topk": legal_candidate_rows[:top_k],
    }
    return action, head_summary


def load_model(model_path: Path, device: torch.device) -> StructuredPolicyNet:
    checkpoint = torch.load(model_path, map_location=device)
    config = dict(checkpoint.get("config") or {})
    model = StructuredPolicyNet(
        card_vocab=int(config["card_vocab"]),
        potion_vocab=int(config["potion_vocab"]),
        power_vocab=int(config["power_vocab"]),
        monster_vocab=int(config["monster_vocab"]),
        intent_vocab=int(config["intent_vocab"]),
    ).to(device)
    model.load_state_dict(checkpoint["model_state"])
    model.eval()
    return model


def _safe_json_for_html(value: Any) -> str:
    return json.dumps(value, ensure_ascii=False).replace("</", "<\\/")


def render_html_report(trace: dict[str, Any]) -> str:
    trace_json = _safe_json_for_html(trace)
    title = f"Structured Policy Trace: {trace['spec_name']}"
    css = """
:root{
  color-scheme:light;
  --bg:#efe7da;
  --bg-2:#e5dbc9;
  --panel:#fffaf2;
  --panel-2:#f7f0e2;
  --text:#231d15;
  --muted:#776a59;
  --line:#d9cbb5;
  --line-strong:#bca88b;
  --accent:#2d7a68;
  --accent-2:#dfeee8;
  --warn:#934c37;
  --warn-2:#f8e3db;
  --good:#35623a;
  --good-2:#e3f1e2;
  --card:#f5ecd9;
   --card-new:#eef9f4;
   --card-removed:#fff0e7;
   --shadow:0 14px 28px rgba(62,44,18,.08);
}
*{box-sizing:border-box}
body{
  margin:0;
  font-family:"Segoe UI",Arial,sans-serif;
  background:radial-gradient(circle at top,#f6f0e7 0%,var(--bg) 45%,var(--bg-2) 100%);
  color:var(--text);
}
.page{max-width:1580px;margin:0 auto;padding:20px;}
.topbar{
  position:sticky;top:0;z-index:20;
  background:rgba(255,250,242,.95);
  backdrop-filter:blur(10px);
  border:1px solid var(--line);
  border-radius:18px;
  padding:18px;
  box-shadow:var(--shadow);
  margin-bottom:18px;
}
.topbar-head{
  display:flex;
  justify-content:space-between;
  gap:18px;
  align-items:flex-start;
  flex-wrap:wrap;
}
.title-block h1{margin:0;font-size:30px;line-height:1.1;}
.subtitle{margin-top:6px;color:var(--muted);}
.meta-row{display:flex;gap:8px;flex-wrap:wrap;margin-top:10px;}
.pill{
  display:inline-block;
  border-radius:999px;
  padding:5px 11px;
  background:#f2e7d4;
  border:1px solid var(--line);
  font-size:12px;
  font-weight:600;
}
.transport{
  display:flex;
  flex-direction:column;
  gap:10px;
  min-width:min(680px,100%);
  align-items:flex-end;
}
.transport-buttons{
  display:flex;
  gap:8px;
  align-items:center;
  flex-wrap:wrap;
}
button{
  border:1px solid var(--line-strong);
  background:var(--panel);
  color:var(--text);
  border-radius:10px;
  padding:8px 12px;
  font-weight:700;
  cursor:pointer;
}
button:hover{background:#fff}
button.is-active{
  background:var(--accent);
  color:white;
  border-color:var(--accent);
}
.slider-row{
  width:min(680px,100%);
  display:grid;
  grid-template-columns:auto 1fr auto;
  gap:10px;
  align-items:center;
}
.slider-row input[type=range]{width:100%}
.action-banner{
  margin-top:14px;
  background:linear-gradient(135deg,#fef7ea 0%,#f4ead6 100%);
  border:1px solid var(--line);
  border-radius:16px;
  padding:14px 16px;
}
.banner-eyebrow{font-size:12px;font-weight:700;color:var(--muted);text-transform:uppercase;letter-spacing:.05em;}
.banner-title{font-size:28px;font-weight:800;margin-top:4px;}
.banner-meta{display:flex;gap:8px;flex-wrap:wrap;margin-top:10px;}
.viewer-layout{
  display:grid;
  grid-template-columns:minmax(0,1fr) 360px;
  gap:18px;
  align-items:start;
}
.panel{
  background:var(--panel);
  border:1px solid var(--line);
  border-radius:18px;
  box-shadow:var(--shadow);
}
.main-column{display:flex;flex-direction:column;gap:18px;}
.cue-panel,.stage-panel,.advanced-panel{padding:18px;}
.stage-panel{padding:18px;}
.stage-grid{display:grid;grid-template-columns:320px minmax(0,1fr);gap:16px;align-items:start;}
.detail-grid{display:grid;grid-template-columns:repeat(2,minmax(0,1fr));gap:18px;}
.player-card,.summary-card,.log-card,.timeline-card,.choice-card,.cue-card{
  background:var(--panel-2);
  border:1px solid var(--line);
  border-radius:16px;
  padding:14px;
}
.section-title{font-size:18px;font-weight:800;margin:0 0 12px 0;}
.subsection-title{font-size:14px;font-weight:800;margin:0 0 8px 0;color:var(--muted);text-transform:uppercase;letter-spacing:.04em;}
.player-stats{
  display:grid;
  grid-template-columns:repeat(3,minmax(0,1fr));
  gap:10px;
}
.stat-tile{
  background:var(--panel);
  border:1px solid var(--line);
  border-radius:14px;
  padding:12px;
}
.stat-label{font-size:12px;color:var(--muted);text-transform:uppercase;letter-spacing:.04em;}
.stat-value{margin-top:4px;font-size:24px;font-weight:800;}
.stat-delta,.pile-delta,.inline-delta{
  display:inline-block;
  margin-left:6px;
  font-size:12px;
  font-weight:800;
}
.chip-row{display:flex;flex-wrap:wrap;gap:8px;margin-top:10px;}
.chip{
  display:inline-flex;
  align-items:center;
  gap:6px;
  padding:6px 10px;
  border-radius:999px;
  border:1px solid var(--line);
  background:#f3ebdf;
  font-size:12px;
  font-weight:700;
}
.chip.buff{background:var(--accent-2);border-color:#b7d7cd;color:#165447;}
.chip.debuff{background:var(--warn-2);border-color:#e1baac;color:#7d3021;}
.chip.neutral{background:#efe7da;}
.chip.new{background:#e5f5ed;border-color:#afd3bf;color:#215647;}
.chip.removed{background:#f8e7df;border-color:#dfb8aa;color:#7a3426;}
.chip-detail{font-weight:600;color:var(--muted);}
.piles-row{
  margin-top:12px;
  display:grid;
  grid-template-columns:repeat(3,minmax(0,1fr));
  gap:10px;
}
.pile-box{
  background:var(--panel);
  border:1px solid var(--line);
  border-radius:12px;
  padding:10px;
  text-align:center;
}
.pile-label{font-size:12px;color:var(--muted);text-transform:uppercase;}
.pile-value{font-size:22px;font-weight:800;margin-top:2px;}
.summary-block{margin-top:14px;padding-top:14px;border-top:1px solid #e6d8c3;}
.summary-block:first-of-type{margin-top:0;padding-top:0;border-top:none;}
.summary-row{
  display:flex;
  justify-content:space-between;
  gap:8px;
  padding:7px 0;
  border-top:1px solid #e8dcc9;
}
.summary-row:first-of-type{border-top:none;}
.summary-label{margin-top:8px;font-size:12px;color:var(--muted);text-transform:uppercase;letter-spacing:.04em;}
.transition-note{
  margin-top:8px;
  padding:10px 12px;
  border-radius:12px;
  background:var(--panel);
  border:1px solid var(--line);
  font-weight:700;
}
.monster-grid{
  display:grid;
  grid-template-columns:repeat(auto-fit,minmax(220px,1fr));
  gap:12px;
}
.monster-card{
  background:var(--panel-2);
  border:2px solid var(--line);
  border-radius:16px;
  padding:14px;
}
.monster-card.is-targeted{
  border-color:#d79255;
  background:linear-gradient(180deg,#fff1de 0%,var(--panel-2) 100%);
  box-shadow:0 0 0 3px rgba(215,146,85,.18);
}
.monster-top{
  display:flex;
  justify-content:space-between;
  gap:8px;
  align-items:flex-start;
}
.monster-name{font-size:20px;font-weight:800;}
.monster-sub{font-size:12px;color:var(--muted);margin-top:2px;}
.target-badge{
  background:#e8904c;
  color:white;
  font-size:11px;
  font-weight:800;
  border-radius:999px;
  padding:5px 9px;
}
.monster-stats{
  display:flex;
  gap:10px;
  margin-top:12px;
  flex-wrap:wrap;
}
.monster-stat{
  min-width:84px;
  background:var(--panel);
  border:1px solid var(--line);
  border-radius:12px;
  padding:8px 10px;
}
.monster-intent{
  margin-top:12px;
  background:var(--panel);
  border:1px solid var(--line);
  border-radius:12px;
  padding:10px;
  font-weight:700;
}
.delta-label{color:var(--muted);}
.delta-value{font-weight:800;}
.delta-pos{color:var(--good)}
.delta-neg{color:var(--warn)}
.log-list{
  list-style:none;
  padding:0;
  margin:0;
  display:flex;
  flex-direction:column;
  gap:8px;
}
.log-item{
  background:var(--panel);
  border:1px solid var(--line);
  border-radius:12px;
  padding:10px 12px;
}
.hand-panel,.choice-panel{
  padding:18px;
}
.cards-row{
  display:flex;
  gap:12px;
  overflow:auto;
  padding-bottom:4px;
}
.card{
  min-width:170px;
  max-width:170px;
  background:var(--card);
  border:2px solid #c8b596;
  border-radius:18px;
  padding:14px 12px;
  box-shadow:0 8px 14px rgba(53,38,16,.06);
}
.card.is-selected{border-color:#d59a3f;background:linear-gradient(180deg,#fff5d8 0%,#fff0c4 100%);transform:translateY(-4px);}
.card.is-new{border-color:#5ca98a;background:linear-gradient(180deg,#f4fbf8 0%,var(--card-new) 100%);}
.card.is-removed{border-color:#d59a7a;background:linear-gradient(180deg,#fff7f3 0%,var(--card-removed) 100%);}
.card.is-disabled{opacity:.65}
.card-cost{
  width:34px;height:34px;border-radius:50%;
  background:#fffaf1;border:2px solid #c4ac82;
  display:flex;align-items:center;justify-content:center;
  font-weight:900;font-size:18px;
}
.card-name{margin-top:12px;font-size:18px;font-weight:800;line-height:1.15;}
.card-meta{margin-top:10px;font-size:12px;color:var(--muted);}
.card-tags{display:flex;flex-wrap:wrap;gap:6px;margin-top:10px;}
.tag{
  border-radius:999px;
  padding:4px 8px;
  background:#f2e8d4;
  border:1px solid #d8c5a6;
  font-size:11px;
  font-weight:700;
}
.tag-new{background:#e5f5ed;border-color:#afd3bf;color:#215647;}
.tag-removed{background:#f8e7df;border-color:#dfb8aa;color:#7a3426;}
.timeline-panel{
  position:sticky;
  top:178px;
  padding:16px;
}
.timeline-list{
  display:flex;
  flex-direction:column;
  gap:8px;
  max-height:calc(100vh - 270px);
  overflow:auto;
  margin-top:12px;
}
.timeline-item{
  width:100%;
  text-align:left;
  border-radius:14px;
  border:1px solid var(--line);
  background:var(--panel-2);
  padding:12px;
}
.timeline-item.is-current{
  background:#fff5dd;
  border-color:#d59a3f;
  box-shadow:0 0 0 3px rgba(213,154,63,.14);
}
.timeline-step{font-size:12px;color:var(--muted);text-transform:uppercase;font-weight:700;}
.timeline-action{margin-top:4px;font-weight:800;}
.timeline-reward{margin-top:6px;font-size:12px;color:var(--muted);}
.advanced-panel{
  padding:16px;
}
.advanced-tabs{
  display:flex;
  gap:8px;
  flex-wrap:wrap;
  margin-bottom:12px;
}
.tab-button{
  border-radius:999px;
  padding:7px 12px;
  border:1px solid var(--line);
  background:#f0e6d6;
  font-weight:800;
}
.tab-button.is-active{
  background:var(--accent);
  color:white;
  border-color:var(--accent);
}
.advanced-content pre{
  margin:0;
  white-space:pre-wrap;
  word-break:break-word;
  background:#f7f1e8;
  border:1px solid var(--line);
  border-radius:12px;
  padding:12px;
  font-family:Consolas,monospace;
  font-size:12px;
}
.advanced-table{
  width:100%;
  border-collapse:collapse;
}
.advanced-table th,.advanced-table td{
  padding:8px 10px;
  border-top:1px solid #e4d8c4;
  text-align:left;
  vertical-align:top;
  word-break:break-word;
}
.advanced-table thead th{
  background:#f3eadb;
  border-top:none;
}
.empty-state{
  color:var(--muted);
  font-style:italic;
}
@media (max-width:1280px){
  .viewer-layout{grid-template-columns:1fr;}
  .timeline-panel{position:static;}
  .detail-grid{grid-template-columns:1fr;}
}
@media (max-width:980px){
  .stage-grid{grid-template-columns:1fr;}
  .player-stats{grid-template-columns:repeat(2,minmax(0,1fr));}
  .transport{align-items:stretch;min-width:unset;width:100%;}
  .slider-row{grid-template-columns:1fr;}
  .slider-row span:last-child{justify-self:end;}
}
"""
    js = """
const ACTION_END_TURN = __ACTION_END_TURN__;
const ACTION_PLAY_CARD = __ACTION_PLAY_CARD__;
const ACTION_USE_POTION = __ACTION_USE_POTION__;
const ACTION_CHOICE = __ACTION_CHOICE__;
const ACTION_PROCEED = __ACTION_PROCEED__;
const ACTION_CANCEL = __ACTION_CANCEL__;

const trace = JSON.parse(document.getElementById("trace-data").textContent);
const state = {
  currentStep: 0,
  playing: false,
  timer: null,
  activeTab: "policy",
};

const els = {
  stepIndicator: document.getElementById("step-indicator"),
  turnIndicator: document.getElementById("turn-indicator"),
  modeIndicator: document.getElementById("mode-indicator"),
  outcomeIndicator: document.getElementById("outcome-indicator"),
  actionBanner: document.getElementById("action-banner"),
  actionMeta: document.getElementById("action-meta"),
  prevBtn: document.getElementById("prev-btn"),
  nextBtn: document.getElementById("next-btn"),
  playBtn: document.getElementById("play-btn"),
  slider: document.getElementById("step-slider"),
  contextPanel: document.getElementById("context-panel"),
  playerPanel: document.getElementById("player-panel"),
  monsterStage: document.getElementById("monster-stage"),
  summaryPanel: document.getElementById("summary-panel"),
  logPanel: document.getElementById("log-panel"),
  handSection: document.getElementById("hand-section"),
  choiceSection: document.getElementById("choice-section"),
  timelineList: document.getElementById("timeline-list"),
  advancedContent: document.getElementById("advanced-content"),
  tabButtons: Array.from(document.querySelectorAll(".tab-button")),
};

function escapeHtml(value) {
  return String(value ?? "")
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;")
    .replaceAll('"', "&quot;")
    .replaceAll("'", "&#39;");
}

function toNumber(value, fallback = 0) {
  const parsed = Number(value);
  return Number.isFinite(parsed) ? parsed : fallback;
}

function formatFloat(value, digits = 2) {
  if (value === null || value === undefined || Number.isNaN(Number(value))) {
    return "n/a";
  }
  return Number(value).toFixed(digits);
}

function formatDelta(value) {
  const num = toNumber(value, 0);
  if (!num) return "0";
  return num > 0 ? `+${num}` : `${num}`;
}

function deltaClass(value) {
  const num = toNumber(value, 0);
  if (num > 0) return "delta-pos";
  if (num < 0) return "delta-neg";
  return "";
}

function formatKind(kind) {
  const text = String(kind ?? "").trim();
  if (!text) return "Unknown";
  return text.replaceAll("_", " ").replace(/\\b\\w/g, (char) => char.toUpperCase());
}

function cleanLabel(value, fallback = "Unknown") {
  const text = String(value ?? "").trim();
  return text || fallback;
}

function inlineDelta(value) {
  return toNumber(value, 0) ? `<span class="inline-delta ${deltaClass(value)}">${escapeHtml(formatDelta(value))}</span>` : "";
}

function chipRow(chips, emptyText = "None") {
  if (!chips || !chips.length) {
    return `<div class='empty-state'>${escapeHtml(emptyText)}</div>`;
  }
  return "<div class='chip-row'>" + chips.map((chip) => {
    const tone = chip.tone || "neutral";
    const detail = chip.detail ? ` <span class="chip-detail">${escapeHtml(chip.detail)}</span>` : "";
    return `<span class="chip ${tone}">${escapeHtml(chip.label)}${detail}</span>`;
  }).join("") + "</div>";
}

function keyForCard(card, index = 0) {
  if (card && card.uuid !== null && card.uuid !== undefined) {
    return `card:${card.uuid}`;
  }
  return `card:${card?.slot ?? index}:${card?.card_id ?? ""}:${card?.name ?? ""}`;
}

function keyForPotion(potion, index = 0) {
  if (potion && potion.uuid !== null && potion.uuid !== undefined) {
    return `potion:${potion.uuid}`;
  }
  return `potion:${potion?.slot ?? index}:${potion?.potion_id ?? ""}:${potion?.name ?? ""}`;
}

function keyForMonster(monster, index = 0) {
  if (monster && monster.entity_id !== null && monster.entity_id !== undefined) {
    return `monster:${monster.entity_id}`;
  }
  return `monster:${monster?.slot ?? index}:${monster?.monster_id ?? ""}`;
}

function buildMap(items, keyFn) {
  const map = new Map();
  (items || []).forEach((item, index) => {
    map.set(keyFn(item, index), item);
  });
  return map;
}

function diffItems(beforeItems, afterItems, keyFn) {
  const beforeMap = buildMap(beforeItems, keyFn);
  const afterMap = buildMap(afterItems, keyFn);
  const added = (afterItems || []).filter((item, index) => !beforeMap.has(keyFn(item, index)));
  const removed = (beforeItems || []).filter((item, index) => !afterMap.has(keyFn(item, index)));
  return { beforeMap, afterMap, added, removed };
}

function buildPowerChips(powers) {
  return (powers || []).map((power) => {
    const amount = power.amount;
    return {
      label: amount === null || amount === undefined ? cleanLabel(power.name || power.id, "Power") : `${cleanLabel(power.name || power.id, "Power")} ${amount}`,
      tone: power.is_debuff ? "debuff" : "buff",
      detail: power.just_applied ? "new" : "",
    };
  });
}

function buildPotionChips(potions) {
  return (potions || []).map((potion) => {
    const details = [];
    if (potion.target_mode && potion.target_mode !== "none") {
      details.push(String(potion.target_mode).replaceAll("_", " "));
    }
    if (!potion.usable) {
      details.push("unusable");
    }
    return {
      label: cleanLabel(potion.name || potion.potion_id, "Potion"),
      tone: potion.usable ? "neutral" : "debuff",
      detail: details.join(" · "),
    };
  });
}

function formatIntent(intent) {
  const kind = String(intent?.kind || "unknown");
  const totalDamage = toNumber(intent?.total_damage, 0);
  const hits = toNumber(intent?.hits, 0);
  const pretty = formatKind(kind);
  if (kind.includes("attack") && totalDamage > 0) {
    return hits > 1 ? `${pretty} ${totalDamage} (${hits} hits)` : `${pretty} ${totalDamage}`;
  }
  return pretty;
}

function mechanicChips(monster, previousMonster) {
  const state = monster?.mechanic_state || {};
  const previousState = previousMonster?.mechanic_state || {};
  const chips = [];
  if ("sleeping" in state) {
    chips.push({ label: state.sleeping ? "Sleep" : "Awake", tone: "neutral" });
  }
  if ("idle_count" in state) {
    chips.push({ label: `Idle ${state.idle_count}`, tone: "neutral" });
  }
  if ("wake_triggered" in state) {
    chips.push({ label: `Wake Triggered: ${state.wake_triggered ? "Yes" : "No"}`, tone: "neutral" });
  }
  if ("debuff_turn_count" in state && toNumber(state.debuff_turn_count, 0) > 0) {
    chips.push({ label: `Debuff Cycle ${state.debuff_turn_count}`, tone: "neutral" });
  }
  if ("guardian_threshold" in state && toNumber(state.guardian_threshold, 0) > 0) {
    chips.push({ label: `Threshold ${state.guardian_threshold}`, tone: "neutral" });
  }
  if ("guardian_damage_taken" in state && toNumber(state.guardian_damage_taken, 0) > 0) {
    chips.push({ label: `Damage ${state.guardian_damage_taken}`, tone: "neutral" });
  }
  if ("guardian_open" in state) {
    chips.push({ label: state.guardian_open ? "Open" : "Closed", tone: "neutral" });
  }
  if (state.close_up_triggered) {
    chips.push({ label: "Close-Up Triggered", tone: "debuff" });
  }
  if ("split_threshold" in state && toNumber(state.split_threshold, 0) > 0) {
    chips.push({ label: `Split <= ${state.split_threshold}`, tone: "neutral" });
  }
  if (state.split_ready) {
    chips.push({ label: "Split Ready", tone: "debuff" });
  }
  if ("regrow_counter" in state) {
    chips.push({ label: `Regrow ${state.regrow_counter}`, tone: "neutral" });
  }
  if (state.half_dead) {
    chips.push({ label: "Half Dead", tone: "debuff" });
  }
  if (state.first_move) {
    chips.push({ label: "First Move", tone: "neutral" });
  }
  if (!previousMonster && monster) {
    chips.push({ label: "New Spawn", tone: "new" });
  }
  if (previousState.sleeping && state.sleeping) {
    chips.push({ label: "Still Asleep", tone: "neutral" });
  } else if (previousState.sleeping && !state.sleeping) {
    chips.push({ label: "Woke Up", tone: "debuff" });
  }
  if (!previousState.split_ready && state.split_ready) {
    chips.push({ label: "Split Triggered", tone: "debuff" });
  }
  if (!previousState.half_dead && state.half_dead) {
    chips.push({ label: "Became Half Dead", tone: "debuff" });
  }
  if (monster && monster.targetable === false) {
    chips.push({ label: "Untargetable", tone: "neutral" });
  }
  if (monster && monster.alive === false) {
    chips.push({ label: "Defeated", tone: "debuff" });
  }
  return chips;
}

function describeChoiceTransition(beforeKind, afterKind) {
  if (beforeKind === afterKind) {
    return null;
  }
  if (!beforeKind && afterKind) {
    return { type: "entered", entered: afterKind, label: `Entered ${formatKind(afterKind)}` };
  }
  if (beforeKind && !afterKind) {
    return { type: "exited", exited: beforeKind, label: `Exited ${formatKind(beforeKind)}` };
  }
  if (beforeKind && afterKind) {
    return { type: "changed", before: beforeKind, after: afterKind, label: `${formatKind(beforeKind)} -> ${formatKind(afterKind)}` };
  }
  return null;
}

function actionContext(step) {
  const before = step.before || {};
  const action = step.canonical_action || {};
  const actionType = toNumber(action.action_type, 0);
  const targetSlot = toNumber(action.target_slot, 0);
  const targetMonster = targetSlot > 0 ? (before.monsters || [])[targetSlot - 1] : null;
  const targetName = targetMonster ? cleanLabel(targetMonster.name || targetMonster.monster_id, `Target ${targetSlot}`) : null;
  const targetKey = targetMonster ? keyForMonster(targetMonster, targetSlot - 1) : null;
  let actionText = step.chosen_action_label || "Action";
  let sourceCard = null;
  let sourcePotion = null;
  let sourceCardKey = null;
  let sourcePotionKey = null;
  let choiceLabel = null;
  let selectedChoiceIndex = null;

  if (actionType === ACTION_PLAY_CARD) {
    const slot = toNumber(action.card_slot, 0);
    sourceCard = (before.hand || [])[slot] || null;
    sourceCardKey = sourceCard ? keyForCard(sourceCard, slot) : null;
    const cardName = cleanLabel(sourceCard?.name || sourceCard?.card_id, `Card ${slot}`);
    actionText = `Play ${cardName}${targetName ? ` on ${targetName}` : ""}`;
  } else if (actionType === ACTION_USE_POTION) {
    const slot = toNumber(action.potion_slot, 0);
    sourcePotion = (before.potions || [])[slot] || null;
    sourcePotionKey = sourcePotion ? keyForPotion(sourcePotion, slot) : null;
    const potionName = cleanLabel(sourcePotion?.name || sourcePotion?.potion_id, `Potion ${slot}`);
    actionText = `Use ${potionName}${targetName ? ` on ${targetName}` : ""}`;
  } else if (actionType === ACTION_CHOICE) {
    selectedChoiceIndex = toNumber(action.choice_index, 0);
    const options = before.pending_choice?.options || [];
    const option = options[selectedChoiceIndex];
    choiceLabel = cleanLabel(option?.label || option?.card_id, `Option ${selectedChoiceIndex}`);
    actionText = `Choose ${choiceLabel}`;
  } else if (actionType === ACTION_END_TURN) {
    actionText = "End Turn";
  } else if (actionType === ACTION_PROCEED) {
    actionText = "Proceed";
  } else if (actionType === ACTION_CANCEL) {
    actionText = "Cancel";
  }

  return {
    actionType,
    actionText,
    targetName,
    targetKey,
    sourceCard,
    sourceCardKey,
    sourcePotion,
    sourcePotionKey,
    choiceLabel,
    selectedChoiceIndex,
  };
}

function buildBattleLog(step, presentation, beforeMonsterMap) {
  const lines = [presentation.context.actionText];
  if (presentation.context.targetName) {
    lines.push(`Targeted ${presentation.context.targetName}.`);
  }
  if (presentation.player.block_delta > 0) {
    lines.push(`Player gained ${presentation.player.block_delta} Block.`);
  } else if (presentation.player.block_delta < 0) {
    lines.push(`Player lost ${Math.abs(presentation.player.block_delta)} Block.`);
  }
  if (presentation.player.hp_delta > 0) {
    lines.push(`Player healed ${presentation.player.hp_delta} HP.`);
  } else if (presentation.player.hp_delta < 0) {
    lines.push(`Player lost ${Math.abs(presentation.player.hp_delta)} HP.`);
  }
  if (presentation.player.energy_delta !== 0) {
    lines.push(`Energy ${formatDelta(presentation.player.energy_delta)}.`);
  }

  const newCards = presentation.hand.cards.filter((card) => card.isNew).map((card) => cleanLabel(card.name || card.card_id, "Card"));
  if (newCards.length) {
    lines.push(`Added to hand: ${newCards.join(", ")}.`);
  }

  const extraRemovedCards = presentation.hand.removed.filter((card) => !card.wasPlayed).map((card) => cleanLabel(card.name || card.card_id, "Card"));
  if (extraRemovedCards.length) {
    lines.push(`Removed from hand: ${extraRemovedCards.join(", ")}.`);
  }

  const usedPotions = presentation.player.removed_potions.filter((potion) => potion.wasUsed).map((potion) => cleanLabel(potion.name || potion.potion_id, "Potion"));
  if (usedPotions.length) {
    lines.push(`Potion spent: ${usedPotions.join(", ")}.`);
  }

  presentation.monsters.forEach((monster) => {
    const name = cleanLabel(monster.name || monster.monster_id, `Monster ${monster.slot}`);
    if (monster.hp_delta < 0) {
      lines.push(`${name} lost ${Math.abs(monster.hp_delta)} HP.`);
    } else if (monster.hp_delta > 0) {
      lines.push(`${name} healed ${monster.hp_delta} HP.`);
    }
    if (monster.block_delta < 0) {
      lines.push(`${name} lost ${Math.abs(monster.block_delta)} Block.`);
    } else if (monster.block_delta > 0) {
      lines.push(`${name} gained ${monster.block_delta} Block.`);
    }
    const previousState = beforeMonsterMap.get(monster.key)?.mechanic_state || {};
    const state = monster.mechanic_state || {};
    if (previousState.sleeping && state.sleeping) {
      lines.push(`${name} remains asleep.`);
    } else if (previousState.sleeping && !state.sleeping) {
      lines.push(`${name} woke up.`);
    }
    if (!previousState.split_ready && state.split_ready) {
      lines.push(`${name} reached split threshold.`);
    }
    if (!previousState.half_dead && state.half_dead) {
      lines.push(`${name} became half dead.`);
    }
  });

  presentation.removedMonsters.forEach((monster) => {
    lines.push(`${cleanLabel(monster.name || monster.monster_id, "Monster")} left the field.`);
  });

  if (presentation.choiceTransition) {
    lines.push(`${presentation.choiceTransition.label}.`);
  }
  return lines;
}

function buildPresentation(step, index) {
  const before = step.before || {};
  const after = step.after || {};
  const context = actionContext(step);
  const cardDiff = diffItems(before.hand || [], after.hand || [], keyForCard);
  const potionDiff = diffItems(before.potions || [], after.potions || [], keyForPotion);
  const monsterDiff = diffItems(before.monsters || [], after.monsters || [], keyForMonster);
  const playerBefore = before.player || {};
  const playerAfter = after.player || {};

  const handCards = (after.hand || []).map((card, indexInHand) => {
    const key = keyForCard(card, indexInHand);
    return {
      ...card,
      key,
      isNew: !cardDiff.beforeMap.has(key),
    };
  });
  const removedCards = (before.hand || []).filter((card, indexInHand) => !cardDiff.afterMap.has(keyForCard(card, indexInHand))).map((card, indexInHand) => {
    const key = keyForCard(card, indexInHand);
    return {
      ...card,
      key,
      wasPlayed: key === context.sourceCardKey,
    };
  });

  const afterPotions = (after.potions || []).map((potion, indexInBelt) => {
    const key = keyForPotion(potion, indexInBelt);
    return {
      ...potion,
      key,
      isNew: !potionDiff.beforeMap.has(key),
    };
  });
  const removedPotions = (before.potions || []).filter((potion, indexInBelt) => !potionDiff.afterMap.has(keyForPotion(potion, indexInBelt))).map((potion, indexInBelt) => {
    const key = keyForPotion(potion, indexInBelt);
    return {
      ...potion,
      key,
      wasUsed: key === context.sourcePotionKey,
    };
  });

  const afterMonsters = (after.monsters || []).map((monster, indexInField) => {
    const key = keyForMonster(monster, indexInField);
    const previousMonster = monsterDiff.beforeMap.get(key);
    return {
      ...monster,
      key,
      hp_delta: toNumber(monster.hp, 0) - toNumber(previousMonster?.hp, 0),
      block_delta: toNumber(monster.block, 0) - toNumber(previousMonster?.block, 0),
      targeted: context.targetKey === key,
      intent_text: formatIntent(monster.intent),
      power_chips: buildPowerChips(monster.powers),
      mechanic_chips: mechanicChips(monster, previousMonster),
    };
  });
  const removedMonsters = (before.monsters || []).filter((monster, indexInField) => !monsterDiff.afterMap.has(keyForMonster(monster, indexInField)));

  const choiceTransition = describeChoiceTransition(before.pending_choice_kind, after.pending_choice_kind);
  const presentation = {
    index,
    turn: toNumber(after.turn_count ?? before.turn_count, 0),
    context,
    player: {
      hp: playerAfter.hp,
      max_hp: playerAfter.max_hp,
      block: playerAfter.block,
      energy: playerAfter.energy,
      hp_delta: toNumber(playerAfter.hp, 0) - toNumber(playerBefore.hp, 0),
      block_delta: toNumber(playerAfter.block, 0) - toNumber(playerBefore.block, 0),
      energy_delta: toNumber(playerAfter.energy, 0) - toNumber(playerBefore.energy, 0),
      powers: buildPowerChips(after.player_powers || []),
      potions: buildPotionChips(afterPotions),
      removed_potions: removedPotions,
      piles: {
        draw: after.draw_count,
        discard: after.discard_count,
        exhaust: after.exhaust_count,
        draw_delta: toNumber(after.draw_count, 0) - toNumber(before.draw_count, 0),
        discard_delta: toNumber(after.discard_count, 0) - toNumber(before.discard_count, 0),
        exhaust_delta: toNumber(after.exhaust_count, 0) - toNumber(before.exhaust_count, 0),
      },
    },
    monsters: afterMonsters,
    removedMonsters,
    hand: {
      cards: handCards,
      removed: removedCards,
    },
    choice: after.pending_choice || null,
    choiceTransition,
  };
  presentation.logs = buildBattleLog(step, presentation, monsterDiff.beforeMap);
  return presentation;
}

function renderContext(presentation) {
  const cues = [];
  if (presentation.context.sourceCard) {
    cues.push({ label: `Played from hand: ${cleanLabel(presentation.context.sourceCard.name || presentation.context.sourceCard.card_id, "Card")}`, tone: "removed" });
  }
  if (presentation.context.sourcePotion) {
    cues.push({ label: `Used potion: ${cleanLabel(presentation.context.sourcePotion.name || presentation.context.sourcePotion.potion_id, "Potion")}`, tone: "removed" });
  }
  if (presentation.context.choiceLabel) {
    cues.push({ label: `Selected: ${presentation.context.choiceLabel}`, tone: "neutral" });
  }
  if (presentation.context.targetName) {
    cues.push({ label: `Targeted: ${presentation.context.targetName}`, tone: "neutral" });
  }
  return `
    <div class="cue-card">
      <h2 class="section-title">Before Context</h2>
      ${chipRow(cues, "No auxiliary pre-action context for this step.")}
    </div>
  `;
}

function renderPlayer(panel) {
  const stats = [
    ["HP", `${panel.hp}/${panel.max_hp}`, panel.hp_delta],
    ["Block", panel.block, panel.block_delta],
    ["Energy", panel.energy, panel.energy_delta],
  ];
  const statHtml = stats.map(([label, value, delta]) => `
    <div class="stat-tile">
      <div class="stat-label">${escapeHtml(label)}</div>
      <div class="stat-value">${escapeHtml(value)}${inlineDelta(delta)}</div>
    </div>
  `).join("");
  const piles = panel.piles || {};
  const pileHtml = [["Draw", piles.draw, piles.draw_delta], ["Discard", piles.discard, piles.discard_delta], ["Exhaust", piles.exhaust, piles.exhaust_delta]].map(([label, value, delta]) => `
    <div class="pile-box">
      <div class="pile-label">${escapeHtml(label)}</div>
      <div class="pile-value">${escapeHtml(value)}${toNumber(delta, 0) ? `<span class="pile-delta ${deltaClass(delta)}">${escapeHtml(formatDelta(delta))}</span>` : ""}</div>
    </div>
  `).join("");
  return `
    <div class="player-card">
      <h2 class="section-title">Player</h2>
      <div class="player-stats">${statHtml}</div>
      <h3 class="section-title" style="margin-top:14px;">Powers</h3>
      ${chipRow(panel.powers, "No powers.")}
      <h3 class="section-title" style="margin-top:14px;">Potions</h3>
      ${chipRow(panel.potions, "No potions.")}
      <div class="summary-label">Spent This Step</div>
      ${chipRow(panel.removed_potions.map((potion) => ({ label: cleanLabel(potion.name || potion.potion_id, "Potion"), tone: "removed" })), "No potion spent.")}
      <div class="piles-row">${pileHtml}</div>
    </div>
  `;
}

function renderMonster(monster) {
  return `
    <div class="monster-card ${monster.targeted ? "is-targeted" : ""}">
      <div class="monster-top">
        <div>
          <div class="monster-name">${escapeHtml(monster.name || monster.monster_id || "Monster")}</div>
          <div class="monster-sub">${escapeHtml(monster.monster_id || "")}</div>
        </div>
        ${monster.targeted ? "<div class='target-badge'>TARGET</div>" : ""}
      </div>
      <div class="monster-stats">
        <div class="monster-stat">
          <div class="stat-label">HP</div>
          <div class="stat-value">${escapeHtml(monster.hp)}/${escapeHtml(monster.max_hp)}${inlineDelta(monster.hp_delta)}</div>
        </div>
        <div class="monster-stat">
          <div class="stat-label">Block</div>
          <div class="stat-value">${escapeHtml(monster.block)}${inlineDelta(monster.block_delta)}</div>
        </div>
      </div>
      <div class="monster-intent">${escapeHtml(monster.intent_text || "Unknown Intent")}</div>
      <h3 class="section-title" style="margin-top:12px;">Mechanics</h3>
      ${chipRow(monster.mechanic_chips, "No mechanic markers.")}
      <h3 class="section-title" style="margin-top:12px;">Powers</h3>
      ${chipRow(monster.power_chips, "No powers.")}
    </div>
  `;
}

function renderHandCards(cards) {
  if (!cards || !cards.length) {
    return "<div class='empty-state'>No cards available.</div>";
  }
  return "<div class='cards-row'>" + cards.map((card) => {
    const tags = [];
    if (card.isNew) tags.push({ label: "NEW", className: "tag-new" });
    tags.push(card.playable ? { label: "Playable", tone: "buff" } : { label: "Locked", tone: "debuff" });
    if (card.target_mode && card.target_mode !== "none") tags.push({ label: card.target_mode.replaceAll("_", " "), tone: "neutral" });
    if (card.upgraded) tags.push({ label: "Upgraded", tone: "neutral" });
    if (card.exhausts_when_played) tags.push({ label: "Exhaust", tone: "debuff" });
    if (card.ethereal) tags.push({ label: "Ethereal", tone: "debuff" });
    if (card.retain) tags.push({ label: "Retain", tone: "neutral" });
    return `
      <div class="card ${card.isNew ? "is-new" : ""} ${card.playable ? "" : "is-disabled"}">
        <div class="card-cost">${escapeHtml(card.cost)}</div>
        <div class="card-name">${escapeHtml(card.name || card.card_id || `Card ${card.slot}`)}</div>
        <div class="card-meta">Slot ${escapeHtml(card.slot)} · ${escapeHtml(card.card_id || "")}${card.uuid !== undefined ? ` · uuid ${escapeHtml(card.uuid)}` : ""}</div>
        <div class="card-tags">
          ${tags.map((tag) => `<span class="tag ${escapeHtml(tag.className || "")}">${escapeHtml(tag.label)}</span>`).join("")}
        </div>
      </div>
    `;
  }).join("") + "</div>";
}

function renderChoicePanel(choicePanel, presentation) {
  if (!choicePanel) return "";
  const options = choicePanel.options || [];
  return `
    <div class="choice-card">
      <h2 class="section-title">Choice After Action</h2>
      <div class="meta-row">
        <span class="pill">${escapeHtml(choicePanel.kind || "choice")}</span>
        <span class="pill">min ${escapeHtml(choicePanel.min_select)}</span>
        <span class="pill">max ${escapeHtml(choicePanel.max_select)}</span>
        <span class="pill">${choicePanel.can_cancel ? "can cancel" : "must choose"}</span>
      </div>
      ${presentation.choiceTransition ? `<p class="subtitle">${escapeHtml(presentation.choiceTransition.label)}</p>` : (choicePanel.reason ? `<p class="subtitle">${escapeHtml(choicePanel.reason)}</p>` : "")}
      <div class="cards-row" style="margin-top:12px;">
        ${options.map((option) => `
          <div class="card ${presentation.context.selectedChoiceIndex === Number(option.option_index) ? "is-selected" : ""}">
            <div class="card-name">${escapeHtml(option.label || `Option ${option.option_index}`)}</div>
            <div class="card-meta">Index ${escapeHtml(option.option_index)}</div>
            <div class="card-tags">
              ${option.card_id ? `<span class="tag">${escapeHtml(option.card_id)}</span>` : ""}
              ${(option.selection_uuids || []).length ? `<span class="tag">uuids ${(option.selection_uuids || []).length}</span>` : ""}
            </div>
          </div>
        `).join("")}
      </div>
    </div>
  `;
}

function renderSummary(presentation) {
  const addedCards = presentation.hand.cards.filter((card) => card.isNew).map((card) => ({ label: cleanLabel(card.name || card.card_id, "Card"), tone: "new" }));
  const removedCards = presentation.hand.removed.map((card) => ({ label: cleanLabel(card.name || card.card_id, "Card"), tone: card.wasPlayed ? "removed" : "neutral" }));
  const removedMonsters = presentation.removedMonsters.map((monster) => ({ label: cleanLabel(monster.name || monster.monster_id, "Monster"), tone: "removed" }));
  const spentPotions = presentation.player.removed_potions.map((potion) => ({ label: cleanLabel(potion.name || potion.potion_id, "Potion"), tone: "removed" }));
  return `
    <div class="summary-card">
      <h2 class="section-title">Resolution</h2>
      <div class="summary-block">
        <h3 class="subsection-title">Player And Piles</h3>
        <div class="summary-row"><span class="delta-label">HP</span><span class="delta-value ${deltaClass(presentation.player.hp_delta)}">${escapeHtml(formatDelta(presentation.player.hp_delta))}</span></div>
        <div class="summary-row"><span class="delta-label">Block</span><span class="delta-value ${deltaClass(presentation.player.block_delta)}">${escapeHtml(formatDelta(presentation.player.block_delta))}</span></div>
        <div class="summary-row"><span class="delta-label">Energy</span><span class="delta-value ${deltaClass(presentation.player.energy_delta)}">${escapeHtml(formatDelta(presentation.player.energy_delta))}</span></div>
        <div class="summary-row"><span class="delta-label">Draw</span><span class="delta-value ${deltaClass(presentation.player.piles.draw_delta)}">${escapeHtml(formatDelta(presentation.player.piles.draw_delta))}</span></div>
        <div class="summary-row"><span class="delta-label">Discard</span><span class="delta-value ${deltaClass(presentation.player.piles.discard_delta)}">${escapeHtml(formatDelta(presentation.player.piles.discard_delta))}</span></div>
        <div class="summary-row"><span class="delta-label">Exhaust</span><span class="delta-value ${deltaClass(presentation.player.piles.exhaust_delta)}">${escapeHtml(formatDelta(presentation.player.piles.exhaust_delta))}</span></div>
      </div>
      <div class="summary-block">
        <h3 class="subsection-title">Hand Changes</h3>
        <div class="summary-label">Added To Hand</div>
        ${chipRow(addedCards, "No new cards.")}
        <div class="summary-label">Left Hand</div>
        ${chipRow(removedCards, "Nothing left hand.")}
      </div>
      <div class="summary-block">
        <h3 class="subsection-title">Field Changes</h3>
        <div class="summary-label">Potions Spent</div>
        ${chipRow(spentPotions, "No potion spent.")}
        <div class="summary-label">Monsters Removed</div>
        ${chipRow(removedMonsters, "No monster removed.")}
        <div class="summary-label">Choice Transition</div>
        ${presentation.choiceTransition ? `<div class="transition-note">${escapeHtml(presentation.choiceTransition.label)}</div>` : "<div class='empty-state'>No choice transition.</div>"}
      </div>
    </div>
  `;
}

function renderLogPanel(lines) {
  return `
    <div class="log-card">
      <h2 class="section-title">Battle Log</h2>
      <ul class="log-list">
        ${(lines || []).map((line) => `<li class="log-item">${escapeHtml(line)}</li>`).join("")}
      </ul>
    </div>
  `;
}

function renderPolicyTab(step, presentation) {
  const policy = step.head_summary || {};
  const sections = [];
  const renderTable = (title, rows) => {
    if (!rows || !rows.length) return "";
    return `
      <section class="panel" style="padding:12px;">
        <h3 class="section-title" style="font-size:16px;">${escapeHtml(title)}</h3>
        <table class="advanced-table">
          <thead><tr><th>Name</th><th>Prob</th></tr></thead>
          <tbody>
            ${rows.map((row) => `<tr><td>${escapeHtml(row.name)}</td><td>${escapeHtml((toNumber(row.prob, 0) * 100).toFixed(1))}%</td></tr>`).join("")}
          </tbody>
        </table>
      </section>
    `;
  };
  sections.push(renderTable("Action Type", policy.action_type_topk || []));
  sections.push(renderTable("Card Choice", policy.card_topk || []));
  sections.push(renderTable("Potion Choice", policy.potion_topk || []));
  sections.push(renderTable("Choice Option", policy.choice_topk || []));
  sections.push(renderTable("Probe Signals", policy.probe_probs || []));

  if (presentation.context.actionType === ACTION_PLAY_CARD) {
    const slot = toNumber(step.canonical_action?.card_slot, 0);
    sections.push(renderTable(`Target For Card Slot ${slot}`, (policy.card_target_topk || {})[String(slot)] || []));
  } else if (presentation.context.actionType === ACTION_USE_POTION) {
    const slot = toNumber(step.canonical_action?.potion_slot, 0);
    sections.push(renderTable(`Target For Potion Slot ${slot}`, (policy.potion_target_topk || {})[String(slot)] || []));
  }

  return sections.filter(Boolean).join("") || "<div class='empty-state'>No policy diagnostics for this step.</div>";
}

function renderCandidatesTab(step) {
  const rows = step.head_summary?.legal_candidate_topk || [];
  if (!rows.length) {
    return "<div class='empty-state'>No candidate summary available.</div>";
  }
  return `
    <table class="advanced-table">
      <thead><tr><th>Rank</th><th>Label</th><th>Family</th><th>Canonical</th><th>Derived Prob</th></tr></thead>
      <tbody>
        ${rows.map((row, index) => `
          <tr>
            <td>${index + 1}</td>
            <td>${escapeHtml(row.label)}</td>
            <td>${escapeHtml(row.action_family)}</td>
            <td><pre>${escapeHtml(JSON.stringify(row.canonical, null, 2))}</pre></td>
            <td>${escapeHtml((toNumber(row.derived_probability, 0) * 100).toFixed(2))}%</td>
          </tr>
        `).join("")}
      </tbody>
    </table>
  `;
}

function renderRewardTab(step) {
  const reward = step.reward_breakdown || {};
  const entries = Object.entries(reward);
  if (!entries.length) {
    return "<div class='empty-state'>No reward breakdown captured.</div>";
  }
  return `
    <table class="advanced-table">
      <thead><tr><th>Metric</th><th>Value</th></tr></thead>
      <tbody>
        ${entries.map(([key, value]) => `
          <tr><td>${escapeHtml(key)}</td><td>${typeof value === "number" ? escapeHtml(formatFloat(value, 3)) : escapeHtml(JSON.stringify(value))}</td></tr>
        `).join("")}
      </tbody>
    </table>
  `;
}

function renderRawTab(step) {
  return `
    <div style="display:grid;grid-template-columns:repeat(auto-fit,minmax(280px,1fr));gap:12px;">
      <div><h3 class="section-title" style="font-size:16px;">Before</h3><pre>${escapeHtml(JSON.stringify(step.before, null, 2))}</pre></div>
      <div><h3 class="section-title" style="font-size:16px;">After</h3><pre>${escapeHtml(JSON.stringify(step.after, null, 2))}</pre></div>
      <div><h3 class="section-title" style="font-size:16px;">Head Summary</h3><pre>${escapeHtml(JSON.stringify(step.head_summary, null, 2))}</pre></div>
    </div>
  `;
}

function renderAdvanced(step, presentation) {
  const tabs = {
    policy: renderPolicyTab(step, presentation),
    candidates: renderCandidatesTab(step),
    reward: renderRewardTab(step),
    raw: renderRawTab(step),
  };
  els.tabButtons.forEach((button) => {
    button.classList.toggle("is-active", button.dataset.tab === state.activeTab);
  });
  els.advancedContent.innerHTML = tabs[state.activeTab] || "";
}

function renderTimeline() {
  els.timelineList.innerHTML = presentations.map((presentation, index) => `
    <button class="timeline-item ${index === state.currentStep ? "is-current" : ""}" data-step-index="${index}">
      <div class="timeline-step">Turn ${presentation.turn} · Step ${index + 1}</div>
      <div class="timeline-action">${escapeHtml(presentation.context.actionText)}</div>
      <div class="timeline-reward">Reward ${formatFloat(trace.steps_data[index].reward)} · Value ${formatFloat(trace.steps_data[index].value, 3)}</div>
    </button>
  `).join("");
}

function renderStep(index) {
  state.currentStep = Math.max(0, Math.min(index, trace.steps_data.length - 1));
  const step = trace.steps_data[state.currentStep];
  const presentation = presentations[state.currentStep];
  els.slider.value = String(state.currentStep);
  els.stepIndicator.textContent = `Step ${state.currentStep + 1} / ${trace.steps}`;
  els.turnIndicator.textContent = `Turn ${presentation.turn}`;
  els.modeIndicator.textContent = trace.policy_mode;
  els.outcomeIndicator.textContent = trace.final_outcome || "ongoing";
  els.actionBanner.innerHTML = `
    <div class="banner-eyebrow">Current Action</div>
    <div class="banner-title">${escapeHtml(presentation.context.actionText)}</div>
    <div class="banner-meta">
      <span class="pill">Reward ${escapeHtml(formatFloat(step.reward))}</span>
      <span class="pill">Value ${escapeHtml(formatFloat(step.value, 3))}</span>
      <span class="pill">Decoded ${escapeHtml(step.decoded_candidate_label || "n/a")}</span>
      ${presentation.context.targetName ? `<span class="pill">Target ${escapeHtml(presentation.context.targetName)}</span>` : ""}
      ${step.outcome ? `<span class="pill">Outcome ${escapeHtml(step.outcome)}</span>` : ""}
    </div>
  `;
  els.actionMeta.textContent = "Main battlefield shows the resolved post-action frame.";
  els.contextPanel.innerHTML = renderContext(presentation);
  els.playerPanel.innerHTML = renderPlayer(presentation.player);
  els.monsterStage.innerHTML = `
    <div class="panel" style="padding:18px;">
      <h2 class="section-title">Enemy Field</h2>
      <div class="monster-grid">${presentation.monsters.map(renderMonster).join("") || "<div class='empty-state'>No monsters.</div>"}</div>
    </div>
  `;
  els.summaryPanel.innerHTML = renderSummary(presentation);
  els.logPanel.innerHTML = renderLogPanel(presentation.logs || []);
  els.handSection.innerHTML = `
    <div class="panel hand-panel">
      <h2 class="section-title">Hand After Action</h2>
      ${renderHandCards(presentation.hand.cards)}
    </div>
  `;
  if (presentation.choice) {
    els.choiceSection.innerHTML = renderChoicePanel(presentation.choice, presentation);
    els.choiceSection.style.display = "";
  } else {
    els.choiceSection.innerHTML = "";
    els.choiceSection.style.display = "none";
  }
  renderTimeline();
  renderAdvanced(step, presentation);
}

function stopPlayback() {
  if (state.timer) {
    clearInterval(state.timer);
    state.timer = null;
  }
  state.playing = false;
  els.playBtn.textContent = "Play";
  els.playBtn.classList.remove("is-active");
}

function startPlayback() {
  stopPlayback();
  state.playing = true;
  els.playBtn.textContent = "Pause";
  els.playBtn.classList.add("is-active");
  state.timer = setInterval(() => {
    if (state.currentStep >= trace.steps_data.length - 1) {
      stopPlayback();
      return;
    }
    renderStep(state.currentStep + 1);
  }, 1200);
}

els.prevBtn.addEventListener("click", () => {
  stopPlayback();
  renderStep(state.currentStep - 1);
});
els.nextBtn.addEventListener("click", () => {
  stopPlayback();
  renderStep(state.currentStep + 1);
});
els.playBtn.addEventListener("click", () => {
  if (state.playing) {
    stopPlayback();
  } else {
    startPlayback();
  }
});
els.slider.addEventListener("input", (event) => {
  stopPlayback();
  renderStep(Number(event.target.value || 0));
});
els.timelineList.addEventListener("click", (event) => {
  const item = event.target.closest("[data-step-index]");
  if (!item) return;
  stopPlayback();
  renderStep(Number(item.dataset.stepIndex || 0));
});
els.tabButtons.forEach((button) => {
  button.addEventListener("click", () => {
    state.activeTab = button.dataset.tab || "policy";
    renderAdvanced(trace.steps_data[state.currentStep], presentations[state.currentStep]);
  });
});

const presentations = trace.steps_data.map((step, index) => buildPresentation(step, index));
els.slider.max = String(Math.max(0, trace.steps_data.length - 1));
if (trace.steps_data.length) {
  renderStep(0);
} else {
  els.actionBanner.innerHTML = "<div class='banner-eyebrow'>Current Action</div><div class='banner-title'>No steps recorded</div>";
  els.actionMeta.textContent = "The trace contains no recorded steps.";
  els.contextPanel.innerHTML = "<div class='cue-card'><div class='empty-state'>No replay data available.</div></div>";
  els.playerPanel.innerHTML = "<div class='player-card'><div class='empty-state'>No replay data available.</div></div>";
  els.monsterStage.innerHTML = "<div class='panel' style='padding:18px;'><div class='empty-state'>No replay data available.</div></div>";
  els.summaryPanel.innerHTML = "<div class='summary-card'><div class='empty-state'>No replay data available.</div></div>";
  els.logPanel.innerHTML = "<div class='log-card'><div class='empty-state'>No replay data available.</div></div>";
  els.handSection.innerHTML = "<div class='panel hand-panel'><div class='empty-state'>No replay data available.</div></div>";
  els.choiceSection.style.display = "none";
}
"""
    js = (
        js.replace("__ACTION_END_TURN__", str(ACTION_TYPE_END_TURN))
        .replace("__ACTION_PLAY_CARD__", str(ACTION_TYPE_PLAY_CARD))
        .replace("__ACTION_USE_POTION__", str(ACTION_TYPE_USE_POTION))
        .replace("__ACTION_CHOICE__", str(ACTION_TYPE_CHOICE))
        .replace("__ACTION_PROCEED__", str(ACTION_TYPE_PROCEED))
        .replace("__ACTION_CANCEL__", str(ACTION_TYPE_CANCEL))
    )
    return "".join(
        [
            "<!doctype html>",
            "<html><head><meta charset='utf-8'>",
            f"<title>{html.escape(title)}</title>",
            f"<style>{css}</style>",
            "</head><body>",
            "<div class='page'>",
            "<header class='topbar'>",
            "<div class='topbar-head'>",
            "<div class='title-block'>",
            f"<h1>{html.escape(title)}</h1>",
            "<p class='subtitle'>Single-fight replay view. The main battlefield is always the resolved post-action frame.</p>",
            "<div class='meta-row'>",
            f"<span class='pill'>Spec {html.escape(str(trace['spec_name']))}</span>",
            f"<span class='pill'>Seed {html.escape(str(trace['seed']))}</span>",
            f"<span class='pill' id='mode-indicator'>{html.escape(str(trace['policy_mode']))}</span>",
            "<span class='pill' id='turn-indicator'></span>",
            "<span class='pill' id='step-indicator'></span>",
            "<span class='pill' id='outcome-indicator'></span>",
            "</div>",
            "</div>",
            "<div class='transport'>",
            "<div class='transport-buttons'>",
            "<button id='prev-btn'>Prev</button>",
            "<button id='play-btn'>Play</button>",
            "<button id='next-btn'>Next</button>",
            "</div>",
            "<div class='slider-row'>",
            "<span>Timeline</span>",
            "<input id='step-slider' type='range' min='0' value='0' step='1' />",
            f"<span>{trace['steps']} steps</span>",
            "</div>",
            "</div>",
            "</div>",
            "<div class='action-banner'>",
            "<div id='action-banner'></div>",
            "<div class='subtitle' id='action-meta'></div>",
            "</div>",
            "</header>",
            "<div class='viewer-layout'>",
            "<main class='main-column'>",
            "<section id='context-panel' class='panel cue-panel'></section>",
            "<section class='stage-panel panel'>",
            "<div class='stage-grid'>",
            "<div id='player-panel'></div>",
            "<div id='monster-stage'></div>",
            "</div>",
            "</section>",
            "<section class='detail-grid'>",
            "<div id='summary-panel'></div>",
            "<div id='log-panel'></div>",
            "</section>",
            "<section id='hand-section'></section>",
            "<section id='choice-section'></section>",
            "<details class='panel advanced-panel'>",
            "<summary class='section-title'>Advanced</summary>",
            "<div class='advanced-tabs'>",
            "<button class='tab-button is-active' data-tab='policy'>Policy</button>",
            "<button class='tab-button' data-tab='candidates'>Candidates</button>",
            "<button class='tab-button' data-tab='reward'>Reward</button>",
            "<button class='tab-button' data-tab='raw'>Raw</button>",
            "</div>",
            "<div id='advanced-content' class='advanced-content'></div>",
            "</details>",
            "</main>",
            "<aside class='panel timeline-panel'>",
            "<div class='timeline-card'>",
            "<h2 class='section-title'>Step History</h2>",
            "<p class='subtitle'>Click any step to jump. Playback stays within this fight.</p>",
            "<div id='timeline-list' class='timeline-list'></div>",
            "</div>",
            "</aside>",
            "</div>",
            "</div>",
            f"<script id='trace-data' type='application/json'>{trace_json}</script>",
            f"<script>{js}</script>",
            "</body></html>",
        ]
    )


def main() -> None:
    parser = argparse.ArgumentParser(description="Render a deterministic structured policy rollout trace.")
    parser.add_argument("--spec", type=Path, required=True, help="Combat spec json path")
    parser.add_argument("--model", type=Path, default=None, help="Structured policy checkpoint (.pt)")
    parser.add_argument("--driver-binary", type=Path, default=None)
    parser.add_argument("--seed", type=int, default=7)
    parser.add_argument("--max-steps", type=int, default=24)
    parser.add_argument("--top-k", type=int, default=8)
    parser.add_argument("--policy-mode", choices=["structured", "random"], default="structured")
    parser.add_argument("--trace-out", type=Path, default=None)
    parser.add_argument("--html-out", type=Path, default=None)
    args = parser.parse_args()

    if args.policy_mode == "structured" and args.model is None:
        raise SystemExit("--model is required when --policy-mode=structured")

    device = torch.device("cuda" if torch.cuda.is_available() else "cpu")
    model = load_model(args.model, device) if args.model is not None else None

    dataset_dir = REPO_ROOT / "tools" / "artifacts" / "learning_dataset"
    trace_out = args.trace_out or (dataset_dir / f"{args.spec.stem}_structured_policy_trace.json")
    html_out = args.html_out or (dataset_dir / f"{args.spec.stem}_structured_policy_trace.html")

    env = StructuredGymCombatEnv([args.spec], driver_binary=args.driver_binary, max_episode_steps=args.max_steps, seed=args.seed)
    try:
        obs, info = env.reset(options={"spec_path": args.spec, "seed_hint": args.seed})
        steps_data: list[dict[str, Any]] = []
        done = False
        truncated = False
        step_index = 0
        while not done and not truncated and step_index < args.max_steps:
            raw_before = info.get("raw_observation") or {}
            if args.policy_mode == "random":
                action = env.sample_random_legal_action()
                head_summary = {"mode": "random"}
            else:
                assert model is not None
                action, head_summary = analyze_policy_step(
                    model,
                    env,
                    obs,
                    raw_before,
                    deterministic=True,
                    device=device,
                    top_k=args.top_k,
                )

            decoded_index, _, _ = env.decode_canonical_action(action)
            payload = (env._last_response or {}).get("payload") or {}
            decoded_candidate = None
            for candidate, legal in zip(payload.get("action_candidates") or [], payload.get("action_mask") or []):
                if legal and int(candidate.get("index")) == decoded_index:
                    decoded_candidate = candidate
                    break

            obs, reward, done, truncated, info = env.step(action)
            raw_after = info.get("raw_observation") or {}
            step_row = {
                "canonical_action": action,
                "decoded_candidate_label": decoded_candidate.get("label") if decoded_candidate else None,
                "chosen_action_label": str(info.get("chosen_action_label") or ""),
                "reward": float(reward),
                "reward_breakdown": (env._last_response or {}).get("reward_breakdown") or {},
                "value": head_summary.get("value"),
                "head_summary": head_summary,
                "before": _raw_observation_summary(raw_before),
                "after": _raw_observation_summary(raw_after),
                "outcome": info.get("outcome"),
            }
            steps_data.append(step_row)
            step_index += 1

        trace = {
            "spec_name": args.spec.stem,
            "spec_path": str(args.spec),
            "policy_mode": args.policy_mode,
            "model_path": str(args.model) if args.model else None,
            "seed": int(args.seed),
            "steps": len(steps_data),
            "final_outcome": info.get("outcome"),
            "steps_data": steps_data,
        }
        write_json(trace_out, trace)
        html_out.parent.mkdir(parents=True, exist_ok=True)
        html_out.write_text(render_html_report(trace), encoding="utf-8")
        print(json.dumps({"trace_out": str(trace_out), "html_out": str(html_out), "steps": len(steps_data), "outcome": info.get("outcome")}, ensure_ascii=False, indent=2))
    finally:
        env.close()


if __name__ == "__main__":
    main()
