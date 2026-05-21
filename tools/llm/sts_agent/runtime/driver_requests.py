"""Typed request helpers for Rust driver evidence commands."""

from __future__ import annotations

import argparse
from typing import Any

from sts_agent.runtime.env_driver import DriverClient


def request_combat_search_engine(client: DriverClient, args: argparse.Namespace) -> dict[str, Any]:
    response = client.request(
        {
            "cmd": "combat_search_engine",
            "horizon_turns": args.combat_search_horizon_turns,
            "max_nodes": args.combat_search_max_nodes,
            "beam_width": args.combat_search_beam_width,
            "particles": args.combat_search_particles,
            "max_engine_steps_per_action": args.search_max_engine_steps_per_action,
            "include_branch_clusters": True,
        }
    )
    return response.get("payload") or {}

def request_combat_plan_probe(client: DriverClient, args: argparse.Namespace) -> dict[str, Any]:
    return client.request(
        {
            "cmd": "combat_plan_probe",
            "max_depth": args.search_max_depth,
            "max_nodes": args.search_max_nodes,
            "beam_width": args.search_beam_width,
            "max_engine_steps_per_action": args.search_max_engine_steps_per_action,
        }
    )["payload"]

def request_candidate_afterstate_summary(
    client: DriverClient,
    action_ids: list[int],
) -> dict[str, Any]:
    return client.request(
        {
            "cmd": "candidate_afterstate_summary",
            "action_ids": action_ids,
        }
    )["payload"]

def request_decision_lab_probe(
    client: DriverClient,
    args: argparse.Namespace,
    action_ids: list[int],
) -> dict[str, Any]:
    return client.request(
        {
            "cmd": "decision_lab_probe",
            "action_ids": action_ids,
            "max_rollout_steps": args.lab_max_rollout_steps,
            "max_depth": args.search_max_depth,
            "max_nodes": args.search_max_nodes,
            "beam_width": args.search_beam_width,
            "max_engine_steps_per_action": args.search_max_engine_steps_per_action,
        }
    )["payload"]

def request_combat_multi_turn_lab(
    client: DriverClient,
    args: argparse.Namespace,
    action_ids: list[int],
) -> dict[str, Any]:
    return client.request(
        {
            "cmd": "decision_lab_probe",
            "action_ids": action_ids,
            "max_rollout_steps": args.combat_lab_max_rollout_steps,
            "max_depth": args.search_max_depth,
            "max_nodes": args.search_max_nodes,
            "beam_width": args.search_beam_width,
            "max_engine_steps_per_action": args.search_max_engine_steps_per_action,
        }
    )["payload"]

def request_campfire_rest_smith_eval(client: DriverClient) -> dict[str, Any]:
    return client.request({"cmd": "campfire_rest_smith_eval"})["payload"]
