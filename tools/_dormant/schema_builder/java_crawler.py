#!/usr/bin/env python3
import sys
import json
from pathlib import Path

try:
    import tree_sitter_java as tsjava
    from tree_sitter import Language, Parser
except ImportError:
    print("Error: Missing tree-sitter dependencies. Run: pip install tree-sitter tree-sitter-java")
    sys.exit(1)

JAVA_LANGUAGE = Language(tsjava.language())
parser = Parser(JAVA_LANGUAGE)

MONITORED_BASE_CLASSES = {
    "AbstractRelic": "relic",
    "AbstractMonster": "monster",
    "AbstractCard": "card",
    "AbstractPower": "power",
    "AbstractPotion": "potion"
}

def node_text(node, source: bytes) -> str:
    return source[node.start_byte:node.end_byte].decode("utf-8", errors="replace")

def find_descendants_by_type(node, type_name: str):
    results = []
    for c in node.children:
        if c.type == type_name:
            results.append(c)
        results.extend(find_descendants_by_type(c, type_name))
    return results

def get_class_name_and_super_class(class_node, source: bytes):
    class_name = None
    super_class = None
    
    for c in class_node.children:
        if c.type == "identifier":
            class_name = node_text(c, source)
        elif c.type == "superclass":
            # superclass usually has type_identifier child
            for sc in c.children:
                if sc.type == "type_identifier":
                    super_class = node_text(sc, source)
                    break
    return class_name, super_class

def extract_id_field(class_node, source: bytes):
    """ Look for public static final String ID = '...'; """
    field_decls = find_descendants_by_type(class_node, "field_declaration")
    for field in field_decls:
        text = node_text(field, source)
        if "String ID" in text and "=" in text:
            # Try to grab the string literal
            string_literals = find_descendants_by_type(field, "string_literal")
            if string_literals:
                # The text usually includes the quotes, we should strip them
                lit_text = node_text(string_literals[0], source)
                # handle "ID" vs ID
                if lit_text.startswith('"') and lit_text.endswith('"'):
                    return lit_text[1:-1]
    return None

def main():
    if len(sys.argv) < 3:
        print("Usage: python java_crawler.py <java_src_dir> <output.json>")
        sys.exit(1)
        
    src_dir = Path(sys.argv[1])
    out_file = Path(sys.argv[2])
    
    if not src_dir.exists() or not src_dir.is_dir():
        print(f"Error: {src_dir} does not exist.")
        sys.exit(1)

    entities = { category: {} for category in MONITORED_BASE_CLASSES.values() }
    
    for fpath in src_dir.rglob("*.java"):
        source = fpath.read_bytes()
        tree = parser.parse(source)
        
        class_nodes = find_descendants_by_type(tree.root_node, "class_declaration")
        for c_node in class_nodes:
            class_name, super_class = get_class_name_and_super_class(c_node, source)
            
            # If we don't have a known superclass, we might check if its package or name implies it.
            # For this MVP, we strictly look for extensions of our base classes.
            if super_class in MONITORED_BASE_CLASSES:
                category = MONITORED_BASE_CLASSES[super_class]
                entity_id = extract_id_field(c_node, source)
                
                # If ID wasn't declared directly, sometimes it uses another constant or inherits (rare for STS core).
                # We'll record it anyway, with fallback to class name if ID is missing.
                if entity_id is None:
                    entity_id = class_name
                
                # Store it
                if entity_id not in entities[category]:
                    entities[category][entity_id] = {
                        "class_name": class_name,
                        "file_path": str(fpath.relative_to(src_dir).as_posix())
                    }

    out_file.write_text(json.dumps(entities, indent=2), encoding="utf-8")
    print(f"Extraction complete! Found {sum(len(v) for v in entities.values())} entities. Wrote to {out_file}")

if __name__ == '__main__':
    main()
