#!/usr/bin/env python3
"""Generate a source-grounded Defect card implementation audit.

This intentionally reads Java source as the authority and writes a Markdown
ledger before any Rust Defect card implementation starts.
"""

from __future__ import annotations

from dataclasses import dataclass
from pathlib import Path
import re
from typing import Iterable


REPO_ROOT = Path(__file__).resolve().parents[1]
JAVA_ROOT = REPO_ROOT.parent / "cardcrawl"
BLUE_ROOT = JAVA_ROOT / "cards" / "blue"
CARD_LIBRARY = JAVA_ROOT / "helpers" / "CardLibrary.java"
OUTPUT = REPO_ROOT / "docs" / "audits" / "DEFECT_CARD_SOURCE_AUDIT.md"


SUPPORTED_ACTIONS = {
    "ApplyPowerAction": "generic",
    "ChannelAction": "partial: Action::ChannelOrb exists",
    "DamageAction": "generic",
    "DamageAllEnemiesAction": "generic",
    "DrawCardAction": "generic",
    "GainBlockAction": "generic",
    "GainEnergyAction": "generic",
    "IncreaseMaxOrbAction": "generic: Action::IncreaseMaxOrb exists",
    "MakeTempCardInDiscardAction": "generic card-copy support exists",
    "MakeTempCardInHandAction": "generic card-copy support exists",
    "RemoveAllBlockAction": "generic",
    "SFXAction": "presentation no-op for Rust simulation",
    "ShuffleAction": "generic shuffle support exists",
    "ShuffleAllAction": "generic shuffle support exists",
    "VFXAction": "presentation no-op for Rust simulation",
    "WaitAction": "presentation/timing no-op unless action ordering proves otherwise",
}

DEFECT_ACTIONS_REQUIRING_WORK = {
    "AggregateEnergyAction",
    "AllCostToHandAction",
    "BarrageAction",
    "BetterDiscardPileToHandAction",
    "BetterDrawPileToHandAction",
    "CompileDriverAction",
    "DarkImpulseAction",
    "DecreaseMaxOrbAction",
    "DoubleEnergyAction",
    "EvokeWithoutRemovingOrbAction",
    "FissionAction",
    "ForTheEyesAction",
    "FTLAction",
    "GashAction",
    "ImpulseAction",
    "IncreaseMiscAction",
    "ModifyBlockAction",
    "MulticastAction",
    "NewRipAndTearAction",
    "NewThunderStrikeAction",
    "RecycleAction",
    "RedoAction",
    "ReduceCostAction",
    "ReinforcedBodyAction",
    "ScrapeFollowUpAction",
    "SunderAction",
    "TempestAction",
}

EXISTING_RUST_DEFECT_POWERS = {
    "ArtifactPower": "PowerId::Artifact exists; shared Artifact interception exists",
    "BufferPower": "PowerId::Buffer exists; runtime hooks need card-specific audit",
    "DexterityPower": "PowerId::Dexterity exists; shared block modifier exists",
    "ElectroPower": "PowerId::Electro exists; lightning all-enemy hook exists",
    "FocusPower": "PowerId::Focus exists; orb focus refresh exists",
    "EnergizedBluePower": "PowerId::Energized exists as shared next-turn energy",
    "EquilibriumPower": "PowerId::Equilibrium exists for retain behavior",
    "StrengthPower": "PowerId::Strength exists; shared damage modifier exists",
    "VulnerablePower": "PowerId::Vulnerable exists; shared debuff behavior exists",
}


@dataclass
class CardAudit:
    file_name: str
    class_name: str
    card_id: str
    registered: bool
    cost: str
    card_type: str
    rarity: str
    target: str
    base_damage: str
    base_block: str
    base_magic: str
    flags: list[str]
    tags: list[str]
    show_evoke: str
    upgrades: list[str]
    actions: list[str]
    orbs: list[str]
    powers: list[str]
    hooks: list[str]
    notes: list[str]


def split_java_args(text: str) -> list[str]:
    out: list[str] = []
    depth = 0
    current: list[str] = []
    for ch in text:
        if ch == "," and depth == 0:
            out.append("".join(current).strip())
            current = []
            continue
        current.append(ch)
        if ch == "(":
            depth += 1
        elif ch == ")":
            depth -= 1
    if current:
        out.append("".join(current).strip())
    return out


def match_body(text: str, signature: str) -> str:
    m = re.search(signature + r"\s*\{(?P<body>.*?)\n    \}", text, re.S)
    return m.group("body") if m else ""


