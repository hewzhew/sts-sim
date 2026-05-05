#!/usr/bin/env python3
"""
engine_ast.py - Java AST Extractor for Slay the Spire Engine

Two modes:
  1. No arguments:  Extracts core engine loop classes (GameActionManager, AbstractRoom, etc.)
                    → output/engine_loop_ast.md

  2. With argument:  python engine_ast.py ApplyPower
                    Automatically finds the corresponding Java Action class(es) by name,
                    extracts their update() method, and prints to stdout + saves to
                    output/<name>_ast.md

     Matching rules:
       - "ApplyPower" → finds ApplyPowerAction.java
       - "Damage"     → finds DamageAction.java + AbstractPlayer.damage() + AbstractMonster.damage()
       - "GainBlock"  → finds GainBlockAction.java
       - Partial match supported: "Draw" → DrawCardAction, DrawPileToHandAction, etc.

Java source directory: d:\\rust\\cardcrawl
"""

import sys
from pathlib import Path

import tree_sitter_java as tsjava
from tree_sitter import Language, Parser

JAVA_LANGUAGE = Language(tsjava.language())
parser = Parser(JAVA_LANGUAGE)

JAVA_SRC = Path(r"d:\rust\cardcrawl")
OUTPUT_DIR = Path(r"d:\rust\sts_simulator\tools\source_extractor\output")

# Core engine loop files — always extracted in mode 1
CORE_TARGETS = {
    "GameActionManager.java": ["callEndOfTurnActions", "endTurn", "getNextAction"],
    "AbstractRoom.java": ["endTurn"],
    "MonsterGroup.java": ["applyPreTurnLogic", "applyEndOfTurnPowers"],
    "AbstractPlayer.java": ["applyEndOfTurnTriggers", "damage"],
    "AbstractMonster.java": ["damage"],
    "AbstractCreature.java": ["applyTurnPowers"],
}

# Supplementary files that are relevant when a specific action name is queried
# Maps action query keywords → extra files + methods to include
SUPPLEMENTARY = {
    "damage": {
        "AbstractPlayer.java": ["damage"],
        "AbstractMonster.java": ["damage"],
    },
    "heal": {
        "AbstractPlayer.java": ["heal"],
        "AbstractMonster.java": ["heal"],
        "AbstractCreature.java": ["heal"],
    },
}

# ---------------------------------------------------------------------------
# Tree-sitter helpers
# ---------------------------------------------------------------------------

def read_source(path: Path):
    source = path.read_bytes()
    tree = parser.parse(source)
    return source.decode("utf-8", errors="replace"), tree

def node_text(node, source: str) -> str:
    if not node: return ""
    return source[node.start_byte:node.end_byte]

def find_descendants_by_type(node, type_name: str, max_depth=None, current_depth=0):
    if max_depth is not None and current_depth > max_depth:
        return []
    results = []
    for c in node.children:
        if c.type == type_name:
            results.append(c)
        results.extend(find_descendants_by_type(c, type_name, max_depth, current_depth + 1))
    return results

def extract_statements(block_node, source, indent=""):
    out = []
    if not block_node: return out
    
    for c in block_node.children:
        if c.type in ["expression_statement", "continue_statement", "break_statement", "throw_statement"]:
            text = node_text(c, source).replace('\n', ' ').strip()
            out.append(f"{indent}- `{text}`")

        elif c.type == "return_statement":
            text = node_text(c, source).replace('\n', ' ').strip()
            out.append(f"{indent}- **RETURN** `{text}`")

        elif c.type == "local_variable_declaration":
            text = node_text(c, source).replace('\n', ' ').strip()
            out.append(f"{indent}- **VAR** `{text}`")

        elif c.type == "if_statement":
            condition = next((ch for ch in c.children if ch.type == "parenthesized_expression"), None)
            out.append(f"{indent}- **IF** `{node_text(condition, source)}`:")
            
            allowed_single_stmt_types = ["block", "expression_statement", "continue_statement", "break_statement", "return_statement", "throw_statement"]
            consequence = next((ch for ch in c.children if ch.type in allowed_single_stmt_types), None)
            
            if consequence and consequence.type == "block":
                out.extend(extract_statements(consequence, source, indent + "  "))
            elif consequence:
                out.append(f"{indent}  - `{node_text(consequence, source).strip()}`")

            # else / else-if branches
            found_first_block = False
            for ch in c.children:
                if ch.type == "block" and not found_first_block:
                    found_first_block = True
                    continue
                if found_first_block and ch.type == "block":
                    out.append(f"{indent}- **ELSE**:")
                    out.extend(extract_statements(ch, source, indent + "  "))
                elif found_first_block and ch.type == "if_statement":
                    cond2 = next((ch2 for ch2 in ch.children if ch2.type == "parenthesized_expression"), None)
                    out.append(f"{indent}- **ELSE IF** `{node_text(cond2, source)}`:")
                    body2 = next((ch2 for ch2 in ch.children if ch2.type == "block"), None)
                    if body2:
                        out.extend(extract_statements(body2, source, indent + "  "))
                    blocks2 = [ch2 for ch2 in ch.children if ch2.type == "block"]
                    if len(blocks2) > 1:
                        out.append(f"{indent}- **ELSE**:")
                        out.extend(extract_statements(blocks2[-1], source, indent + "  "))

        elif c.type == "enhanced_for_statement":
            iter_text = node_text(c, source).split(')')[0] + ')'
            out.append(f"{indent}- **FOR EACH** `{iter_text.replace('for (', '').strip()}`:")
            body = next((ch for ch in c.children if ch.type == "block"), None)
            if body:
                out.extend(extract_statements(body, source, indent + "  "))
    return out


