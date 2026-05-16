#!/usr/bin/env python3
"""Audit direct upgrade-sensitive CombatCard field reads in Silent cards.

This intentionally does not edit gameplay code. It scans
src/content/cards/silent for direct reads of:

  card.base_magic_num_mut
  card.base_damage_mut
  card.base_block_mut
  card.upgrades

and writes a markdown report classifying each file into one of three review
buckets:

  * 应改为 evaluate_card_for_play
  * 合理特殊升级
  * 疑似测试掩盖风险
"""

from __future__ import annotations

import argparse
import datetime as _dt
import re
import sys
from dataclasses import dataclass
from pathlib import Path


FIELD_RE = re.compile(r"\bcard\.(base_magic_num_mut|base_damage_mut|base_block_mut|upgrades)\b")

CATEGORY_LABELS = {
    "should_evaluate": "应改为 evaluate_card_for_play",
    "reasonable_special": "合理特殊升级",
    "test_masking_risk": "疑似测试掩盖风险",
}

CATEGORY_ORDER = ("should_evaluate", "reasonable_special", "test_masking_risk")


@dataclass(frozen=True)
class Classification:
    category: str
    reason: str
    recommendation: str


CLASSIFICATIONS: dict[str, Classification] = {
    "acrobatics.rs": Classification(
        "test_masking_risk",
        "读取 card.base_magic_num_mut 决定抽牌数；当前测试里存在手动预填 acrobatics.base_magic_num_mut 的模式，可能绕过正常出牌前评价。",
        "改为在 play 内调用 evaluate_card_for_play 或显式从 CardDefinition+upgrades 计算抽牌数；测试应避免手动预填 mutable 渲染字段。",
    ),
    "adrenaline.rs": Classification(
        "should_evaluate",
        "GainEnergy 的 card.upgrades 分支符合 Java this.upgraded，但 DrawCards 读取 card.base_magic_num_mut；Java 固定 DrawCardAction(p, 2)，不依赖 mutable 渲染字段。",
        "保留 energy 的 upgraded 分支；DrawCards 改为 Java 字面量 2 或由统一评价/定义 helper 取得，避免默认 0。",
    ),
    "backflip.rs": Classification(
        "should_evaluate",
        "读取 card.base_block_mut 和 card.base_magic_num_mut；这是普通 use 路径，升级后的 block/draw 应在本函数入口通过 card evaluation 得到。",
        "改为调用 evaluate_card_for_play(card, state, None)，再读取 evaluated.base_block_mut/base_magic_num_mut。",
    ),
    "blade_dance.rs": Classification(
        "should_evaluate",
        "读取 card.base_magic_num_mut 决定生成 Shiv 数；普通 use 路径不应依赖调用者预先刷新 mutable 字段。",
        "改为调用 evaluate_card_for_play(card, state, None) 或统一 magic helper。",
    ),
    "bouncing_flask.rs": Classification(
        "should_evaluate",
        "读取 card.base_magic_num_mut 决定随机弹跳次数；普通 use 路径不应依赖外部预评价。",
        "改为调用 evaluate_card_for_play(card, state, None) 后使用 evaluated.base_magic_num_mut。",
    ),
    "burst.rs": Classification(
        "should_evaluate",
        "读取 card.base_magic_num_mut 作为 BurstPower amount；升级影响 amount，普通 use 路径应自行评价。",
        "改为调用 evaluate_card_for_play(card, state, None)。",
    ),
    "catalyst.rs": Classification(
        "should_evaluate",
        "读取 card.base_magic_num_mut 计算毒倍数；升级影响 multiplier，普通 use 路径应自行评价。",
        "改为调用 evaluate_card_for_play(card, state, Some(target)) 或定义专用 magic helper 后再计算 extra poison。",
    ),
    "cloak_and_dagger.rs": Classification(
        "should_evaluate",
        "读取 card.base_block_mut 和 card.base_magic_num_mut；升级影响格挡和 Shiv 数，普通 use 路径应自行评价。",
        "改为调用 evaluate_card_for_play(card, state, None)，并用 evaluated block/magic。",
    ),
    "corpse_explosion.rs": Classification(
        "test_masking_risk",
        "读取 card.base_magic_num_mut 作为 Poison amount；测试中存在 corpse_plus.base_magic_num_mut 手动预填，可能掩盖 play 路径未评价。",
        "改为调用 evaluate_card_for_play(card, state, Some(target))；测试用 upgrades 表达升级，不要直接写 mutable magic。",
    ),
    "dagger_throw.rs": Classification(
        "should_evaluate",
        "读取 card.base_damage_mut 作为输出伤害，并用 def.base_damage 判断 modified；普通 attack use 应在本函数内 calculate damage。",
        "改为调用 evaluate_card_for_play(card, state, Some(target))，并用 evaluated damage。",
    ),
    "deadly_poison.rs": Classification(
        "should_evaluate",
        "读取 card.base_magic_num_mut 作为 Poison amount；升级影响 poison 数值，普通 use 路径应自行评价。",
        "改为调用 evaluate_card_for_play(card, state, Some(target)) 或统一 magic helper。",
    ),
    "doppelganger.rs": Classification(
        "reasonable_special",
        "Java DoppelgangerAction 直接接收 this.upgraded、freeToPlayOnce、energyOnUse；这里读取 card.upgrades 是动作语义参数，不是数值评价。",
        "保留直接传 upgraded，但建议测试覆盖 X 费、free_to_play_once、energy_on_use 与升级组合。",
    ),
    "footwork.rs": Classification(
        "should_evaluate",
        "读取 card.base_magic_num_mut 作为 Dexterity amount；普通 power use 应自行评价升级后的 magic。",
        "改为调用 evaluate_card_for_play(card, state, None)。",
    ),
    "malaise.rs": Classification(
        "reasonable_special",
        "Java MalaiseAction 直接接收 this.upgraded、freeToPlayOnce、energyOnUse；这里读取 card.upgrades 是 X 费动作语义参数。",
        "保留直接传 upgraded，但建议测试覆盖升级和 energy_on_use 的组合。",
    ),
    "nightmare.rs": Classification(
        "test_masking_risk",
        "读取 card.base_magic_num_mut 作为复制数量；测试中存在 nightmare_plus.base_magic_num_mut 手动预填，可能掩盖正常 play 路径未评价。Java Nightmare+ 主要是费用变化，不应靠 mutable magic 预填。",
        "改为使用 evaluate_card_for_play 或定义/literal helper；测试应断言升级费用而不是手写 mutable magic。",
    ),
    "noxious_fumes.rs": Classification(
        "should_evaluate",
        "读取 card.base_magic_num_mut 作为 power amount；升级影响 amount，普通 power use 应自行评价。",
        "改为调用 evaluate_card_for_play(card, state, None)。",
    ),
    "poisoned_stab.rs": Classification(
        "should_evaluate",
        "同时读取 card.base_damage_mut 和 card.base_magic_num_mut；伤害和 poison 都受升级/力量等评价影响，普通 attack use 应自行评价。",
        "改为调用 evaluate_card_for_play(card, state, Some(target))，统一使用 evaluated damage/magic。",
    ),
    "prepared.rs": Classification(
        "test_masking_risk",
        "读取 card.base_magic_num_mut 决定抽牌和弃牌数；测试中存在 prepared_plus.base_magic_num_mut 手动预填，可能掩盖正常 play 路径未评价。",
        "改为调用 evaluate_card_for_play(card, state, None) 或统一 magic helper；测试通过 upgrades 表达 Prepared+。",
    ),
    "reflex.rs": Classification(
        "reasonable_special",
        "triggerOnManualDiscard 不是普通 play 路径；这里按 CardDefinition + card.upgrades 计算 magic，避免依赖未评价的 mutable magic。",
        "保留这种模式，后续可抽成 manual-discard magic helper，并继续保留与弃牌顺序相关测试。",
    ),
    "storm_of_steel.rs": Classification(
        "reasonable_special",
        "Java BladeFuryAction 直接接收 this.upgraded；读取 card.upgrades 是生成 Shiv 是否升级的语义参数。",
        "保留直接传 upgraded，确保 BladeFuryAction/生成 Shiv 的升级行为有测试覆盖。",
    ),
    "survivor.rs": Classification(
        "should_evaluate",
        "读取 card.base_block_mut；Survivor+ 升级格挡，普通 use 路径应自行评价。",
        "改为调用 evaluate_card_for_play(card, state, None)。",
    ),
    "tactician.rs": Classification(
        "reasonable_special",
        "triggerOnManualDiscard 不是普通 play 路径；这里按 CardDefinition + card.upgrades 计算 energy amount，匹配 Java addToTop(new GainEnergyAction(this.magicNumber))。",
        "保留这种模式，后续可抽成 manual-discard magic helper，并继续保留 addToTop 顺序测试。",
    ),
    "well_laid_plans.rs": Classification(
        "test_masking_risk",
        "读取 card.base_magic_num_mut 作为 RetainCards amount；测试中存在 plans_plus.base_magic_num_mut 手动预填，可能掩盖正常 play 路径未评价。",
        "改为调用 evaluate_card_for_play(card, state, None)；测试通过 upgrades 表达 Well-Laid Plans+。",
    ),
}


