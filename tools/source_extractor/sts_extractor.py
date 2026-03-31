#!/usr/bin/env python3
"""
StS Java Source Extractor
=========================
Uses tree-sitter to extract structured info from decompiled Slay the Spire Java source.

Usage:
    pip install tree-sitter tree-sitter-java
    python sts_extractor.py /path/to/decompiled/java/files /path/to/output/dir

Outputs:
    inheritance.md   - Class hierarchy (who extends what)
    hooks.md         - Which classes override which hook methods
    actions.md       - Action call chains per card/power/relic
    call_graph.md    - Method-level call graph for key classes
    damage_pipeline.md - Damage calculation flow details
    cards.md         - Per-card use() breakdown
    powers.md        - Per-power hook implementations
    relics.md        - Per-relic hook implementations
    scattered_logic.md - CRITICAL: where relic/power logic lives OUTSIDE their own class
    completeness_checklist.md - Tracking table: own-class vs engine-side implementation status
    taint_report.md  - Line-level presentation vs logic classification (MIXED = needs review)
"""

import sys
import os
import json
from pathlib import Path
from collections import defaultdict
from dataclasses import dataclass, field
from typing import Optional

import tree_sitter_java as tsjava
from tree_sitter import Language, Parser

# ── Setup ──────────────────────────────────────────────────────────────────

JAVA_LANGUAGE = Language(tsjava.language())

parser = Parser(JAVA_LANGUAGE)

# Known base classes we care about
BASE_CLASSES = {
    "AbstractCard", "AbstractRelic", "AbstractPower", "AbstractAction",
    "AbstractMonster", "AbstractPlayer", "AbstractRoom", "AbstractDungeon",
    "AbstractCreature", "AbstractPotion", "AbstractOrb",
}

# Known hook methods (from AbstractPower, AbstractRelic, etc.)
KNOWN_HOOKS = {
    # AbstractPower hooks
    "atStartOfTurn", "atStartOfTurnPostDraw", "atEndOfTurn", "atEndOfTurnPreEndTurnCards",
    "atEndOfRound", "duringTurn", "onPlayCard", "onUseCard", "onAfterUseCard",
    "onCardDraw", "onDrawOrDiscard", "atDamageGive", "atDamageReceive",
    "atDamageFinalGive", "atDamageFinalReceive", "onAttack", "onAttacked",
    "onAttackedToChangeDamage", "onInflictDamage", "onSpecificTrigger",
    "onApplyPower", "onGainedBlock", "onRemove", "onDeath", "onChannel",
    "onEvokeOrb", "onInitialApplication", "onStackPower", "reducePower",
    "makeCopy", "updateDescription", "onAfterCardPlayed", "onHeal",
    "onVictory", "onExhaust", "onChangeStance",
    # AbstractRelic hooks
    "atBattleStart", "atBattleStartPreDraw", "atPreBattle",
    "onPlayerEndTurn", "onPlayerGainBlock", "onPlayerGainedBlock",
    "onMonsterDeath", "onEquip", "onUnequip", "onObtainCard",
    "onMasterDeckChange", "onPreApplyPower", "onLoseHp", "onGainHp",
    "onBlockBroken", "onTrigger", "onShuffle", "onSmith",
    "onEnterRoom", "onEnterRestRoom", "justEnteredRoom",
    "onChestOpen", "onSpawnMonster", "onManualDiscard",
    "onPlayerHeal", "wasHPLost",
    # AbstractCard hooks
    "use", "upgrade", "makeStatEquivalentCopy", "makeCopy",
    "triggerOnGlowCheck", "triggerOnExhaust", "triggerOnEndOfPlayerTurn",
    "triggerOnEndOfTurnForPlayingCard", "triggerWhenDrawn",
    "triggerOnManualDiscard", "triggerOnOtherCardPlayed",
    "triggerAtStartOfTurn", "canUse",
}


# ── AST Helpers ────────────────────────────────────────────────────────────

def read_source(path: Path) -> tuple[str, object]:
    """Read and parse a Java file, return (source_text, tree)."""
    source = path.read_bytes()
    tree = parser.parse(source)
    return source.decode("utf-8", errors="replace"), tree


def node_text(node, source: str) -> str:
    """Extract text of an AST node."""
    return source[node.start_byte:node.end_byte]


def find_children_by_type(node, type_name: str):
    """Find all direct children of a given type."""
    return [c for c in node.children if c.type == type_name]


def find_descendants_by_type(node, type_name: str):
    """Recursively find all descendants of a given type."""
    results = []
    for c in node.children:
        if c.type == type_name:
            results.append(c)
        results.extend(find_descendants_by_type(c, type_name))
    return results


def find_class_declarations(tree):
    """Find all class_declaration nodes in a tree."""
    return find_descendants_by_type(tree.root_node, "class_declaration")


def get_class_name(class_node, source: str) -> Optional[str]:
    """Get the name of a class declaration."""
    for child in class_node.children:
        if child.type == "identifier":
            return node_text(child, source)
    return None


def get_superclass(class_node, source: str) -> Optional[str]:
    """Get the superclass name from a class declaration."""
    for child in class_node.children:
        if child.type == "superclass":
            for sc in child.children:
                if sc.type == "type_identifier":
                    return node_text(sc, source)
    return None


