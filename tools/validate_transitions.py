#!/usr/bin/env python3
"""
State Transition Validator

For each combat action in a JSONL replay, validates that the state
transition follows known game rules. Reports discrepancies.

Usage:
    python validate_transitions.py replay_short.jsonl
"""

import json
import sys
from collections import defaultdict


def get_power(powers, power_id):
    """Get power amount, 0 if not present."""
    for p in powers:
        if p["id"] == power_id:
            return p["amount"]
    return 0


def validate_play_card(prev, curr, card_id, target, card_cost, findings):
    """Validate state transition after playing a card."""
    pp, cp = prev["player"], curr["player"]
    tag = f"turn{prev['turn']}"

    # Energy cost
    expected_energy = pp["energy"] - card_cost
    if cp["energy"] != expected_energy:
        # Could be free play (Dropkick on vulnerable), or 0-cost card
        if card_id == "Dropkick" and target is not None:
            # Dropkick gives 1 energy + 1 draw if target vulnerable
            pm = prev["monsters"]
            if target < len(pm) and get_power(pm[target].get("powers", []), "Vulnerable") > 0:
                expected_energy = pp["energy"]  # 0-cost refund + 1 energy
                if cp["energy"] == expected_energy:
                    findings.append(("OK", f"{tag}: Dropkick energy refund on Vulnerable target"))
                else:
                    findings.append(("INFO", f"{tag}: Dropkick energy={cp['energy']} (expected ~{expected_energy}, was {pp['energy']})"))
            else:
                if cp["energy"] != expected_energy:
                    findings.append(("WARN", f"{tag}: {card_id} energy {pp['energy']}→{cp['energy']} (expected {expected_energy})"))
        elif card_id in ("Dazed", "Burn", "Wound", "Slimed"):
            # Status cards: ethereal, unplayable, or cost varies
            pass
        else:
            findings.append(("WARN", f"{tag}: {card_id} energy {pp['energy']}→{cp['energy']} (expected {expected_energy})"))

    # Hand size should decrease by 1
    expected_hand = prev["hand_size"] - 1
    if curr["hand_size"] != expected_hand:
        delta = curr["hand_size"] - expected_hand
        if card_id == "Dropkick":
            findings.append(("INFO", f"{tag}: Dropkick hand_size delta={delta:+d} (draw effect)"))
        elif card_id == "Pommel Strike":
            findings.append(("INFO", f"{tag}: Pommel Strike hand_size delta={delta:+d} (draw effect)"))
        elif card_id == "Shrug It Off":
            findings.append(("INFO", f"{tag}: Shrug It Off hand_size delta={delta:+d} (draw effect)"))
        else:
            findings.append(("WARN", f"{tag}: {card_id} hand_size {prev['hand_size']}→{curr['hand_size']} (expected {expected_hand})"))

    # Block changes
    if card_id == "Defend_R":
        base_block = 5
        dex = get_power(pp.get("powers", []), "Dexterity")
        expected_block = pp["block"] + base_block + dex
        if cp["block"] != expected_block:
            findings.append(("FAIL", f"{tag}: Defend block {pp['block']}→{cp['block']} (expected {expected_block}, dex={dex})"))
        else:
            findings.append(("OK", f"{tag}: Defend block correct ({expected_block})"))

    elif card_id == "Shrug It Off":
        base_block = 8
        dex = get_power(pp.get("powers", []), "Dexterity")
        expected_block = pp["block"] + base_block + dex
        if cp["block"] != expected_block:
            findings.append(("FAIL", f"{tag}: Shrug It Off block {pp['block']}→{cp['block']} (expected {expected_block})"))

    # Damage calculations for attack cards
    if target is not None and card_id in ("Strike_R", "Bash", "Perfected Strike", "Clothesline", "Pommel Strike"):
        pm_prev = prev["monsters"][target] if target < len(prev["monsters"]) else None
        pm_curr = curr["monsters"][target] if target < len(curr["monsters"]) else None
        if pm_prev and pm_curr and not pm_prev.get("is_gone"):
            vuln = get_power(pm_prev.get("powers", []), "Vulnerable")
            weak = get_power(pp.get("powers", []), "Weakened")
            strength = get_power(pp.get("powers", []), "Strength")

            # Base damages
            base_dmg = {
                "Strike_R": 6, "Bash": 8, "Pommel Strike": 9,
                "Clothesline": 12,
            }.get(card_id)

            if card_id == "Perfected Strike":
                # Count strikes in deck - we don't have full deck info here, approximate
                base_dmg = None  # Skip exact check

            if base_dmg is not None:
                dmg = base_dmg + strength
                if weak:
                    dmg = int(dmg * 0.75)
                if vuln:
                    dmg = int(dmg * 1.5)

                # Account for block
                block = pm_prev.get("block", 0)
                expected_hp = max(0, pm_prev["hp"] - max(0, dmg - block))
                expected_block_after = max(0, block - dmg)

                if pm_curr["hp"] != expected_hp:
                    # Check if Curl Up triggered (Louse gets block on first hit)
                    curl_up = get_power(pm_prev.get("powers", []), "Curl Up")
                    if curl_up > 0:
                        # Curl Up adds block before damage lands
                        effective_block = block + curl_up
                        expected_hp_curl = pm_prev["hp"] - max(0, dmg - effective_block)
                        if pm_curr["hp"] == expected_hp_curl:
                            findings.append(("OK", f"{tag}: {card_id} damage with Curl Up correct (hp={expected_hp_curl})"))
                            return
                    findings.append(("FAIL", f"{tag}: {card_id}→monster[{target}] hp {pm_prev['hp']}→{pm_curr['hp']} "
                                   f"(expected {expected_hp}, base={base_dmg} str={strength} weak={weak} vuln={vuln} block={block})"))
                else:
                    findings.append(("OK", f"{tag}: {card_id} damage correct (hp {pm_prev['hp']}→{pm_curr['hp']})"))

    # Bash applies Vulnerable
    if card_id == "Bash" and target is not None:
        pm_curr = curr["monsters"][target] if target < len(curr["monsters"]) else None
        if pm_curr and not pm_curr.get("is_gone"):
            vuln_after = get_power(pm_curr.get("powers", []), "Vulnerable")
            if vuln_after < 2:
                pm_prev_t = prev["monsters"][target] if target < len(prev["monsters"]) else {}
                art = get_power(pm_prev_t.get("powers", []), "Artifact")
                if art > 0:
                    findings.append(("OK", f"{tag}: Bash Vulnerable blocked by Artifact"))
                else:
                    findings.append(("FAIL", f"{tag}: Bash should apply Vulnerable(2), got {vuln_after}"))

    # Clothesline applies Weak
    if card_id == "Clothesline" and target is not None:
        pm_curr = curr["monsters"][target] if target < len(curr["monsters"]) else None
        if pm_curr and not pm_curr.get("is_gone"):
            weak_after = get_power(pm_curr.get("powers", []), "Weakened")
            if weak_after < 2:
                # Could be Artifact consuming it
                art = get_power((prev["monsters"][target] if target < len(prev["monsters"]) else {}).get("powers", []), "Artifact")
                if art > 0:
                    findings.append(("OK", f"{tag}: Clothesline Weak blocked by Artifact"))
                else:
                    findings.append(("FAIL", f"{tag}: Clothesline should apply Weak(2), got {weak_after}"))