def field_number(body: str, field: str) -> str:
    direct = re.search(rf"this\.{field}\s*=\s*(-?\d+)", body)
    if direct:
        return direct.group(1)
    chained = re.search(rf"this\.\w+\s*=\s*this\.{field}\s*=\s*(-?\d+)", body)
    if chained:
        return chained.group(1)
    return "0"


def field_notes(body: str) -> list[str]:
    notes: list[str] = []
    misc = re.search(r"this\.misc\s*=\s*(-?\d+)", body)
    if misc:
        notes.append(f"misc starts {misc.group(1)}")
    for field, label in [
        ("baseDamage", "base damage"),
        ("baseBlock", "base block"),
        ("baseMagicNumber", "base magic"),
    ]:
        if re.search(rf"this\.{field}\s*=\s*this\.misc\b", body):
            notes.append(f"{label} is initialized from misc")
    return notes


def registered_blue_classes() -> list[str]:
    text = CARD_LIBRARY.read_text(encoding="utf-8", errors="replace")
    body = match_body(text, r"private static void addBlueCards\(\)")
    return re.findall(r"CardLibrary\.add\(new\s+(\w+)\(", body)


def parse_card(path: Path, registered: set[str]) -> CardAudit:
    text = path.read_text(encoding="utf-8", errors="replace")
    class_name = re.search(r"public class\s+(\w+)", text).group(1)
    card_id = re.search(r'public static final String ID = "([^"]+)"', text).group(1)

    ctor = match_body(text, rf"public {re.escape(class_name)}\(\)")
    super_match = re.search(r"super\((.*?)\);", ctor, re.S)
    args = split_java_args(super_match.group(1).replace("\n", " ")) if super_match else []
    cost = args[3] if len(args) > 3 else "?"
    card_type = args[5].split(".")[-1] if len(args) > 5 else "?"
    rarity = args[7].split(".")[-1] if len(args) > 7 else "?"
    target = args[8].split(".")[-1] if len(args) > 8 else "?"

    flags = []
    for field in ["exhaust", "isEthereal", "isInnate", "selfRetain", "isMultiDamage"]:
        m = re.search(rf"this\.{field}\s*=\s*(true|false)", ctor)
        if m and m.group(1) == "true":
            flags.append(field)

    tags = [
        tag.split(".")[-1]
        for tag in re.findall(r"this\.tags\.add\(AbstractCard\.CardTags\.(\w+)\)", ctor)
    ]

    show_evoke_parts = []
    if "showEvokeValue = true" in ctor:
        show_evoke_parts.append("value")
    count = re.search(r"showEvokeOrbCount\s*=\s*(-?\d+)", ctor)
    if count:
        show_evoke_parts.append(f"count={count.group(1)}")

    upgrade = match_body(text, r"public void upgrade\(\)")
    upgrades = [
        f"{name}({arg.strip()})"
        for name, arg in re.findall(r"this\.(upgrade\w+)\((.*?)\);", upgrade)
    ]
    if "rawDescription" in upgrade:
        upgrades.append("rawDescription")
    if "initializeDescription" in upgrade:
        upgrades.append("initializeDescription")
    for field in ["exhaust", "isEthereal", "isInnate", "selfRetain", "isMultiDamage"]:
        for value in re.findall(rf"this\.{field}\s*=\s*(true|false)", upgrade):
            upgrades.append(f"{field}={value}")

    use = match_body(text, r"public void use\(.*?\)")
    actions = sorted(set(re.findall(r"new\s+(\w+Action)\b", use)))
    orbs = sorted(set(re.findall(r"new\s+(Dark|Frost|Lightning|Plasma)\s*\(", use)))
    powers = sorted(set(re.findall(r"new\s+(\w+Power)\b", use)))

    hooks = []
    for name in [
        "applyPowers",
        "calculateCardDamage",
        "onMoveToDiscard",
        "triggerOnOtherCardPlayed",
        "triggerWhenDrawn",
        "triggerOnManualDiscard",
    ]:
        if re.search(rf"public void {name}\(", text):
            hooks.append(name)

    notes = field_notes(ctor)
    if "Settings.isDebug" in use:
        notes.append("use has Settings.isDebug branch; normal gameplay branch is authoritative")
    if re.search(r"this\.baseBlock\s*=\s*this\.misc", text) and "base block is initialized from misc" not in notes:
        notes.append("base block is refreshed from misc outside constructor")

    return CardAudit(
        file_name=path.name,
        class_name=class_name,
        card_id=card_id,
        registered=class_name in registered,
        cost=cost,
        card_type=card_type,
        rarity=rarity,
        target=target,
        base_damage=field_number(ctor, "baseDamage"),
        base_block=field_number(ctor, "baseBlock"),
        base_magic=field_number(ctor, "baseMagicNumber"),
        flags=flags,
        tags=tags,
        show_evoke=", ".join(show_evoke_parts) if show_evoke_parts else "",
        upgrades=upgrades,
        actions=actions,
        orbs=orbs,
        powers=powers,
        hooks=hooks,
        notes=notes,
    )


