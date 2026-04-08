#!/usr/bin/env python3
"""
Combat Trace Analyzer

Reads a compact JSONL replay file and produces a human-readable combat trace
showing every action and its resulting state change. This can be directly
compared with Rust simulator output.

Also outputs: combat_assertions.jsonl — machine-readable test cases.

Usage:
    python combat_trace.py replay_short.jsonl
"""

import json
import sys
from pathlib import Path


def format_powers(powers):
    if not powers:
        return ""
    return " [" + ", ".join(f"{p['id']}({p['amount']})" for p in powers) + "]"


def format_monster(m):
    gone = " GONE" if m.get("is_gone") else ""
    block = f" block={m['block']}" if m.get("block", 0) > 0 else ""
    powers = format_powers(m.get("powers", []))
    intent = f" intent={m['intent']}" if m.get("intent") else ""
    dmg = ""
    if m.get("move_adjusted_damage") and m["move_adjusted_damage"] > 0:
        hits = m.get("move_hits", 1)
        if hits > 1:
            dmg = f" dmg={m['move_adjusted_damage']}x{hits}"
        else:
            dmg = f" dmg={m['move_adjusted_damage']}"
    return f"{m['name']}(hp={m['hp']}/{m['max_hp']}{block}{powers}{intent}{dmg}{gone})"


def format_hand(hand):
    cards = []
    for c in hand:
        upgrade = "+" if c.get("upgrades", 0) > 0 else ""
        cards.append(f"{c['id']}{upgrade}")
    return "[" + ", ".join(cards) + "]"