def validate_end_turn(prev, curr, findings):
    """Validate state transition after ending turn."""
    pp, cp = prev["player"], curr["player"]
    tag = f"turn{prev['turn']}→{curr['turn']}"

    # Energy should reset to 3 (base)
    if cp["energy"] != 3:
        findings.append(("WARN", f"{tag}: energy after end_turn = {cp['energy']} (expected 3)"))

    # Player block should be 0 at start of new turn (unless Barricade)
    if cp["block"] != 0:
        findings.append(("WARN", f"{tag}: player block = {cp['block']} (expected 0 at turn start)"))

    # Monster damage to player
    for i, m_prev in enumerate(prev["monsters"]):
        if m_prev.get("is_gone"):
            continue
        intent = m_prev.get("intent", "")
        if "ATTACK" in intent:
            weak = get_power(m_prev.get("powers", []), "Weakened")
            base_dmg = m_prev.get("move_adjusted_damage", m_prev.get("move_base_damage", 0))
            hits = m_prev.get("move_hits", 1)
            if base_dmg and base_dmg > 0:
                # move_adjusted_damage already accounts for strength+weak in Java
                total_dmg = base_dmg * hits
                block_absorbed = min(pp["block"], total_dmg)
                expected_hp = pp["hp"] - (total_dmg - block_absorbed)
                # This is approximate since multiple monsters can attack
                # and order matters for block consumption
                findings.append(("INFO", f"{tag}: {m_prev['name']} attacks for {base_dmg}x{hits}={total_dmg}, "
                               f"block={pp['block']}, expected HP≈{expected_hp}, actual={cp['hp']}"))

    # Monster Vulnerable/Weak should decrement
    for i, m_prev in enumerate(prev["monsters"]):
        if m_prev.get("is_gone"):
            continue
        m_curr = curr["monsters"][i] if i < len(curr["monsters"]) else None
        if m_curr and not m_curr.get("is_gone"):
            vuln_prev = get_power(m_prev.get("powers", []), "Vulnerable")
            vuln_curr = get_power(m_curr.get("powers", []), "Vulnerable")
            if vuln_prev > 0:
                expected_vuln = vuln_prev - 1
                if vuln_curr != expected_vuln:
                    findings.append(("WARN", f"{tag}: monster[{i}] Vuln {vuln_prev}→{vuln_curr} (expected {expected_vuln})"))

    # Burning Blood heals at end of combat (tracked at combat_end level, skip here)


