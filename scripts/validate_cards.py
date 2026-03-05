#!/usr/bin/env python3
"""
Card Data Validator & Cross-Reference Index

Validates all JSON card files against the Rust schema vocabulary.
Produces:
  1. Vocabulary violations (unknown enum values)
  2. Duplicate ID warnings
  3. Cross-reference index (command → cards, card → commands)
  4. Engine command coverage matrix

Usage:
  python scripts/validate_cards.py [data/cards]
  python scripts/validate_cards.py --index          # show cross-reference index
  python scripts/validate_cards.py --engine-status  # show engine coverage
"""

import json
import os
import sys
from collections import defaultdict
from pathlib import Path

# ============================================================================
# A1. VOCABULARY TABLE — Ground truth from Rust schema.rs
# ============================================================================

VOCAB = {
    "type": {  # CardType enum
        "Attack", "Skill", "Power", "Status", "Curse",
    },
    "color": {  # CardColor enum
        "Red", "Green", "Blue", "Purple", "Colorless", "Curse",
    },
    "rarity": {  # CardRarity enum
        "Basic", "Common", "Uncommon", "Rare", "Special", "Curse",
    },
    "target_type": {  # TargetType enum (serde renames applied)
        "Self", "Enemy", "AllEnemies", "RandomEnemy",
    },
    "command_type": {  # CardCommand enum variant names
        # === Core damage/block ===
        "DealDamage",
        "DealDamageAll",
        "DealDamageRandom",
        "GainBlock",
        "StrengthMultiplier",
        # === Status/buff ===
        "ApplyStatus",
        "ApplyStatusAll",
        "GainBuff",
        "LoseBuff",
        "ApplyBuff",
        "ApplyDebuff",
        "DoubleBuff",
        "RemoveEnemyBuff",
        "ApplyPower",
        "MultiplyStatus",
        "DoubleStatus",
        # === Card draw/discard ===
        "DrawCards",
        "Draw",
        "DrawUntil",
        "DrawUntilFull",
        "DiscardCards",
        "Discard",
        # === Energy ===
        "GainEnergy",
        "DoubleEnergy",
        # === Card manipulation ===
        "AddCard",
        "ExhaustSelf",
        "ExhaustCard",
        "ExhaustCards",
        "UpgradeCards",
        "UpgradeCard",
        "ShuffleInto",
        "MoveCard",
        "PutOnTop",
        "RetainSelf",
        "InnateSelf",
        # === HP ===
        "LoseHp", "LoseHP",  # alias
        "GainHp", "GainHP",  # alias
        "Heal",
        "GainMaxHP",
        "IncreaseDamage",
        # === Special effects ===
        "PlayTopCard",
        "Discover",
        "DoubleBlock",
        "EndTurn",
        "ExtraTurn",
        "Execute",
        "Conditional",
        "MultiHit",
        "Unplayable",
        # === Orb (Defect) ===
        "ChannelOrb",
        "EvokeOrb",
        "GainFocus",
        # === Stance (Watcher) ===
        "EnterStance",
        "ExitStance",
        "Scry",
        "GainMantra",
        # === Economy ===
        "GainGold",
        "ObtainPotion",
        # === Block/cost manipulation ===
        "RemoveBlock",
        "SetCostAll",
        "SetCostRandom",
        # === Markers ===
        "Ethereal",
        "Innate",
        "Retain",
    },
    "card_location": {  # CardLocation enum
        "Hand", "DrawPile", "DiscardPile", "ExhaustPile",
    },
}

# Known parameter names for DealDamageRandom
# The correct schema uses: base, upgrade, times, times_upgrade
# Old/wrong names to flag: damage_base, damage_upgrade, hits_base, hits_upgrade
DEPRECATED_PARAMS = {
    "damage_base": "base",
    "damage_upgrade": "upgrade", 
    "hits_base": "times",
    "hits_upgrade": "times_upgrade",
}


# ============================================================================
# Scanner
# ============================================================================