def get_interfaces(class_node, source: str) -> list[str]:
    """Get implemented interfaces."""
    interfaces = []
    for child in class_node.children:
        if child.type == "super_interfaces":
            for ti in find_descendants_by_type(child, "type_identifier"):
                interfaces.append(node_text(ti, source))
    return interfaces


def get_methods(class_node, source: str) -> list[dict]:
    """Extract all method declarations from a class body."""
    methods = []
    body = None
    for child in class_node.children:
        if child.type == "class_body":
            body = child
            break
    if not body:
        return methods

    for member in body.children:
        if member.type == "method_declaration":
            method_name = None
            params = ""
            for c in member.children:
                if c.type == "identifier":
                    method_name = node_text(c, source)
                elif c.type == "formal_parameters":
                    params = node_text(c, source)
            if method_name:
                methods.append({
                    "name": method_name,
                    "params": params,
                    "node": member,
                    "start_line": member.start_point[0] + 1,
                    "end_line": member.end_point[0] + 1,
                })
    return methods


def extract_method_calls(method_node, source: str) -> list[dict]:
    """Extract all method calls within a method body, in order of appearance."""
    calls = []
    invocations = find_descendants_by_type(method_node, "method_invocation")
    for inv in sorted(invocations, key=lambda n: n.start_byte):
        call_text = node_text(inv, source)
        # Try to get just the method name
        method_name = None
        for c in inv.children:
            if c.type == "identifier":
                method_name = node_text(c, source)
        calls.append({
            "text": call_text,
            "method": method_name,
            "line": inv.start_point[0] + 1,
        })
    return calls


def extract_object_creations(method_node, source: str) -> list[dict]:
    """Extract all 'new XxxAction(...)' style object creations in order."""
    creations = []
    news = find_descendants_by_type(method_node, "object_creation_expression")
    for n in sorted(news, key=lambda x: x.start_byte):
        type_id = None
        for c in n.children:
            if c.type == "type_identifier":
                type_id = node_text(c, source)
        full_text = node_text(n, source)
        # Truncate if too long
        if len(full_text) > 200:
            full_text = full_text[:200] + "..."
        creations.append({
            "type": type_id,
            "text": full_text,
            "line": n.start_point[0] + 1,
        })
    return creations


# ── Line-level Taint Classification ────────────────────────────────────────
#
# Classifies each line of a method body as PRESENTATION, LOGIC, or MIXED.
# This lets the agent know which parts to skip, which to implement,
# and which need careful inspection because they interleave both.

# Presentation-layer indicators (UI, sound, animation, rendering)
_PRES_KEYWORDS = {
    # Animation / timing
    "getDeltaTime", "deltaTime", "Gdx.graphics", "this.duration",
    "tickDuration", "MathUtils.lerp", "Interpolation",
    # Rendering / visual
    "render", "draw", "SpriteBatch", "TextureAtlas", "Color.",
    "setScale", "setRotation", "setPosition", "setAlpha",
    "FontHelper", "drawString", "renderText",
    "AbstractGameEffect", "VfxBuilder", "AbstractGameAction.AttackEffect",
    "FireBurstParticleEffect", "FlashAtkImgEffect", "DamageNumberEffect",
    # Sound
    "CardCrawlGame.sound", "SoundMaster", ".playA(", ".playV(",
    ".play(", "SOUND_",
    # UI
    "flash(", "beginLongPulse", "stopPulse", "grayscale",
    "updateDescription", "AbstractDungeon.effectList",
    "AbstractDungeon.effectsQueue", "AbstractDungeon.topLevelEffects",
    "AbstractDungeon.actionManager.addToTop(new WaitAction",
    "AbstractDungeon.actionManager.addToTop(new SFXAction",
    "WaitAction", "SFXAction", "VFXAction", "TextAboveCreatureAction",
    "showEnergyCount", "AbstractDungeon.topPanel",
    "TalkAction", "TextAboveCreatureAction", "FlashAtkImgEffect",
}

# Logic-layer indicators (game state changes)
_LOGIC_KEYWORDS = {
    # State mutation
    ".currentHealth", ".maxHealth", ".block", ".gold", ".energy",
    "reduceHealth", "loseHp", "heal(", "addBlock",
    "DamageAction", "LoseHPAction", "HealAction", "GainBlockAction",
    "DrawCardAction", "DiscardAction", "ExhaustAction", "MakeTempCardAction",
    "ApplyPowerAction", "RemoveSpecificPowerAction", "ReducePowerAction",
    "GainEnergyAction", "LoseEnergyAction",
    # Card manipulation
    "drawCards", "moveToDiscard", "moveToExhaustPile", "moveToHand",
    "moveToBottomOfDeck", "moveToDeck", "purge(", "makeInHand",
    # Damage/block calculation
    "atDamageGive", "atDamageReceive", "atDamageFinal",
    "calculateCardDamage", "applyPowers",
    # Action queue
    "addToBot(", "addToTop(", "addToBottom(",
    # Power/relic effects
    "hasRelic(", "hasPower(", "getPower(", "getRelic(",
    "AbstractDungeon.player.relics", "AbstractDungeon.player.powers",
    "stackPower", "reducePower", "onSpecificTrigger",
    # RNG
    "cardRandomRng", "miscRng", "monsterRng", "relicRng",
    # Conditional game logic
    "isPlayer", "isDying", "isDead", "halfDead",
    "AbstractDungeon.getCurrRoom", "AbstractDungeon.getMonsters",
    # Turn/combat flow
    "endTurn", "endOfTurnActions", "atStartOfTurn",
}


