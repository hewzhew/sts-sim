#!/usr/bin/env python3
"""
monster_ast.py - Java AST Extractor for Slay the Spire Monsters

Uses tree-sitter to parse Slay the Spire Java classes, outputting a human-readable Markdown
report containing exactly the structured logic of HP, Damage, getMove(), and takeTurn() methods.
Strictly relies on AST traversal. Does NOT use regex replacements or logic generation.
"""

import sys
import json
from pathlib import Path
from collections import defaultdict

import tree_sitter_java as tsjava
from tree_sitter import Language, Parser

JAVA_LANGUAGE = Language(tsjava.language())
parser = Parser(JAVA_LANGUAGE)

def read_source(path: Path):
    source = path.read_bytes()
    tree = parser.parse(source)
    return source.decode("utf-8", errors="replace"), tree

def node_text(node, source: str) -> str:
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

def get_class_name(class_node, source: str) -> str:
    for child in class_node.children:
        if child.type == "identifier":
            return node_text(child, source)
    return ""

def parse_if_statement(if_node, source, extract_body_fn):
    """
    Parses an if_statement node recursively, returning a structured dict 
    representing the condition and its body.
    """
    condition_node = next((c for c in if_node.children if c.type == "parenthesized_expression"), None)
    condition_str = node_text(condition_node, source) if condition_node else "UNKNOWN"
    
    consequence = next((c for c in if_node.children if c.type == "block" or c.type == "expression_statement" or c.type == "return_statement"), None)
    alternative = next((c for c in if_node.children if c.type == "if_statement" or c.type == "block"), None) # if there's an 'else', the child is usually 'else' keyword then 'if_statement' or 'block'
    
    # We need to explicitly find the 'else' and what follows it
    alt_node = None
    found_else = False
    for c in if_node.children:
        if found_else:
            alt_node = c
            break
        if c.type == "else":
            found_else = True

    result = {
        "type": "if",
        "condition": condition_str,
        "then_body": extract_body_fn(consequence) if consequence else []
    }
    
    if alt_node:
        if alt_node.type == "if_statement":
            result["else_body"] = [parse_if_statement(alt_node, source, extract_body_fn)]
        else:
            result["else_body"] = extract_body_fn(alt_node)
            
    return result

def extract_get_move_body(block_node, source):
    """ Extracts statements within getMove() into a sequence """
    if not block_node: return []
    seq = []
    
    nodes_to_process = block_node.children if block_node.type == "block" else [block_node]
    
    for c in nodes_to_process:
        if c.type == "if_statement":
            seq.append(parse_if_statement(c, source, lambda n: extract_get_move_body(n, source)))
        elif c.type == "expression_statement":
            seq.append({
                "type": "call",
                "text": node_text(c, source).strip()
            })
        elif c.type == "return_statement":
            seq.append({
                "type": "return",
                "text": node_text(c, source).strip()
            })
    return seq

def extract_take_turn_switch(switch_node, source):
    """ Extracts the switch(this.nextMove) from takeTurn() """
    out = {}
    switch_block = next((c for c in switch_node.children if c.type == "switch_block"), None)
    if not switch_block: return out
    
    for group in find_descendants_by_type(switch_block, "switch_block_statement_group", max_depth=1):
        case_label = next((c for c in group.children if c.type == "switch_label"), None)
        case_val = node_text(case_label, source).strip() if case_label else "default"
        
        actions = []
        for expr in find_descendants_by_type(group, "expression_statement", max_depth=2):
            text = node_text(expr, source).strip()
            if "addToBottom" in text or "addToTop" in text:
                # Extract new XXXAction(...)
                action_nodes = find_descendants_by_type(expr, "object_creation_expression")
                if action_nodes:
                    inner_text = node_text(action_nodes[-1], source).replace('\n', '')
                    actions.append(inner_text)
                else:
                    actions.append(text)
        out[case_val] = actions
    return out

def format_get_move(branch, indent=""):
    out = []
    if not isinstance(branch, list):
        branch = [branch]
    for b in branch:
        if b["type"] == "if":
            out.append(f"{indent}- IF `{b['condition']}`:")
            out.extend(format_get_move(b["then_body"], indent + "  "))
            if "else_body" in b and b["else_body"]:
                out.append(f"{indent}- ELSE:")
                out.extend(format_get_move(b["else_body"], indent + "  "))
        elif b["type"] == "call":
            out.append(f"{indent}- {b['text']}")
        elif b["type"] == "return":
            out.append(f"{indent}- RETURN: {b['text']}")
    return out

def analyze_monster(fpath: Path):
    try:
        source, tree = read_source(fpath)
    except Exception as e:
        return None

    class_nodes = find_descendants_by_type(tree.root_node, "class_declaration", max_depth=3)
    if not class_nodes:
        return None
        
    class_node = class_nodes[0]
    class_name = get_class_name(class_node, source)
    
    info = {
        "class_name": class_name,
        "file": fpath.name,
        "take_turn": {},
        "get_move_tree": []
    }
    
    # Extract getMove and takeTurn exactly
    methods = find_descendants_by_type(class_node, "method_declaration", max_depth=2)
    for m in methods:
        method_name = ""
        for c in m.children:
            if c.type == "identifier":
                method_name = node_text(c, source)
                break
                
        if method_name == "getMove":
            body_node = next((c for c in m.children if c.type == "block"), None)
            info["get_move_tree"] = extract_get_move_body(body_node, source)
        elif method_name == "takeTurn":
            switch_nodes = find_descendants_by_type(m, "switch_expression", max_depth=3)
            if switch_nodes:
                info["take_turn"] = extract_take_turn_switch(switch_nodes[0], source)
                
    return info

def generate_markdown(info) -> str:
    md = [f"# Monster: {info['class_name']}", ""]
    
    md.append("## `getMove()` Intent Decision Tree")
    if info["get_move_tree"]:
        md.extend(format_get_move(info["get_move_tree"]))
    else:
        md.append("_No getMove() AST extracted._")
    
    md.append("\n## `takeTurn()` Action Outputs")
    if info["take_turn"]:
        for case_label, actions in info["take_turn"].items():
            md.append(f"### {case_label}")
            if not actions:
                md.append("- (Empty or unknown statements)")
            for act in actions:
                md.append(f"- `{act}`")
    else:
        md.append("_No standard switch block found in takeTurn()._")
        
    md.append("")
    return "\n".join(md)

def main():
    if len(sys.argv) > 1:
        fpath = Path(sys.argv[1])
        if fpath.exists():
            info = analyze_monster(fpath)
            if info:
                out = generate_markdown(info)
                print(out)
                Path("monster_analysis_output.md").write_text(out, encoding='utf-8')
                print("Wrote monster_analysis_output.md")
            else:
                print("Failed to analyze.")
        return

    # Batch process
    src_dir = Path(r"d:\rust\cardcrawl\monsters")
    java_files = list(src_dir.rglob("*.java"))
    results = []

    for fpath in java_files:
        if "monsters" in fpath.parts:
            info = analyze_monster(fpath)
            if info:
                results.append(generate_markdown(info) + "\n---\n")

    out_file = Path(r"d:\rust\sts_simulator\tools\all_monsters_analysis.md")
    out_file.write_text("\n".join(results), encoding="utf-8")
    print(f"Extracted {len(results)} Java monsters to {out_file}")

if __name__ == "__main__":
    main()
