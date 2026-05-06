#!/usr/bin/env python3
"""Export small synthetic combat author specs for draw/search payoff probes.

These specs are controlled fixtures for `sts_dev_tool combat plan-probe-author-spec`.
They are intentionally tiny and diagnostic: each one isolates a draw/search card
and a nearby payoff axis (damage, block, setup, or search-to-skill).
"""
from __future__ import annotations

import argparse
import json
from datetime import datetime, timezone
from pathlib import Path
from typing import Any

from combat_reranker_common import write_json

REPO_ROOT = Path(__file__).resolve().parents[2]
REPORT_VERSION = "draw_payoff_author_specs_v0"


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Export synthetic draw/search payoff author specs.")
    parser.add_argument(
        "--out-dir",
        type=Path,
        default=REPO_ROOT / "tools" / "artifacts" / "draw_payoff_author_specs" / "v0",
    )
    return parser.parse_args()


def resolve(path: Path) -> Path:
    return path if path.is_absolute() else REPO_ROOT / path


def monster(
    *,
    hp: int = 54,
    intent: str = "attack",
    damage: int = 12,
    hits: int = 1,
    monster_id: str = "Cultist",
) -> dict[str, Any]:
    return {
        "id": monster_id,
        "current_hp": hp,
        "max_hp": hp,
        "intent": intent,
        "move_base_damage": damage,
        "move_adjusted_damage": damage,
        "move_hits": hits,
    }


def spec(
    *,
    name: str,
    description: str,
    hand: list[Any],
    draw_pile: list[Any],
    monsters: list[dict[str, Any]] | None = None,
    hp: int = 70,
    energy: int = 3,
    tags: list[str] | None = None,
) -> dict[str, Any]:
    return {
        "name": name,
        "player_class": "IRONCLAD",
        "room_type": "MonsterRoom",
        "turn": 1,
        "player": {
            "current_hp": hp,
            "max_hp": 80,
            "block": 0,
            "energy": energy,
            "gold": 99,
        },
        "monsters": monsters or [monster()],
        "hand": hand,
        "draw_pile": draw_pile,
        "discard_pile": [],
        "exhaust_pile": [],
        "relics": ["Burning Blood"],
        "potions": [],
        "tags": ["draw_payoff_probe", *(tags or [])],
        "provenance": {
            "source": "synthetic_draw_payoff_author_specs_v0",
            "notes": [description],
        },
    }


def specs() -> list[dict[str, Any]]:
    return [
        spec(
            name="battle_trance_draws_inflame_damage_payoff",
            description=(
                "Battle Trance should expose Inflame, then attacks can cash out "
                "setup into same-turn damage."
            ),
            hand=["Battle Trance", "Wild Strike", "Strike_R", "Defend_R"],
            draw_pile=["Inflame", "Strike_R", "Strike_R", "Defend_R"],
            monsters=[monster(hp=64, damage=6)],
            tags=["battle_trance", "setup_payoff", "damage_payoff"],
        ),
        spec(
            name="pommel_strike_draws_block_payoff_under_pressure",
            description=(
                "Pommel Strike is not just damage; drawing Defend should matter "
                "when there is visible incoming damage and spare energy."
            ),
            hand=["Pommel Strike", "Strike_R", "Defend_R"],
            draw_pile=["Defend_R", "Strike_R", "Strike_R"],
            monsters=[monster(hp=48, damage=18)],
            hp=45,
            tags=["pommel_strike", "block_payoff", "pressure"],
        ),
        spec(
            name="offering_resource_window_damage_and_block_payoff",
            description=(
                "Offering pays HP to open a same-turn resource window; it should "
                "only be attractive if drawn cards can become damage or block."
            ),
            hand=["Offering", "Strike_R", "Defend_R"],
            draw_pile=["Inflame", "Strike_R", "Defend_R", "Strike_R"],
            monsters=[monster(hp=72, damage=18)],
            hp=55,
            tags=["offering", "resource_window", "damage_payoff", "block_payoff"],
        ),
        spec(
            name="secret_technique_searches_shrug_block_draw_payoff",
            description=(
                "Secret Technique should be able to search a skill payoff such as "
                "Shrug It Off when block and further draw are valuable."
            ),
            hand=["Secret Technique", "Strike_R", "Defend_R"],
            draw_pile=["Shrug It Off", "Inflame", "Strike_R", "Strike_R"],
            monsters=[monster(hp=56, damage=18)],
            hp=48,
            tags=["secret_technique", "search_payoff", "block_payoff", "draw_payoff"],
        ),
    ]


def markdown_manifest(manifest: dict[str, Any]) -> str:
    lines = [
        "# Draw Payoff Author Specs",
        "",
        "Synthetic author specs for current-turn draw/search payoff probes.",
        "",
        "| spec | tags | intent | path |",
        "| --- | --- | --- | --- |",
    ]
    for row in manifest["specs"]:
        lines.append(
            f"| `{row['name']}` | `{','.join(row['tags'])}` | {row['description']} | `{row['path']}` |"
        )
    return "\n".join(lines) + "\n"


def main() -> None:
    args = parse_args()
    out_dir = resolve(args.out_dir)
    spec_dir = out_dir / "specs"
    spec_dir.mkdir(parents=True, exist_ok=True)
    rows = []
    for payload in specs():
        path = spec_dir / f"{payload['name']}.json"
        write_json(path, payload)
        rows.append(
            {
                "name": payload["name"],
                "path": str(path),
                "tags": payload.get("tags") or [],
                "description": (payload.get("provenance") or {}).get("notes", [""])[0],
            }
        )
    manifest = {
        "report_version": REPORT_VERSION,
        "generated_at_utc": datetime.now(timezone.utc).isoformat(),
        "spec_count": len(rows),
        "specs": rows,
        "next_command": (
            "cargo run --bin sts_dev_tool -- combat plan-probe-author-spec "
            "--author-spec <spec.json> --out <report.json>"
        ),
    }
    write_json(out_dir / "manifest.json", manifest)
    (out_dir / "manifest.md").write_text(markdown_manifest(manifest), encoding="utf-8")
    print(f"Wrote {out_dir / 'manifest.json'}")
    print(f"Wrote {out_dir / 'manifest.md'}")
    print(json.dumps({"spec_count": len(rows), "out_dir": str(out_dir)}, indent=2))


if __name__ == "__main__":
    main()