def analyze_combat(events, combat_idx):
    """Analyze a single combat's events and produce trace + assertions."""
    trace_lines = []
    assertions = []

    combat_events = [e for e in events if e.get("combat_idx") == combat_idx]
    if not combat_events:
        return [], []

    start = next((e for e in combat_events if e["type"] == "combat_start"), None)
    if start:
        trace_lines.append(f"\n{'='*60}")
        trace_lines.append(f"COMBAT #{combat_idx} — Floor {start['floor']} ({start['room_type']})")
        trace_lines.append(f"  Player: HP={start['player_hp']}/{start['player_max_hp']} Gold={start['gold']}")
        relic_strs = []
        for r in start.get('relics', []):
            s = r['id']
            if r.get('counter', -1) != -1:
                s += f"[{r['counter']}]"
            relic_strs.append(s)
        trace_lines.append(f"  Relics: {', '.join(relic_strs)}")
        trace_lines.append(f"  Monsters: {', '.join(format_monster(m) for m in start['monsters'])}")

        snap = start["snapshot"]
        trace_lines.append(f"  Turn {snap['turn']}: Energy={snap['player']['energy']} Hand={format_hand(snap['hand'])}")
        trace_lines.append(f"    Draw({snap['draw_pile_size']})={snap['draw_pile_ids']}  Discard({snap['discard_pile_size']})")
        trace_lines.append(f"{'='*60}")

    prev_snapshot = start["snapshot"] if start else None
    action_idx = 0

    for event in combat_events:
        if event["type"] == "play":
            action_idx += 1
            snap = event["result"]
            card_idx = event["card_index"]
            target = event.get("target")

            # Identify what card was played (from previous hand)
            card_name = "?"
            if prev_snapshot and card_idx < len(prev_snapshot["hand"]):
                card_info = prev_snapshot["hand"][card_idx]
                card_name = card_info["id"]
                if card_info.get("upgrades", 0) > 0:
                    card_name += "+"

            target_str = f" → target[{target}]" if target is not None else ""
            trace_lines.append(f"  [{action_idx}] PLAY {card_name} (idx={card_idx}){target_str}")

            # Show state changes
            player = snap["player"]
            trace_lines.append(f"    Player: HP={player['hp']}/{player['max_hp']} Block={player['block']} Energy={player['energy']}{format_powers(player['powers'])}")
            for i, m in enumerate(snap["monsters"]):
                trace_lines.append(f"    Monster[{i}]: {format_monster(m)}")
            trace_lines.append(f"    Hand({snap['hand_size']})={format_hand(snap['hand'])}")
            trace_lines.append(f"    Draw({snap['draw_pile_size']}) Discard({snap['discard_pile_size']}) Exhaust({snap['exhaust_pile_size']})")

            # Generate assertion
            assertion = {
                "combat_idx": combat_idx,
                "action_idx": action_idx,
                "action": "play",
                "card": card_name,
                "card_index": card_idx,
                "target": target,
                "turn": snap["turn"],
                "expected": {
                    "player_hp": player["hp"],
                    "player_block": player["block"],
                    "player_energy": player["energy"],
                    "player_powers": player["powers"],
                    "monsters": [{
                        "hp": m["hp"],
                        "block": m["block"],
                        "powers": m["powers"],
                        "is_gone": m.get("is_gone", False),
                    } for m in snap["monsters"]],
                    "hand_size": snap["hand_size"],
                    "draw_pile_size": snap["draw_pile_size"],
                    "discard_pile_size": snap["discard_pile_size"],
                    "exhaust_pile_size": snap["exhaust_pile_size"],
                }
            }
            assertions.append(assertion)
            prev_snapshot = snap

        elif event["type"] == "end_turn":
            action_idx += 1
            snap = event["result"]
            trace_lines.append(f"  [{action_idx}] END TURN → Turn {snap['turn']}")

            player = snap["player"]
            trace_lines.append(f"    Player: HP={player['hp']}/{player['max_hp']} Block={player['block']} Energy={player['energy']}{format_powers(player['powers'])}")
            for i, m in enumerate(snap["monsters"]):
                trace_lines.append(f"    Monster[{i}]: {format_monster(m)}")
            trace_lines.append(f"    Hand({snap['hand_size']})={format_hand(snap['hand'])}")
            trace_lines.append(f"    Draw({snap['draw_pile_size']}) Discard({snap['discard_pile_size']}) Exhaust({snap['exhaust_pile_size']})")

            # Generate assertion
            assertion = {
                "combat_idx": combat_idx,
                "action_idx": action_idx,
                "action": "end_turn",
                "turn": snap["turn"],
                "expected": {
                    "player_hp": player["hp"],
                    "player_block": player["block"],
                    "player_energy": player["energy"],
                    "player_powers": player["powers"],
                    "monsters": [{
                        "hp": m["hp"],
                        "block": m["block"],
                        "powers": m["powers"],
                        "is_gone": m.get("is_gone", False),
                        "intent": m.get("intent"),
                        "move_id": m.get("move_id"),
                    } for m in snap["monsters"]],
                    "hand_size": snap["hand_size"],
                    "draw_pile_size": snap["draw_pile_size"],
                    "discard_pile_size": snap["discard_pile_size"],
                    "exhaust_pile_size": snap["exhaust_pile_size"],
                }
            }
            assertions.append(assertion)
            prev_snapshot = snap

        elif event["type"] == "potion":
            action_idx += 1
            snap = event["result"]
            trace_lines.append(f"  [{action_idx}] POTION: {event['command']}")
            player = snap["player"]
            trace_lines.append(f"    Player: HP={player['hp']}/{player['max_hp']} Block={player['block']} Energy={player['energy']}{format_powers(player['powers'])}")
            prev_snapshot = snap

    # Combat end
    end_event = next((e for e in events if e.get("type") == "combat_end" and e.get("combat_idx") == combat_idx), None)
    if end_event:
        trace_lines.append(f"  --- COMBAT END --- HP={end_event['player_hp']} Gold={end_event['gold']}")
        trace_lines.append(f"  Deck: {end_event['deck']}")
        trace_lines.append(f"  Relics: {end_event['relics']}")

    return trace_lines, assertions


def main():
    if len(sys.argv) < 2:
        print(f"Usage: {sys.argv[0]} <replay.jsonl>")
        sys.exit(1)

    replay_path = sys.argv[1]
    with open(replay_path, "r") as f:
        events = [json.loads(line) for line in f]

    # Print init info
    init = next((e for e in events if e["type"] == "init"), None)
    if init:
        print(f"Run: {init['class']} Asc{init['ascension']} Seed={init['seed']}")
        print(f"Starting deck: {init['deck']}")
        print(f"Starting relics: {init['relics']}")

    # Find all combat indices
    combat_indices = sorted(set(e.get("combat_idx", 0) for e in events if "combat_idx" in e))

    all_assertions = []
    for ci in combat_indices:
        trace, assertions = analyze_combat(events, ci)
        for line in trace:
            print(line)
        all_assertions.extend(assertions)

    # Summary
    print(f"\n{'='*60}")
    print(f"SUMMARY: {len(combat_indices)} combats, {len(all_assertions)} verifiable assertions")

    # Write assertions
    assertions_path = Path(replay_path).with_suffix(".assertions.jsonl")
    with open(assertions_path, "w") as f:
        for a in all_assertions:
            f.write(json.dumps(a, separators=(",", ":")) + "\n")
    print(f"Assertions written to {assertions_path}")


if __name__ == "__main__":
    main()