def md_escape(value: str) -> str:
    return value.replace("|", r"\|")


def join(values: Iterable[str]) -> str:
    values = list(values)
    return ", ".join(values) if values else "-"


def action_status(action: str) -> str:
    if action in SUPPORTED_ACTIONS:
        return f"{action}: {SUPPORTED_ACTIONS[action]}"
    if action == "AnimateOrbAction":
        return f"{action}: presentation no-op; real behavior is later evoke action"
    if action == "EvokeOrbAction":
        return f"{action}: partial: Action::EvokeOrb exists; Java class semantics need parity tests"
    if action in DEFECT_ACTIONS_REQUIRING_WORK:
        return f"{action}: needs Defect action implementation/audit"
    return f"{action}: unknown, audit before use"


def generate() -> str:
    registered = registered_blue_classes()
    registered_set = set(registered)
    cards = [parse_card(path, registered_set) for path in sorted(BLUE_ROOT.glob("*.java"))]
    by_class = {card.class_name: card for card in cards}
    registered_cards = [by_class[name] for name in registered if name in by_class]
    source_only = [c for c in cards if not c.registered]

    all_actions = sorted({action for card in registered_cards for action in card.actions})
    all_powers = sorted({power for card in registered_cards for power in card.powers})
    all_orbs = sorted({orb for card in registered_cards for orb in card.orbs})

    lines: list[str] = []
    lines.append("# Defect Card Source Audit")
    lines.append("")
    lines.append("Status: source-grounding ledger before Rust Defect card implementation.")
    lines.append("")
    lines.append("Java sources:")
    lines.append("")
    lines.append("- `D:/rust/cardcrawl/cards/blue/*.java`")
    lines.append("- `D:/rust/cardcrawl/helpers/CardLibrary.java::addBlueCards()`")
    lines.append("")
    lines.append("Rust state at audit time:")
    lines.append("")
    lines.append("- no `src/content/cards/defect/` directory exists yet;")
    lines.append("- `CardId` has no Defect card variants yet;")
    lines.append("- `defect_pool_for_rarity` is still an empty stub;")
    lines.append("- core orb primitives exist: `OrbId`, `OrbEntity`, `Action::ChannelOrb`, `Action::EvokeOrb`, `Action::IncreaseMaxOrb`, start/end orb hooks, and Focus-aware orb values.")
    lines.append("")
    lines.append("## Counts")
    lines.append("")
    lines.append(f"- Java files in `cards/blue`: `{len(cards)}`")
    lines.append(f"- Cards registered by `CardLibrary.addBlueCards()`: `{len(registered_cards)}`")
    lines.append(f"- Source-present but not registered: `{len(source_only)}`")
    if source_only:
        lines.append(f"- Not registered: `{', '.join(c.class_name for c in source_only)}`")
    lines.append("")
    lines.append("Do not add source-present/unregistered cards to reward pools unless Java `CardLibrary` does.")
    lines.append("The registered card table follows Java `CardLibrary.addBlueCards()` order, not filesystem order.")
    lines.append("")
    lines.append("## Immediate Implementation Order")
    lines.append("")
    lines.append("1. Create `src/content/cards/defect/` and add only registered Blue cards.")
    lines.append("2. Add `CardId` variants and `get_card_definition` dispatch in small batches.")
    lines.append("3. Start with starter/basic cards: `Strike_Blue`, `Defend_Blue`, `Zap`, `Dualcast`.")
    lines.append("4. Before broad card implementation, harden orb action support for multiple evoke, evoke-without-removing, random orb channeling, and max-orb decrease.")
    lines.append("5. Keep each card file in the established `definition() + <card>_play(...)` shape.")
    lines.append("")
    lines.append("## Existing Rust Orb Support")
    lines.append("")
    lines.append("| Area | Status | Notes |")
    lines.append("| --- | --- | --- |")
    lines.append("| Orb IDs | partial | `Empty`, `Lightning`, `Dark`, `Frost`, `Plasma` exist. |")
    lines.append("| Orb values | partial | Passive/evoke values and Focus refresh exist. |")
    lines.append("| Channel | partial | `Action::ChannelOrb` channels or front-queues evoke+channel when full. Needs Java parity tests for multi-channel ordering. |")
    lines.append("| Evoke | partial | `Action::EvokeOrb` evokes one front orb and removes it. Defect cards need multiple evoke and evoke-without-removing semantics. |")
    lines.append("| Start/end hooks | partial | Start/end orb triggers exist, including Gold Plated Cables handling. |")
    lines.append("| Random orb channeling | missing | Needed for `Chaos`. |")
    lines.append("| Max orb decrease | missing | Needed for `Consume`; must evoke/drop slots exactly like Java. |")
    lines.append("| Orb history | likely missing | `Blizzard` and `Thunder Strike` depend on combat orb-channel history. |")
    lines.append("")
    lines.append("## Registered Card Table")
    lines.append("")
    lines.append("| Java class | ID | Cost | Type | Rarity | Target | D/B/M | Flags | Tags | Evoke UI | Upgrade ops | Use dependencies | Orbs | Powers | Hooks | Notes |")
    lines.append("| --- | --- | ---: | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |")
    for card in registered_cards:
        lines.append(
            "| "
            + " | ".join(
                md_escape(v)
                for v in [
                    card.class_name,
                    card.card_id,
                    card.cost,
                    card.card_type,
                    card.rarity,
                    card.target,
                    f"{card.base_damage}/{card.base_block}/{card.base_magic}",
                    join(card.flags),
                    join(card.tags),
                    card.show_evoke or "-",
                    join(card.upgrades),
                    join(card.actions),
                    join(card.orbs),
                    join(card.powers),
                    join(card.hooks),
                    join(card.notes),
                ]
            )
            + " |"
        )
    lines.append("")
    lines.append("## Source-Present But Not Registered")
    lines.append("")
    lines.append("| Java class | ID | Reason |")
    lines.append("| --- | --- | --- |")
    for card in source_only:
        lines.append(f"| {card.class_name} | {md_escape(card.card_id)} | Present in `cards/blue`, absent from `CardLibrary.addBlueCards()`. Treat as non-pool/deprecated until proven otherwise. |")
    lines.append("")
    lines.append("## Action Dependency Inventory")
    lines.append("")
    lines.append("| Java action | Rust status |")
    lines.append("| --- | --- |")
    for action in all_actions:
        status = action_status(action)
        lines.append(f"| `{action}` | {md_escape(status.split(': ', 1)[1])} |")
    lines.append("")
    lines.append("## Orb Dependency Inventory")
    lines.append("")
    lines.append("| Orb | Rust status |")
    lines.append("| --- | --- |")
    for orb in all_orbs:
        lines.append(f"| `{orb}` | `OrbId::{orb}` exists; card-specific channel/evoke ordering still needs tests. |")
    lines.append("")
    lines.append("## Power Dependency Inventory")
    lines.append("")
    lines.append("| Java power | Rust status |")
    lines.append("| --- | --- |")
    for power in all_powers:
        lines.append(f"| `{power}` | {md_escape(EXISTING_RUST_DEFECT_POWERS.get(power, 'missing or not audited for Defect card behavior'))} |")
    lines.append("")
    lines.append("## Early Risk Notes")
    lines.append("")
    lines.append("- `Dualcast`, `Multi-Cast`, `Fission`, `Recursion`, and `Consume` should not be implemented as card-local hacks; they require reusable orb actions.")
    lines.append("- `Blizzard` and `Thunder Strike` require combat orb-channel history, not only current orb slots.")
    lines.append("- `Genetic Algorithm`, `Claw`, `Streamline`, and `Steam Barrier` mutate card instance fields and need UUID/all-instances/cost/block mutation parity.")
    lines.append("- `Hologram`, `Seek`, `Recycle`, and `Scrape` depend on choice/draw/discard mechanics from the high-risk backlog.")
    lines.append("- `Echo Form`, `Amplify`, `Heatsinks`, `Hello World`, `Creative AI`, `Loop`, `Static Discharge`, and `Storm` require real power hooks before their cards are meaningful.")
    lines.append("")
    lines.append("## Verification")
    lines.append("")
    lines.append("Regenerate this file with:")
    lines.append("")
    lines.append("```powershell")
    lines.append("python tools/audit_defect_card_source.py")
    lines.append("```")
    lines.append("")
    return "\n".join(lines) + "\n"


def main() -> None:
    if not BLUE_ROOT.exists():
        raise SystemExit(f"missing Java source directory: {BLUE_ROOT}")
    content = generate()
    previous = OUTPUT.read_text(encoding="utf-8") if OUTPUT.exists() else None
    if previous == content:
        print(f"unchanged {OUTPUT}")
    else:
        OUTPUT.write_text(content, encoding="utf-8", newline="\n")
        print(f"wrote {OUTPUT}")


if __name__ == "__main__":
    main()