@dataclass
class Match:
    file: str
    line_no: int
    field: str
    text: str


def rel(path: Path, root: Path) -> str:
    return path.relative_to(root).as_posix()


def scan(root: Path, silent_dir: Path) -> dict[str, list[Match]]:
    results: dict[str, list[Match]] = {}
    for path in sorted(silent_dir.glob("*.rs")):
        if path.name == "mod.rs":
            continue
        matches: list[Match] = []
        text = path.read_text(encoding="utf-8")
        for line_no, line in enumerate(text.splitlines(), start=1):
            for m in FIELD_RE.finditer(line):
                matches.append(
                    Match(
                        file=rel(path, root),
                        line_no=line_no,
                        field=m.group(1),
                        text=line.strip(),
                    )
                )
        if matches:
            results[path.name] = matches
    return results


def markdown_table_row(cells: list[str]) -> str:
    escaped = [cell.replace("\n", "<br>").replace("|", "\\|") for cell in cells]
    return "| " + " | ".join(escaped) + " |"


def render_report(root: Path, matches_by_file: dict[str, list[Match]]) -> str:
    now = _dt.datetime.now().strftime("%Y-%m-%d %H:%M:%S")
    total_files = len(matches_by_file)
    total_matches = sum(len(v) for v in matches_by_file.values())
    unclassified = sorted(set(matches_by_file) - set(CLASSIFICATIONS))

    grouped: dict[str, list[str]] = {key: [] for key in CATEGORY_ORDER}
    for file_name in sorted(matches_by_file):
        info = CLASSIFICATIONS.get(file_name)
        if info is None:
            continue
        grouped[info.category].append(file_name)

    lines: list[str] = [
        "# Silent Card Upgrade Read Audit",
        "",
        f"Generated: `{now}`",
        "",
        "Command:",
        "",
        "```powershell",
        "python tools/audit_silent_card_upgrade_reads.py",
        "```",
        "",
        "Scope: `src/content/cards/silent/*.rs` excluding `mod.rs`.",
        "",
        "Scanned direct reads:",
        "",
        "- `card.base_magic_num_mut`",
        "- `card.base_damage_mut`",
        "- `card.base_block_mut`",
        "- `card.upgrades`",
        "",
        "This report is an audit artifact only. It does not modify gameplay code.",
        "",
        "## Summary",
        "",
        f"- Files with direct reads: `{total_files}`",
        f"- Direct read occurrences: `{total_matches}`",
        f"- Unclassified files: `{len(unclassified)}`",
        "",
    ]

    if unclassified:
        lines.extend(
            [
                "## Unclassified Matches",
                "",
                "These must be classified before treating the audit as complete.",
                "",
            ]
        )
        for file_name in unclassified:
            lines.append(f"- `{file_name}`")
        lines.append("")

    lines.extend(
        [
            "## Category Definitions",
            "",
            "- **应改为 evaluate_card_for_play**: ordinary `use` logic reads mutable rendered fields directly. The card should evaluate itself at play time or use a shared definition/upgrades helper.",
            "- **合理特殊升级**: direct `card.upgrades` is part of Java action semantics or happens outside normal card play, so `evaluate_card_for_play` is not the right abstraction.",
            "- **疑似测试掩盖风险**: current tests appear to prefill mutable render fields, which can hide the fact that normal play does not evaluate the card.",
            "",
        ]
    )

    for category in CATEGORY_ORDER:
        label = CATEGORY_LABELS[category]
        lines.extend([f"## {label}", ""])
        files = grouped[category]
        if not files:
            lines.extend(["No files.", ""])
            continue
        lines.extend(
            [
                markdown_table_row(
                    ["File", "Fields", "Lines", "Reason", "Recommendation"]
                ),
                markdown_table_row(["---", "---", "---", "---", "---"]),
            ]
        )
        for file_name in files:
            info = CLASSIFICATIONS[file_name]
            matches = matches_by_file[file_name]
            fields = ", ".join(sorted({m.field for m in matches}))
            line_refs = ", ".join(f"`{m.file}:{m.line_no}`" for m in matches)
            lines.append(
                markdown_table_row(
                    [
                        f"`src/content/cards/silent/{file_name}`",
                        f"`{fields}`",
                        line_refs,
                        info.reason,
                        info.recommendation,
                    ]
                )
            )
        lines.append("")

    lines.extend(["## Raw Matches", ""])
    lines.extend(
        [
            markdown_table_row(["File", "Line", "Field", "Code"]),
            markdown_table_row(["---", "---", "---", "---"]),
        ]
    )
    for file_name in sorted(matches_by_file):
        for m in matches_by_file[file_name]:
            lines.append(
                markdown_table_row([f"`{m.file}`", str(m.line_no), f"`{m.field}`", f"`{m.text}`"])
            )
    lines.append("")

    lines.extend(["## Immediate Recommendations", ""])
    if grouped["test_masking_risk"]:
        lines.append(
            "1. Fix the `疑似测试掩盖风险` group first because tests may be asserting manually prepared transient fields rather than real play behavior."
        )
        lines.append(
            "2. Then fix the `应改为 evaluate_card_for_play` group by making ordinary play functions evaluate locally before reading damage/block/magic."
        )
    else:
        lines.append(
            "1. The `疑似测试掩盖风险` group is empty; continue with the `应改为 evaluate_card_for_play` group."
        )
        lines.append(
            "2. Make ordinary play functions evaluate locally before reading damage/block/magic."
        )
    lines.extend(
        [
            "3. Keep the `合理特殊升级` group as direct upgrade reads, but add comments/tests where the Java source passes `this.upgraded` directly to an action.",
            "4. Keep this script failing on unclassified files so future direct reads cannot silently enter Silent card code.",
            "",
        ]
    )

    return "\n".join(lines)


def main() -> int:
    repo_root = Path(__file__).resolve().parents[1]
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--silent-dir",
        type=Path,
        default=repo_root / "src" / "content" / "cards" / "silent",
    )
    parser.add_argument(
        "--output",
        type=Path,
        default=repo_root / "docs" / "audits" / "SILENT_CARD_UPGRADE_READ_AUDIT.md",
    )
    args = parser.parse_args()

    matches_by_file = scan(repo_root, args.silent_dir)
    report = render_report(repo_root, matches_by_file)
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(report, encoding="utf-8", newline="\n")

    unclassified = sorted(set(matches_by_file) - set(CLASSIFICATIONS))
    print(f"wrote {args.output}")
    print(
        f"files={len(matches_by_file)} matches={sum(len(v) for v in matches_by_file.values())} unclassified={len(unclassified)}"
    )
    if unclassified:
        for file_name in unclassified:
            print(f"unclassified: {file_name}", file=sys.stderr)
        return 2
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