def classify_line(line: str) -> str:
    """
    Classify a single line of code.
    Returns: 'PRES' | 'LOGIC' | 'MIXED' | 'NEUTRAL'
    """
    stripped = line.strip()
    if not stripped or stripped.startswith("//") or stripped in ("{", "}"):
        return "NEUTRAL"

    has_pres = any(kw in line for kw in _PRES_KEYWORDS)
    has_logic = any(kw in line for kw in _LOGIC_KEYWORDS)

    if has_pres and has_logic:
        return "MIXED"
    elif has_pres:
        return "PRES"
    elif has_logic:
        return "LOGIC"
    else:
        return "NEUTRAL"


def classify_method(body_text: str) -> dict:
    """
    Classify all lines of a method body.
    Returns summary stats + per-line annotations.
    """
    lines = body_text.split("\n")
    annotations = []
    counts = {"PRES": 0, "LOGIC": 0, "MIXED": 0, "NEUTRAL": 0}

    for line in lines:
        tag = classify_line(line)
        counts[tag] += 1
        annotations.append((tag, line))

    total_meaningful = counts["PRES"] + counts["LOGIC"] + counts["MIXED"]
    if total_meaningful == 0:
        method_class = "NEUTRAL"
    elif counts["MIXED"] > 0 or (counts["PRES"] > 0 and counts["LOGIC"] > 0):
        method_class = "MIXED"
    elif counts["PRES"] > 0:
        method_class = "PURE_PRES"
    else:
        method_class = "PURE_LOGIC"

    return {
        "classification": method_class,
        "counts": counts,
        "annotations": annotations,
    }


def write_taint_report(classes: list[ClassInfo], out_dir: Path):
    """
    Write taint_report.md - line-level classification of presentation vs logic.
    Focuses on MIXED methods (the dangerous ones) and PURE_PRES methods that
    might be hiding logic.
    """
    mixed_methods = []   # (class_name, method_name, classification_result)
    pure_pres = []
    summary_stats = {"PURE_LOGIC": 0, "PURE_PRES": 0, "MIXED": 0, "NEUTRAL": 0}

    for c in classes:
        for m in c.methods:
            result = classify_method(m.get("body_text", ""))
            summary_stats[result["classification"]] += 1

            if result["classification"] == "MIXED":
                mixed_methods.append((c, m, result))
            elif result["classification"] == "PURE_PRES":
                pure_pres.append((c, m, result))

    with open(out_dir / "taint_report.md", "w", encoding="utf-8") as f:
        f.write("# StS Presentation vs Logic Taint Report\n\n")
        f.write("Line-level classification of every method in the codebase.\n\n")
        f.write("- **LOGIC**: Pure game logic, port directly to Rust\n")
        f.write("- **PRES**: Pure presentation (animation/sound/UI), skip for headless sim\n")
        f.write("- **MIXED**: Logic and presentation interleaved, requires manual review\n")
        f.write("- **NEUTRAL**: Boilerplate (variable declarations, braces, etc.)\n\n")

        f.write(f"## Summary\n\n")
        f.write(f"- Pure logic methods: {summary_stats['PURE_LOGIC']}\n")
        f.write(f"- Pure presentation methods: {summary_stats['PURE_PRES']}\n")
        f.write(f"- **MIXED methods (need review): {summary_stats['MIXED']}**\n")
        f.write(f"- Neutral: {summary_stats['NEUTRAL']}\n\n")

        # ── Mixed methods (the dangerous ones) ──
        f.write(f"## ⚠️ MIXED Methods ({len(mixed_methods)})\n\n")
        f.write("These methods interleave presentation and logic code.\n")
        f.write("**Do NOT skip these entirely.** Extract the LOGIC lines, ignore the PRES lines.\n")
        f.write("Lines marked MIXED need individual inspection.\n\n")

        for c, m, result in sorted(mixed_methods, key=lambda x: (x[0].name, x[1]["name"])):
            f.write(f"### {c.name}.{m['name']} ({c.category})\n")
            f.write(f"File: `{c.file_path}` L{m['start_line']}-{m['end_line']}\n")
            f.write(f"Lines: {result['counts']['LOGIC']} logic, "
                    f"{result['counts']['PRES']} pres, "
                    f"{result['counts']['MIXED']} mixed\n\n")
            f.write("```java\n")
            for tag, line in result["annotations"]:
                if tag == "NEUTRAL":
                    f.write(f"         {line}\n")
                else:
                    f.write(f"[{tag:5s}] {line}\n")
            f.write("```\n\n")

        # ── Pure presentation methods ──
        f.write(f"## Pure Presentation Methods ({len(pure_pres)})\n\n")
        f.write("These can be skipped for headless simulation.\n")
        f.write("Spot-check: if any of these are in a class you're implementing, verify nothing was misclassified.\n\n")

        for c, m, result in sorted(pure_pres, key=lambda x: (x[0].name, x[1]["name"])):
            f.write(f"- `{c.name}.{m['name']}` ({c.category}) — `{c.file_path}` L{m['start_line']}\n")

    print(f"  taint_report.md: {len(mixed_methods)} mixed methods, {len(pure_pres)} pure-pres methods")