class CardValidator:
    def __init__(self, data_dir: str):
        self.data_dir = Path(data_dir)
        self.cards: list[dict] = []
        self.card_files: dict[str, str] = {}  # card_id -> file_path
        self.errors: list[str] = []
        self.warnings: list[str] = []
        
        # Cross-reference indices
        self.cmd_to_cards: dict[str, set[str]] = defaultdict(set)
        self.card_to_cmds: dict[str, list[str]] = defaultdict(list)
        self.status_to_cards: dict[str, set[str]] = defaultdict(set)
        self.buff_to_cards: dict[str, set[str]] = defaultdict(set)
        self.unknown_terms: dict[str, set[str]] = defaultdict(set)  # domain -> {term: [cards]}
        
        # Duplicate tracking
        self.id_occurrences: dict[str, list[str]] = defaultdict(list)  # id -> [file_paths]
    
    def scan(self):
        """Scan all JSON files in the data directory."""
        if self.data_dir.is_file():
            self._scan_file(self.data_dir)
        else:
            for root, _, files in os.walk(self.data_dir):
                for f in sorted(files):
                    if f.endswith('.json'):
                        self._scan_file(Path(root) / f)
    
    def _scan_file(self, path: Path):
        """Scan a single JSON file."""
        rel = path.relative_to(self.data_dir.parent) if not self.data_dir.is_file() else path.name
        try:
            with open(path, 'r', encoding='utf-8') as f:
                cards = json.load(f)
        except json.JSONDecodeError as e:
            self.errors.append(f"  JSON_PARSE_ERROR in {rel}: {e}")
            return
        
        if not isinstance(cards, list):
            self.errors.append(f"  NOT_AN_ARRAY in {rel}")
            return
        
        for card in cards:
            self._validate_card(card, str(rel))
    
    def _validate_card(self, card: dict, file_path: str):
        """Validate a single card definition."""
        card_id = card.get('id', '<NO_ID>')
        
        # Track file origin
        self.id_occurrences[card_id].append(file_path)
        self.card_files[card_id] = file_path
        self.cards.append(card)
        
        # Required fields
        for field in ['id', 'type', 'logic']:
            if field not in card:
                self.errors.append(f"  MISSING_FIELD [{card_id}] in {file_path}: '{field}' is required")
        
        # Validate enum fields
        self._check_enum(card_id, 'type', card.get('type'), file_path)
        self._check_enum(card_id, 'color', card.get('color'), file_path)
        self._check_enum(card_id, 'rarity', card.get('rarity'), file_path)
        
        # Validate logic block
        logic = card.get('logic', {})
        if isinstance(logic, dict):
            self._check_enum(card_id, 'target_type', logic.get('target_type'), file_path)
            
            commands = logic.get('commands', [])
            for i, cmd in enumerate(commands):
                self._validate_command(card_id, cmd, i, file_path)
    
    def _check_enum(self, card_id: str, domain: str, value, file_path: str):
        """Check if a value is in the valid vocabulary for its domain."""
        if value is None:
            return  # Optional field
        if value not in VOCAB.get(domain, set()):
            self.errors.append(
                f"  INVALID_{domain.upper()} [{card_id}] in {file_path}: "
                f"'{value}' not in {sorted(VOCAB[domain])}"
            )
            self.unknown_terms[domain].add(f"{value} (in {card_id})")
    
    def _validate_command(self, card_id: str, cmd: dict, idx: int, file_path: str):
        """Validate a single command object."""
        cmd_type = cmd.get('type', '<NO_TYPE>')
        
        # Check command type against vocabulary
        if cmd_type not in VOCAB['command_type']:
            self.errors.append(
                f"  UNKNOWN_COMMAND [{card_id}] cmd[{idx}] in {file_path}: "
                f"'{cmd_type}'"
            )
            self.unknown_terms['command_type'].add(f"{cmd_type} (in {card_id})")
        
        # Build cross-reference
        self.cmd_to_cards[cmd_type].add(card_id)
        self.card_to_cmds[card_id].append(cmd_type)
        
        # Track status/buff references
        params = cmd.get('params', {})
        if isinstance(params, dict):
            if 'status' in params:
                self.status_to_cards[params['status']].add(card_id)
            if 'buff' in params:
                self.buff_to_cards[params['buff']].add(card_id)
            
            # Check for deprecated parameter names
            for old_name, new_name in DEPRECATED_PARAMS.items():
                if old_name in params:
                    self.warnings.append(
                        f"  DEPRECATED_PARAM [{card_id}] cmd[{idx}] in {file_path}: "
                        f"'{old_name}' should be '{new_name}'"
                    )
    
    def check_duplicates(self):
        """Report duplicate card IDs."""
        for card_id, files in self.id_occurrences.items():
            if len(files) > 1:
                # Duplicates within same file are worse than across files
                if len(set(files)) == 1:
                    self.errors.append(
                        f"  DUPLICATE_ID [{card_id}] appears {len(files)}x in {files[0]}"
                    )
                else:
                    self.warnings.append(
                        f"  DUPLICATE_ID [{card_id}] in {len(files)} files: {files}"
                    )
    
    # ========================================================================
    # Reporting
    # ========================================================================
    
    def report_validation(self):
        """Print validation results."""
        total_cards = len(self.cards)
        unique_ids = len(self.id_occurrences)
        
        print(f"\n{'='*60}")
        print(f"Card Validation Report")
        print(f"{'='*60}")
        print(f"  Total entries: {total_cards}")
        print(f"  Unique IDs:    {unique_ids}")
        print(f"  Errors:        {len(self.errors)}")
        print(f"  Warnings:      {len(self.warnings)}")
        
        if self.errors:
            print(f"\n{'─'*60}")
            print("ERRORS:")
            for e in sorted(self.errors):
                print(e)
        
        if self.warnings:
            print(f"\n{'─'*60}")
            print("WARNINGS:")
            for w in sorted(self.warnings):
                print(w)
        
        if not self.errors and not self.warnings:
            print("\n  ✅ All cards valid!")
        
        print()
        return len(self.errors) == 0
    
    def report_index(self):
        """Print cross-reference index."""
        print(f"\n{'='*60}")
        print("Command → Cards Index")
        print(f"{'='*60}")
        for cmd in sorted(self.cmd_to_cards):
            cards = sorted(self.cmd_to_cards[cmd])
            marker = "✅" if cmd in VOCAB['command_type'] else "❌"
            print(f"  {marker} {cmd:25s} ({len(cards):2d} cards): {', '.join(cards[:10])}"
                  + (f" +{len(cards)-10} more" if len(cards) > 10 else ""))
        
        if self.status_to_cards:
            print(f"\n{'─'*60}")
            print("Status/Debuff → Cards:")
            for status in sorted(self.status_to_cards):
                cards = sorted(self.status_to_cards[status])
                print(f"  {status:20s} → {', '.join(cards)}")
        
        if self.buff_to_cards:
            print(f"\n{'─'*60}")
            print("Buff → Cards:")
            for buff in sorted(self.buff_to_cards):
                cards = sorted(self.buff_to_cards[buff])
                print(f"  {buff:20s} → {', '.join(cards)}")
        
        if self.unknown_terms:
            print(f"\n{'─'*60}")
            print("⚠️  Unknown Terms (not in vocabulary):")
            for domain, terms in sorted(self.unknown_terms.items()):
                for term in sorted(terms):
                    print(f"  [{domain}] {term}")
        print()
    
    def report_engine_status(self):
        """Print engine command coverage matrix."""
        # Check which commands have engine handlers
        engine_path = Path(self.data_dir)
        # Walk up to find src/engine.rs
        project_root = self.data_dir
        while project_root.parent != project_root:
            if (project_root / 'src' / 'engine.rs').exists():
                break
            project_root = project_root.parent
        
        engine_file = project_root / 'src' / 'engine.rs'
        engine_handlers = set()
        if engine_file.exists():
            content = engine_file.read_text(encoding='utf-8')
            for cmd in VOCAB['command_type']:
                # Check for handler patterns like "CardCommand::DealDamage" 
                if f"CardCommand::{cmd}" in content:
                    engine_handlers.add(cmd)
        
        print(f"\n{'='*60}")
        print("Engine Command Coverage")
        print(f"{'='*60}")
        print(f"  {'Command':<25s} {'Schema':>6s} {'Engine':>8s} {'Used by'}")
        print(f"  {'─'*25} {'─'*6} {'─'*8} {'─'*30}")
        
        all_cmds = sorted(VOCAB['command_type'] | set(self.cmd_to_cards.keys()))
        implemented = 0
        total_used = 0
        
        for cmd in all_cmds:
            in_schema = "✅" if cmd in VOCAB['command_type'] else "❌"
            in_engine = "✅" if cmd in engine_handlers else "  ❌"
            cards = sorted(self.cmd_to_cards.get(cmd, set()))
            used_by = ', '.join(cards[:5]) + (f" +{len(cards)-5}" if len(cards) > 5 else "")
            
            if cards:  # Only show commands used by cards
                total_used += 1
                if cmd in engine_handlers:
                    implemented += 1
            
            # Skip commands not used by any card and in schema (noise reduction)
            if not cards and cmd in VOCAB['command_type']:
                continue
            
            print(f"  {cmd:<25s} {in_schema:>6s} {in_engine:>8s}   {used_by}")
        
        print(f"\n  Coverage: {implemented}/{total_used} used commands have engine handlers")
        print()
    
    def generate_engine_status_md(self, output_path: str):
        """Generate ENGINE_STATUS.md file."""
        engine_path = Path(self.data_dir)
        project_root = self.data_dir
        while project_root.parent != project_root:
            if (project_root / 'src' / 'engine.rs').exists():
                break
            project_root = project_root.parent
        
        engine_file = project_root / 'src' / 'engine.rs'
        engine_handlers = set()
        if engine_file.exists():
            content = engine_file.read_text(encoding='utf-8')
            for cmd in VOCAB['command_type']:
                if f"CardCommand::{cmd}" in content:
                    engine_handlers.add(cmd)
        
        lines = [
            "# Engine Command Coverage\n",
            "",
            "Auto-generated by `scripts/validate_cards.py --engine-status`\n",
            "",
            "| Command | Schema | Engine | Used by Cards |",
            "|---------|--------|--------|---------------|",
        ]
        
        all_cmds = sorted(VOCAB['command_type'] | set(self.cmd_to_cards.keys()))
        for cmd in all_cmds:
            cards = sorted(self.cmd_to_cards.get(cmd, set()))
            if not cards and cmd in VOCAB['command_type'] and cmd not in engine_handlers:
                continue  # Skip unused schema-only commands
            
            in_schema = "✅" if cmd in VOCAB['command_type'] else "❌"
            in_engine = "✅" if cmd in engine_handlers else "❌"
            used_by = ', '.join(cards[:8]) + (f" +{len(cards)-8}" if len(cards) > 8 else "")
            lines.append(f"| {cmd} | {in_schema} | {in_engine} | {used_by} |")
        
        with open(output_path, 'w', encoding='utf-8') as f:
            f.write('\n'.join(lines) + '\n')
        
        print(f"  Written to {output_path}")


# ============================================================================
# Main
# ============================================================================

def main():
    import argparse
    parser = argparse.ArgumentParser(description='Card Data Validator')
    parser.add_argument('data_dir', nargs='?', default='data/cards',
                        help='Path to cards directory or single JSON file')
    parser.add_argument('--index', action='store_true',
                        help='Show cross-reference index')
    parser.add_argument('--engine-status', action='store_true',
                        help='Show engine command coverage')
    parser.add_argument('--generate-md', action='store_true',
                        help='Generate ENGINE_STATUS.md')
    parser.add_argument('--all', action='store_true',
                        help='Show all reports')
    args = parser.parse_args()
    
    validator = CardValidator(args.data_dir)
    validator.scan()
    validator.check_duplicates()
    
    # Always show validation
    ok = validator.report_validation()
    
    if args.index or args.all:
        validator.report_index()
    
    if args.engine_status or args.all:
        validator.report_engine_status()
    
    if args.generate_md:
        validator.generate_engine_status_md('ENGINE_STATUS.md')
    
    sys.exit(0 if ok else 1)


if __name__ == '__main__':
    main()
