#!/usr/bin/env python3
from __future__ import annotations

from dataclasses import dataclass
from pathlib import Path

from combat_rl_common import REPO_ROOT


@dataclass(frozen=True)
class BenchmarkCase:
    spec_name: str
    seed: int
    tag: str


FIXED_BENCHMARK_CASES = [
    BenchmarkCase("survival_override_guardrail", 11, "must_block"),
    BenchmarkCase("survival_override_plays_defend_not_slimed", 13, "survival"),
    BenchmarkCase("jaw_worm_opening", 15, "attack_over_defend"),
    BenchmarkCase("spot_weakness_attack_intent_window", 17, "target_selection"),
    BenchmarkCase("flex_before_strike_cultist_light_pressure_turn2", 18, "setup_before_payoff"),
    BenchmarkCase("power_through_not_on_lagavulin_sleep_turn", 19, "lagavulin_sleep"),
    BenchmarkCase("guardian_corruption_too_early_floor16", 23, "guardian_threshold"),
    BenchmarkCase("fire_breathing_over_dark_embrace_vs_slime_boss_attack_turn", 29, "slime_boss_split"),
    BenchmarkCase("power_through_second_wind_net_value_mixed", 30, "status_exhaust_draw"),
    BenchmarkCase("jaw_worm_attack_potion_overuse_floor7", 31, "potion"),
    BenchmarkCase("colorless_potion_not_on_light_pressure_cultist_turn2", 37, "discovery"),
]


def benchmark_spec_dir() -> Path:
    return REPO_ROOT / "data" / "combat_lab" / "specs"


def resolve_benchmark_cases(spec_dir: Path | None = None) -> list[tuple[BenchmarkCase, Path]]:
    root = spec_dir or benchmark_spec_dir()
    resolved: list[tuple[BenchmarkCase, Path]] = []
    for case in FIXED_BENCHMARK_CASES:
        path = root / f"{case.spec_name}.json"
        if path.exists():
            resolved.append((case, path))
    return resolved