@dataclass
class ClassInfo:
    name: str
    superclass: Optional[str]
    interfaces: list[str]
    file_path: str
    methods: list[dict]
    category: str = ""  # card, power, relic, action, etc.


def categorize(class_info: ClassInfo, inheritance_map: dict) -> str:
    """Determine category by walking up inheritance chain."""
    categories = {
        "AbstractCard": "card",
        "AbstractPower": "power",
        "AbstractRelic": "relic",
        "AbstractAction": "action",
        "AbstractMonster": "monster",
        "AbstractPotion": "potion",
        "AbstractOrb": "orb",
        "AbstractRoom": "room",
    }
    visited = set()
    current = class_info.name
    while current and current not in visited:
        if current in categories:
            return categories[current]
        visited.add(current)
        current = inheritance_map.get(current)
    return "other"


def scan_all_files(src_dir: Path) -> list[ClassInfo]:
    """Scan all .java files and extract class info."""
    classes = []
    java_files = list(src_dir.rglob("*.java"))
    print(f"Found {len(java_files)} Java files")

    for fpath in java_files:
        try:
            source, tree = read_source(fpath)
        except Exception as e:
            print(f"  SKIP {fpath}: {e}")
            continue

        for class_node in find_class_declarations(tree):
            name = get_class_name(class_node, source)
            if not name:
                continue
            superclass = get_superclass(class_node, source)
            interfaces = get_interfaces(class_node, source)
            methods = get_methods(class_node, source)

            # Enrich methods with call info
            for m in methods:
                m["calls"] = extract_method_calls(m["node"], source)
                m["creations"] = extract_object_creations(m["node"], source)
                m["body_text"] = node_text(m["node"], source)
                # Don't carry the AST node into output
                del m["node"]

            classes.append(ClassInfo(
                name=name,
                superclass=superclass,
                interfaces=interfaces,
                file_path=str(fpath.relative_to(src_dir)),
                methods=methods,
            ))

    return classes


# ── Reverse Index: Scattered Logic Detection ──────────────────────────────

# Patterns that indicate logic-at-a-distance:
# engine code checking for a specific relic/power instead of the relic/power handling its own logic
SCATTER_PATTERNS = [
    "hasRelic", "getRelic",
    "hasPower", "getPower",
    "hasPotion",
    "hasOrb",
]

import re

# Regex to match e.g. hasRelic("Runic Dome") or hasRelic(RunicDome.ID)
SCATTER_RE = re.compile(
    r'\b(hasRelic|getRelic|hasPower|getPower|hasPotion|hasOrb)\s*\(\s*'
    r'(?:"([^"]+)"|(\w+)\.ID|(\w+)\.POWER_ID)'
)


def scan_scattered_logic(src_dir: Path) -> dict[str, list[dict]]:
    """
    Scan ALL Java files for hasRelic/hasPower/etc calls.
    Returns: { "entity_name": [ {file, class, method, line, context, check_type} ] }
    """
    refs = defaultdict(list)

    for fpath in src_dir.rglob("*.java"):
        try:
            source = fpath.read_text(errors="replace")
        except Exception:
            continue

        for i, line in enumerate(source.split("\n"), 1):
            for match in SCATTER_RE.finditer(line):
                check_fn = match.group(1)          # hasRelic / getPower / etc
                entity = match.group(2) or match.group(3) or match.group(4) or "UNKNOWN"
                # grab surrounding context (the full line, trimmed)
                ctx = line.strip()
                if len(ctx) > 200:
                    ctx = ctx[:200] + "..."

                refs[entity].append({
                    "file": str(fpath.relative_to(src_dir)),
                    "line": i,
                    "check_fn": check_fn,
                    "context": ctx,
                })

    return dict(refs)