def main():
    if len(sys.argv) < 2:
        print(f"Usage: {sys.argv[0]} <replay.jsonl>")
        sys.exit(1)

    with open(sys.argv[1]) as f:
        events = [json.loads(line) for line in f]

    # Group by combat
    combats = defaultdict(list)
    for e in events:
        if "combat_idx" in e:
            combats[e["combat_idx"]].append(e)

    stats = {"OK": 0, "INFO": 0, "WARN": 0, "FAIL": 0}

    for ci in sorted(combats):
        combat_events = combats[ci]
        start = next((e for e in combat_events if e["type"] == "combat_start"), None)
        if not start:
            continue

        print(f"\n--- Combat #{ci} Floor {start['floor']} vs {', '.join(m['name'] for m in start['monsters'])} ---")
        findings = []
        prev_snapshot = start["snapshot"]

        for event in combat_events:
            if event["type"] == "play":
                # Resolve card name from prev hand
                card_idx = event["card_index"]
                card_id = "?"
                card_cost = 1
                if card_idx < len(prev_snapshot["hand"]):
                    card_info = prev_snapshot["hand"][card_idx]
                    card_id = card_info["id"]
                    card_cost = card_info["cost"]

                validate_play_card(prev_snapshot, event["result"], card_id,
                                  event.get("target"), card_cost, findings)
                prev_snapshot = event["result"]

            elif event["type"] == "end_turn":
                validate_end_turn(prev_snapshot, event["result"], findings)
                prev_snapshot = event["result"]

            elif event["type"] == "potion":
                prev_snapshot = event["result"]

        # Print findings
        for level, msg in findings:
            stats[level] += 1
            if level in ("FAIL", "WARN"):
                print(f"  [{level}] combat#{ci} {msg}")
            elif level == "OK" and "--verbose" in sys.argv:
                print(f"  [{level}] {msg}")

    print(f"\n{'='*50}")
    print(f"SUMMARY: {stats['OK']} OK, {stats['INFO']} INFO, {stats['WARN']} WARN, {stats['FAIL']} FAIL")
    if stats["FAIL"] > 0:
        print("⚠️  FAIL items indicate potential game rule violations")


if __name__ == "__main__":
    main()
