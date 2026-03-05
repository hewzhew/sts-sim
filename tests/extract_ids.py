"""
Extract all monster IDs from Java source, current JSON, and Rust hardcoded AI.
Compare naming conventions across all three systems.
"""
import os, re, json

# 1. Java IDs
java_dir = r"C:\Dev\rust\cardcrawl\monsters"
java_monsters = {}
for root, dirs, files in os.walk(java_dir):
    for fname in sorted(files):
        if not fname.endswith(".java") or fname.startswith("Abstract") or fname.startswith("Monster"):
            continue
        fpath = os.path.join(root, fname)
        with open(fpath, "r", encoding="utf-8") as f:
            content = f.read()
        id_match = re.search(r'public static final String ID = "([^"]+)"', content)
        if id_match:
            java_id = id_match.group(1)
            # Get act from directory
            rel = os.path.relpath(root, java_dir)
            act_map = {"exordium": "Act1", "city": "Act2", "beyond": "Act3", "ending": "Act4"}
            act = act_map.get(rel, rel)
            java_monsters[java_id] = {"file": fname, "act": act}

# 2. JSON IDs  
with open("data/monsters_with_behavior.json", "r", encoding="utf-8") as f:
    data = json.load(f)
json_monsters = {}
for mon in data["monsters"]:
    json_monsters[mon["name"]] = {
        "internal_id": mon.get("internal_id", ""),
        "act": mon.get("act", ""),
    }

# 3. Rust hardcoded IDs (from hardcoded_get_move match arms)
with open("src/monsters/hardcoded_ai.rs", "r", encoding="utf-8") as f:
    rust_content = f.read()
rust_ids = set(re.findall(r'"([^"]+)"\s*(?:\|[^=]*)?=>\s*Some\(self\.', rust_content))

# 4. Compare
lines = []
lines.append("=" * 100)
lines.append(f"{'Java ID':<25} {'JSON name':<25} {'JSON internal_id':<20} {'In Rust HC?':<12} {'Act':<6} {'Issues'}")
lines.append("=" * 100)

# Match Java IDs to JSON
all_java_ids = sorted(java_monsters.keys())
matched = 0
issues_count = 0

for jid in all_java_ids:
    jinfo = java_monsters[jid]
    
    # Find in JSON - try by internal_id first, then by name
    json_match = None
    json_name = ""
    json_iid = ""
    for jname, jdata in json_monsters.items():
        if jdata["internal_id"] == jid or jname == jid:
            json_match = jdata
            json_name = jname
            json_iid = jdata["internal_id"]
            break
    
    # Check if space-separated version matches
    if not json_match:
        # Try converting CamelCase to space-separated
        spaced = re.sub(r'([a-z])([A-Z])', r'\1 \2', jid)
        spaced = spaced.replace("_", " ")
        for jname, jdata in json_monsters.items():
            if jname == spaced:
                json_match = jdata
                json_name = jname
                json_iid = jdata["internal_id"]
                break
    
    in_rust = "YES" if jid in rust_ids else ""
    if not in_rust:
        # Try other forms
        for rid in rust_ids:
            if rid.replace(" ", "") == jid or rid == jid:
                in_rust = "YES"
                break
    
    issues = []
    if not json_match:
        issues.append("NO_JSON")
    else:
        matched += 1
        if json_name != jid and json_iid != jid:
            issues.append(f"ID_MISMATCH")
        if " " in json_name and json_iid != jid:
            issues.append("SPACE_IN_NAME")
    
    if issues:
        issues_count += 1
    
    issue_str = ", ".join(issues) if issues else "OK"
    lines.append(f"{jid:<25} {json_name:<25} {json_iid:<20} {in_rust:<12} {jinfo['act']:<6} {issue_str}")

lines.append("")
lines.append(f"Total Java monsters: {len(java_monsters)}")
lines.append(f"Matched in JSON: {matched}")
lines.append(f"With issues: {issues_count}")
lines.append(f"Rust hardcoded: {len(rust_ids)}")
lines.append("")

# Also list naming patterns
lines.append("=== Java ID Naming Patterns ===")
patterns = {"CamelCase": [], "With_Underscore": [], "With Space": [], "Mixed": []}
for jid in all_java_ids:
    if "_" in jid:
        patterns["With_Underscore"].append(jid)
    elif " " in jid:
        patterns["With Space"].append(jid)
    elif jid[0].isupper() and any(c.islower() for c in jid):
        patterns["CamelCase"].append(jid)
    else:
        patterns["Mixed"].append(jid)

for pname, plist in patterns.items():
    lines.append(f"\n{pname} ({len(plist)}):")
    for item in plist:
        lines.append(f"  {item}")

# Print Java IDs that look messy
lines.append("\n=== Java IDs with potential naming issues ===")
for jid in all_java_ids:
    problems = []
    if "_" in jid and any(c.isupper() for c in jid.split("_")[0]):
        problems.append("mixed_Case_Underscore")
    if jid != jid.strip():
        problems.append("whitespace")
    if any(c.isdigit() for c in jid):
        pass  # digits are OK (AcidSlime_L)
    
    # Check if the ID is confusingly different from the display name
    # (e.g., "SlaverBlue" for "Blue Slaver")
    for jname in json_monsters:
        jiid = json_monsters[jname].get("internal_id", "")
        if jiid == jid and jname != jid:
            if jname.split()[0] != jid[:len(jname.split()[0])]:
                problems.append(f"confusing: display='{jname}'")
    
    if problems:
        lines.append(f"  {jid}: {', '.join(problems)}")

with open("tests/id_comparison.txt", "w", encoding="utf-8") as f:
    f.write("\n".join(lines))

print(f"Done. {len(java_monsters)} Java, {len(json_monsters)} JSON, {len(rust_ids)} Rust HC. See tests/id_comparison.txt")