def write_scattered_logic(classes: list[ClassInfo], scattered: dict, out_dir: Path):
    """Write scattered_logic.md - reverse index of where relic/power logic actually lives."""
    # Build a set of known relic/power class names for cross-referencing
    relic_names = {c.name for c in classes if c.category == "relic"}
    power_names = {c.name for c in classes if c.category == "power"}

    with open(out_dir / "scattered_logic.md", "w", encoding="utf-8") as f:
        f.write("# StS Scattered Logic Index\n\n")
        f.write("**This file is critical.** Many relic/power behaviors are not in their own classes,\n")
        f.write("but scattered across engine code via `hasRelic()`/`hasPower()` checks.\n")
        f.write("When porting to Rust, translating the relic/power class alone is insufficient —\n")
        f.write("the corresponding engine-side checks must also be implemented.\n\n")
        f.write("Format: For each entity checked by the engine, all check sites (file, line, context) are listed.\n\n")

        # Separate into relics, powers, and others
        relic_refs = {}
        power_refs = {}
        other_refs = {}

        for entity, locs in sorted(scattered.items()):
            # Classify by check function
            check_types = {loc["check_fn"] for loc in locs}
            if any("Relic" in ct for ct in check_types):
                relic_refs[entity] = locs
            elif any("Power" in ct for ct in check_types):
                power_refs[entity] = locs
            else:
                other_refs[entity] = locs

        # ── Relic scattered logic ──
        f.write(f"## RELIC Scattered Logic ({len(relic_refs)} relics checked by engine)\n\n")
        for entity, locs in sorted(relic_refs.items()):
            # Check if this relic's own class has meaningful hooks
            own_hooks = []
            for c in classes:
                if c.name == entity or c.name == entity.replace(" ", ""):
                    own_hooks = [m["name"] for m in c.methods if m["name"] in KNOWN_HOOKS]
            hook_note = f" (own hooks: {', '.join(own_hooks)})" if own_hooks else " **(NO hooks in own class — logic is ENTIRELY engine-side)**"

            f.write(f"### {entity}{hook_note}\n\n")
            f.write(f"Referenced in {len(locs)} locations:\n\n")
            for loc in sorted(locs, key=lambda x: (x["file"], x["line"])):
                f.write(f"- `{loc['file']}` L{loc['line']}: `{loc['context']}`\n")
            f.write("\n")

        # ── Power scattered logic ──
        f.write(f"## POWER Scattered Logic ({len(power_refs)} powers checked by engine)\n\n")
        for entity, locs in sorted(power_refs.items()):
            own_hooks = []
            for c in classes:
                if c.name == entity or c.name == entity.replace(" ", ""):
                    own_hooks = [m["name"] for m in c.methods if m["name"] in KNOWN_HOOKS]
            hook_note = f" (own hooks: {', '.join(own_hooks)})" if own_hooks else " **(NO hooks in own class)**"

            f.write(f"### {entity}{hook_note}\n\n")
            f.write(f"Referenced in {len(locs)} locations:\n\n")
            for loc in sorted(locs, key=lambda x: (x["file"], x["line"])):
                f.write(f"- `{loc['file']}` L{loc['line']}: `{loc['context']}`\n")
            f.write("\n")

        # ── Other ──
        if other_refs:
            f.write(f"## OTHER Scattered Logic\n\n")
            for entity, locs in sorted(other_refs.items()):
                f.write(f"### {entity}\n\n")
                for loc in sorted(locs, key=lambda x: (x["file"], x["line"])):
                    f.write(f"- `{loc['file']}` L{loc['line']}: `{loc['context']}`\n")
                f.write("\n")

    # ── Also write a completeness checklist ──
    with open(out_dir / "completeness_checklist.md", "w", encoding="utf-8") as f:
        f.write("# StS Implementation Completeness Checklist\n\n")
        f.write("Each relic/power implementation must cover two parts: own-class logic + engine-side logic.\n")
        f.write("Use this checklist to track progress and avoid missing scattered engine hooks.\n\n")

        f.write("## Relics\n\n")
        f.write("| Relic | Own Class Hooks | Engine-side Checks | Self Done | Engine Done |\n")
        f.write("|---|---|---|---|---|\n")
        for c in sorted(classes, key=lambda x: x.name):
            if c.category != "relic":
                continue
            own_hooks = [m["name"] for m in c.methods if m["name"] in KNOWN_HOOKS]
            # Find scattered refs for this relic
            # Try multiple name formats: class name, or string with spaces
            engine_count = 0
            for entity, locs in scattered.items():
                # Normalize: "Runic Dome" matches RunicDome, etc.
                entity_normalized = entity.replace(" ", "").lower()
                class_normalized = c.name.lower()
                if entity_normalized == class_normalized:
                    engine_count = len(locs)
                    break
            own_str = ", ".join(own_hooks) if own_hooks else "NONE"
            engine_str = f"{engine_count} refs" if engine_count > 0 else "0"
            f.write(f"| {c.name} | {own_str} | {engine_str} | [ ] | [ ] |\n")

        f.write("\n## Powers\n\n")
        f.write("| Power | Own Class Hooks | Engine-side Checks | Self Done | Engine Done |\n")
        f.write("|---|---|---|---|---|\n")
        for c in sorted(classes, key=lambda x: x.name):
            if c.category != "power":
                continue
            own_hooks = [m["name"] for m in c.methods if m["name"] in KNOWN_HOOKS]
            engine_count = 0
            for entity, locs in scattered.items():
                entity_normalized = entity.replace(" ", "").lower()
                class_normalized = c.name.lower()
                if entity_normalized == class_normalized:
                    engine_count = len(locs)
                    break
            own_str = ", ".join(own_hooks) if own_hooks else "NONE"
            engine_str = f"{engine_count} refs" if engine_count > 0 else "0"
            f.write(f"| {c.name} | {own_str} | {engine_str} | [ ] | [ ] |\n")

    total_scattered = sum(len(v) for v in scattered.values())
    print(f"  scattered_logic.md: {len(scattered)} entities, {total_scattered} total references")
    print(f"  completeness_checklist.md: written")


# ── Output Generators ──────────────────────────────────────────────────────