def analyze_file(fpath: Path, target_methods):
    """Extract specified methods from a Java file. Returns markdown string or None."""
    try:
        source, tree = read_source(fpath)
    except Exception as e:
        return f"Failed to parse {fpath.name}: {e}"

    out = [f"# {fpath.name}  (`{fpath.relative_to(JAVA_SRC)}`)"]
    
    methods = find_descendants_by_type(tree.root_node, "method_declaration")
    found_any = False
    
    for m in methods:
        method_name = ""
        for c in m.children:
            if c.type == "identifier":
                method_name = node_text(c, source)
                break
                
        if method_name in target_methods:
            found_any = True
            out.append(f"## `{method_name}()`")
            body_node = next((c for c in m.children if c.type == "block"), None)
            stmts = extract_statements(body_node, source)
            if stmts:
                out.extend(stmts)
            else:
                out.append("- _(Empty or no parsable statements)_")
            out.append("\n")
            
    if not found_any:
        return None
    return "\n".join(out)


def find_action_files(query: str):
    """
    Given a Rust Action name like 'ApplyPower', find all matching Java Action files.
    
    Search strategy:
      1. Exact:  {query}Action.java  (e.g. ApplyPower → ApplyPowerAction.java)
      2. Fuzzy:  any *Action.java containing the query as substring (case-insensitive)
    """
    actions_dir = JAVA_SRC / "actions"
    exact_name = f"{query}Action.java"
    
    # Exact match first
    exact_matches = list(actions_dir.rglob(exact_name))
    if exact_matches:
        return exact_matches
    
    # Fuzzy match
    query_lower = query.lower()
    fuzzy = [f for f in actions_dir.rglob("*Action.java") 
             if query_lower in f.stem.lower() 
             and f.name != "AbstractGameAction.java"]
    return fuzzy


def find_supplementary_files(query: str):
    """Find supplementary files (e.g. AbstractPlayer.damage for 'Damage' query)."""
    query_lower = query.lower()
    targets = {}
    for keyword, file_methods in SUPPLEMENTARY.items():
        if keyword in query_lower:
            targets.update(file_methods)
    
    results = []
    if targets:
        for f in JAVA_SRC.rglob("*.java"):
            if f.name in targets:
                results.append((f, targets[f.name]))
    return results


# ---------------------------------------------------------------------------
# Main entry points
# ---------------------------------------------------------------------------

def run_core_extraction():
    """Mode 1: Extract core engine loop → output/engine_loop_ast.md"""
    all_reports = []
    for f in JAVA_SRC.rglob("*.java"):
        if f.name in CORE_TARGETS:
            res = analyze_file(f, CORE_TARGETS[f.name])
            if res:
                all_reports.append(res)
    
    out_file = OUTPUT_DIR / "engine_loop_ast.md"
    out_file.parent.mkdir(parents=True, exist_ok=True)
    out_file.write_text("\n---\n".join(all_reports), encoding="utf-8")
    print(f"Core engine AST → {out_file}")


def run_action_extraction(query: str):
    """Mode 2: Extract a specific action by Rust name → stdout + output/<name>_ast.md"""
    all_reports = []
    
    # 1. Find action files
    action_files = find_action_files(query)
    if not action_files:
        print(f"No Java Action file found matching '{query}'.")
        print(f"  Searched: {JAVA_SRC / 'actions'} for *{query}*Action.java")
    else:
        print(f"Found {len(action_files)} action file(s):")
        for af in action_files:
            print(f"  → {af.relative_to(JAVA_SRC)}")
            res = analyze_file(af, ["update"])
            if res:
                all_reports.append(res)
    
    # 2. Find supplementary files
    supps = find_supplementary_files(query)
    for fpath, methods in supps:
        print(f"  + {fpath.relative_to(JAVA_SRC)}  ({', '.join(methods)})")
        res = analyze_file(fpath, methods)
        if res:
            all_reports.append(res)
    
    if not all_reports:
        return
    
    combined = "\n---\n".join(all_reports)
    
    # Print to stdout
    print("\n" + "=" * 60)
    print(combined)
    
    # Save to file
    out_file = OUTPUT_DIR / f"{query.lower()}_ast.md"
    out_file.parent.mkdir(parents=True, exist_ok=True)
    out_file.write_text(combined, encoding="utf-8")
    print(f"\nSaved → {out_file}")


def main():
    if not JAVA_SRC.exists():
        print(f"Java source directory not found: {JAVA_SRC}")
        return
    
    if len(sys.argv) > 1:
        query = sys.argv[1]
        run_action_extraction(query)
    else:
        run_core_extraction()


if __name__ == "__main__":
    main()