def write_inheritance(classes: list[ClassInfo], out_dir: Path):
    """Write inheritance.md - full class hierarchy."""
    inheritance_map = {c.name: c.superclass for c in classes}

    # Group by category
    by_category = defaultdict(list)
    for c in classes:
        by_category[c.category].append(c)

    with open(out_dir / "inheritance.md", "w", encoding="utf-8") as f:
        f.write("# StS Class Inheritance Map\n\n")
        f.write("Format: `ClassName -> ParentClass` (file: path)\n\n")
        for cat in ["action", "card", "power", "relic", "monster", "potion", "orb", "room", "other"]:
            items = by_category.get(cat, [])
            if not items:
                continue
            f.write(f"## {cat.upper()} ({len(items)} classes)\n\n")
            for c in sorted(items, key=lambda x: x.name):
                parent = c.superclass or "(none)"
                f.write(f"- `{c.name}` -> `{parent}` (file: `{c.file_path}`)\n")
            f.write("\n")

    print(f"  inheritance.md: {len(classes)} classes")


def write_hooks(classes: list[ClassInfo], out_dir: Path):
    """Write hooks.md - which classes override which hook methods."""
    with open(out_dir / "hooks.md", "w", encoding="utf-8") as f:
        f.write("# StS Hook Override Table\n\n")
        f.write("Shows which hook methods each power/relic/card overrides.\n\n")

        for cat in ["power", "relic", "card"]:
            items = [c for c in classes if c.category == cat]
            if not items:
                continue
            f.write(f"## {cat.upper()} Hooks\n\n")
            for c in sorted(items, key=lambda x: x.name):
                hooks_found = []
                for m in c.methods:
                    if m["name"] in KNOWN_HOOKS:
                        hooks_found.append(m["name"])
                if hooks_found:
                    f.write(f"### {c.name}\n")
                    f.write(f"File: `{c.file_path}`\n\n")
                    for h in hooks_found:
                        f.write(f"- `{h}`\n")
                    f.write("\n")

    print(f"  hooks.md: written")


def write_actions(classes: list[ClassInfo], out_dir: Path):
    """Write actions.md - each Action subclass with its update() logic."""
    actions = [c for c in classes if c.category == "action"]

    with open(out_dir / "actions.md", "w", encoding="utf-8") as f:
        f.write("# StS Action Reference\n\n")
        f.write(f"Total Action subclasses: {len(actions)}\n\n")

        for c in sorted(actions, key=lambda x: x.name):
            f.write(f"## {c.name}\n")
            f.write(f"File: `{c.file_path}`\n")
            if c.superclass:
                f.write(f"Extends: `{c.superclass}`\n")

            update_method = None
            for m in c.methods:
                if m["name"] == "update":
                    update_method = m
                    break

            if update_method:
                f.write(f"Lines: {update_method['start_line']}-{update_method['end_line']}\n\n")
                # Show object creations (sub-actions spawned)
                if update_method["creations"]:
                    f.write("**Creates:**\n")
                    for cr in update_method["creations"]:
                        f.write(f"- L{cr['line']}: `{cr['type']}` — `{cr['text']}`\n")
                    f.write("\n")
                # Show key method calls
                key_calls = [
                    call for call in update_method["calls"]
                    if call["method"] in ("addToTop", "addToBot", "addToBottom",
                                          "isDone", "tickDuration", "isDead")
                    or (call["method"] and "damage" in call["method"].lower())
                    or (call["method"] and "block" in call["method"].lower())
                ]
                if key_calls:
                    f.write("**Key calls:**\n")
                    for call in key_calls:
                        f.write(f"- L{call['line']}: `{call['text']}`\n")
                    f.write("\n")
                # Show full method body
                f.write("<details><summary>Full update() body</summary>\n\n")
                f.write("```java\n")
                f.write(update_method["body_text"])
                f.write("\n```\n\n</details>\n\n")
            else:
                f.write("(no update() method found)\n\n")

    print(f"  actions.md: {len(actions)} actions")


def write_cards(classes: list[ClassInfo], out_dir: Path):
    """Write cards.md - per-card use() breakdown."""
    cards = [c for c in classes if c.category == "card"]

    with open(out_dir / "cards.md", "w", encoding="utf-8") as f:
        f.write("# StS Card Reference\n\n")
        f.write(f"Total Card subclasses: {len(cards)}\n\n")

        for c in sorted(cards, key=lambda x: x.name):
            use_method = None
            for m in c.methods:
                if m["name"] == "use":
                    use_method = m
                    break
            if not use_method:
                continue

            f.write(f"## {c.name}\n")
            f.write(f"File: `{c.file_path}`\n\n")

            # Show action sequence
            if use_method["creations"]:
                f.write("**Action sequence (in order):**\n")
                for cr in use_method["creations"]:
                    f.write(f"- `{cr['type']}` — `{cr['text']}`\n")
                f.write("\n")

            # Show addToBot/addToTop calls to reveal ordering
            bot_top_calls = [
                call for call in use_method["calls"]
                if call["method"] in ("addToBot", "addToTop", "addToBottom")
            ]
            if bot_top_calls:
                f.write("**Queue order:**\n")
                for call in bot_top_calls:
                    f.write(f"- L{call['line']}: `{call['text']}`\n")
                f.write("\n")

            f.write("<details><summary>Full use() body</summary>\n\n")
            f.write("```java\n")
            f.write(use_method["body_text"])
            f.write("\n```\n\n</details>\n\n")

    print(f"  cards.md: {len(cards)} cards")


def write_powers(classes: list[ClassInfo], out_dir: Path):
    """Write powers.md - per-power hook implementations."""
    powers = [c for c in classes if c.category == "power"]

    with open(out_dir / "powers.md", "w", encoding="utf-8") as f:
        f.write("# StS Power Reference\n\n")
        f.write(f"Total Power subclasses: {len(powers)}\n\n")

        for c in sorted(powers, key=lambda x: x.name):
            hook_methods = [m for m in c.methods if m["name"] in KNOWN_HOOKS]
            if not hook_methods:
                continue

            f.write(f"## {c.name}\n")
            f.write(f"File: `{c.file_path}`\n\n")

            for m in hook_methods:
                f.write(f"### {m['name']}{m['params']}\n\n")
                if m["creations"]:
                    f.write("**Creates:**\n")
                    for cr in m["creations"]:
                        f.write(f"- `{cr['type']}` — `{cr['text']}`\n")
                    f.write("\n")
                f.write("<details><summary>Full body</summary>\n\n")
                f.write("```java\n")
                f.write(m["body_text"])
                f.write("\n```\n\n</details>\n\n")

    print(f"  powers.md: {len(powers)} powers")


def write_relics(classes: list[ClassInfo], out_dir: Path):
    """Write per-letter relic files under relics/ directory for fast lookup."""
    relics = [c for c in classes if c.category == "relic"]
    relics_dir = out_dir / "relics"
    relics_dir.mkdir(parents=True, exist_ok=True)

    # Group relics by first letter
    by_letter = defaultdict(list)
    for c in sorted(relics, key=lambda x: x.name):
        letter = c.name[0].upper() if c.name else "_"
        by_letter[letter].append(c)

    total_written = 0
    for letter, group in sorted(by_letter.items()):
        with open(relics_dir / f"{letter}.md", "w", encoding="utf-8") as f:
            f.write(f"# Relics: {letter}\n\n")
            f.write(f"{len(group)} relics\n\n")

            for c in group:
                hook_methods = [m for m in c.methods if m["name"] in KNOWN_HOOKS]
                if not hook_methods:
                    f.write(f"## {c.name}\n")
                    f.write(f"File: `{c.file_path}`\n\n")
                    f.write("*No hook methods — passive or out-of-combat only.*\n\n")
                    total_written += 1
                    continue

                f.write(f"## {c.name}\n")
                f.write(f"File: `{c.file_path}`\n\n")

                for m in hook_methods:
                    f.write(f"### {m['name']}{m['params']}\n\n")
                    if m["creations"]:
                        f.write("**Creates:**\n")
                        for cr in m["creations"]:
                            f.write(f"- `{cr['type']}` — `{cr['text']}`\n")
                        f.write("\n")

                    # Extract and show addToBot/addToTop calls prominently
                    bot_top_calls = [
                        call for call in m.get("calls", [])
                        if call["method"] in ("addToBot", "addToTop", "addToBottom")
                    ]
                    if bot_top_calls:
                        f.write("**Queue insertion:**\n")
                        for call in bot_top_calls:
                            mode = "BOT" if "addToBot" in call["text"] else "TOP"
                            f.write(f"- [{mode}] `{call['text'][:150]}`\n")
                        f.write("\n")

                    f.write("<details><summary>Full body</summary>\n\n")
                    f.write("```java\n")
                    f.write(m["body_text"])
                    f.write("\n```\n\n</details>\n\n")
                total_written += 1

    # Also write a small index file
    with open(relics_dir / "_index.md", "w", encoding="utf-8") as f:
        f.write("# Relic Reference Index\n\n")
        f.write(f"Total: {len(relics)} relics across {len(by_letter)} letter files\n\n")
        for letter, group in sorted(by_letter.items()):
            names = ", ".join(c.name for c in group)
            f.write(f"- **[{letter}.md]({letter}.md)** ({len(group)}): {names}\n")

    print(f"  relics/: {total_written} relics across {len(by_letter)} files")


def write_call_graph(classes: list[ClassInfo], out_dir: Path):
    """Write call_graph.md - method call sequences in critical classes."""
    critical_classes = {
        "AbstractRoom", "AbstractPlayer", "AbstractDungeon", "AbstractCreature",
        "AbstractMonster", "AbstractCard", "AbstractPower", "AbstractRelic",
        "ActionManager", "CardGroup", "MonsterGroup", "GameActionManager",
        "CombatRoom", "MonsterRoomBoss", "MonsterRoomElite",
    }

    with open(out_dir / "call_graph.md", "w", encoding="utf-8") as f:
        f.write("# StS Critical Call Graph\n\n")
        f.write("Method call sequences in core engine classes.\n\n")

        for c in sorted(classes, key=lambda x: x.name):
            if c.name not in critical_classes:
                continue

            f.write(f"## {c.name}\n")
            f.write(f"File: `{c.file_path}`\n\n")

            for m in c.methods:
                if not m["calls"] and not m["creations"]:
                    continue
                f.write(f"### {m['name']}{m['params']} (L{m['start_line']}-{m['end_line']})\n\n")

                if m["calls"]:
                    f.write("**Calls (in order):**\n")
                    for call in m["calls"][:50]:  # cap to avoid huge output
                        f.write(f"- L{call['line']}: `{call['text'][:120]}`\n")
                    if len(m["calls"]) > 50:
                        f.write(f"- ... ({len(m['calls']) - 50} more)\n")
                    f.write("\n")

                if m["creations"]:
                    f.write("**Objects created:**\n")
                    for cr in m["creations"]:
                        f.write(f"- L{cr['line']}: `{cr['type']}`\n")
                    f.write("\n")

    print(f"  call_graph.md: written")


def write_damage_pipeline(classes: list[ClassInfo], out_dir: Path):
    """Write damage_pipeline.md - damage calculation flow."""
    with open(out_dir / "damage_pipeline.md", "w", encoding="utf-8") as f:
        f.write("# StS Damage Pipeline\n\n")
        f.write("Extracted from AbstractCreature.damage(), AbstractCard.calculateCardDamage(),\n")
        f.write("and related methods. Check call order carefully for modifier application.\n\n")

        # Find relevant classes and methods
        damage_keywords = ["damage", "calculateCardDamage", "applyPowers",
                          "atDamageGive", "atDamageReceive", "atDamageFinalGive",
                          "atDamageFinalReceive", "onAttack", "onAttacked"]

        for c in sorted(classes, key=lambda x: x.name):
            relevant_methods = [
                m for m in c.methods
                if m["name"] in damage_keywords
                or (c.name in ("AbstractCreature", "AbstractPlayer", "AbstractMonster",
                              "AbstractCard", "DamageAction")
                    and m["name"] in ("damage", "update", "calculateCardDamage",
                                      "applyPowers", "applyPowersToBlock"))
            ]
            if not relevant_methods:
                continue

            f.write(f"## {c.name}\n")
            f.write(f"File: `{c.file_path}`\n\n")

            for m in relevant_methods:
                f.write(f"### {m['name']}{m['params']} (L{m['start_line']}-{m['end_line']})\n\n")
                f.write("```java\n")
                f.write(m["body_text"])
                f.write("\n```\n\n")

    print(f"  damage_pipeline.md: written")


# ── Summary Stats ──────────────────────────────────────────────────────────

def write_summary(classes: list[ClassInfo], out_dir: Path):
    """Write summary.md with counts and quick reference."""
    by_cat = defaultdict(list)
    for c in classes:
        by_cat[c.category].append(c.name)

    with open(out_dir / "summary.md", "w", encoding="utf-8") as f:
        f.write("# StS Extraction Summary\n\n")
        for cat in ["action", "card", "power", "relic", "monster", "potion", "orb", "room", "other"]:
            items = by_cat.get(cat, [])
            f.write(f"- **{cat}**: {len(items)} classes\n")
        f.write(f"\n**Total classes**: {len(classes)}\n\n")

        # List all hook methods actually found in the codebase
        all_hooks_used = set()
        for c in classes:
            for m in c.methods:
                if m["name"] in KNOWN_HOOKS:
                    all_hooks_used.add(m["name"])
        f.write(f"## Hook Methods Found ({len(all_hooks_used)})\n\n")
        for h in sorted(all_hooks_used):
            users = [c.name for c in classes if any(m["name"] == h for m in c.methods)]
            f.write(f"- `{h}` — used by {len(users)} classes\n")

    print(f"  summary.md: written")


# ── Main ───────────────────────────────────────────────────────────────────

def main():
    if len(sys.argv) < 2:
        print("Usage: python sts_extractor.py <java_source_dir> [output_dir]")
        print("  java_source_dir: directory containing decompiled .java files")
        print("  output_dir: where to write output (default: ./sts_extracted)")
        sys.exit(1)

    src_dir = Path(sys.argv[1])
    out_dir = Path(sys.argv[2]) if len(sys.argv) > 2 else Path("./sts_extracted")

    if not src_dir.exists():
        print(f"Error: {src_dir} does not exist")
        sys.exit(1)

    out_dir.mkdir(parents=True, exist_ok=True)

    print(f"Scanning {src_dir} ...")
    classes = scan_all_files(src_dir)

    # Build inheritance map and categorize
    inheritance_map = {c.name: c.superclass for c in classes}
    for c in classes:
        c.category = categorize(c, inheritance_map)

    print(f"\nFound {len(classes)} classes. Writing output to {out_dir}/\n")

    # Scan for scattered logic (hasRelic/hasPower checks across the codebase)
    print("Scanning for scattered logic (hasRelic/hasPower checks)...")
    scattered = scan_scattered_logic(src_dir)

    write_summary(classes, out_dir)
    write_inheritance(classes, out_dir)
    write_hooks(classes, out_dir)
    write_actions(classes, out_dir)
    write_cards(classes, out_dir)
    write_powers(classes, out_dir)
    write_relics(classes, out_dir)
    write_call_graph(classes, out_dir)
    write_damage_pipeline(classes, out_dir)
    write_scattered_logic(classes, scattered, out_dir)
    write_taint_report(classes, out_dir)

    print(f"\nDone! All output in {out_dir}/")
    print(f"Next: read AGENT_GUIDE.md for how to use these files with your AI agent.")


if __name__ == "__main__":
    main()
