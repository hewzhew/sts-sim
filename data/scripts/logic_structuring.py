#!/usr/bin/env python3
"""
Phase 7.3 Step 3: Logic Structuring
Transform semi-cleaned event JSON into strict logic schema for Rust game engine.

This script:
1. Extracts structured requirements from conditions
2. Separates costs and rewards from descriptions
3. Converts Ascension values to value/value_ascension fields
4. Validates output schema
"""

import json
import re
import os
from pathlib import Path
from typing import Dict, List, Any, Optional, Tuple

# Base directory
SCRIPT_DIR = Path(__file__).parent
DATA_DIR = SCRIPT_DIR.parent / "events_preprocessed"
OUTPUT_DIR = SCRIPT_DIR.parent / "events_structured"


class LogicStructurer:
    """Transform event data into game-engine-ready format."""

    # Patterns for extracting Ascension values
    ASCENSION_PATTERNS = [
        # Pattern: "X (A15+: Y)" or "X (A15: Y)"
        (r'(\d+)\s*\(A15\+?:\s*(\d+)\)', lambda m: (int(m.group(1)), int(m.group(2)))),
        # Pattern: "X% (A15+: Y%)" 
        (r'(\d+)%\s*\(A15\+?:\s*(\d+)%\)', lambda m: (int(m.group(1)), int(m.group(2)))),
        # Pattern: just a number
        (r'^(\d+)$', lambda m: (int(m.group(1)), None)),
    ]

    # Cost keywords and their types
    COST_KEYWORDS = {
        'lose': ['hp', 'gold', 'max hp', 'max_hp'],
        'take': ['damage'],
        'remove': ['card'],
        'sacrifice': ['hp'],
    }

    # Reward keywords and their types
    REWARD_KEYWORDS = {
        'gain': ['gold', 'hp', 'max hp', 'max_hp', 'strength', 'dexterity'],
        'obtain': ['relic', 'card', 'potion'],
        'receive': ['relic', 'card', 'potion', 'gold'],
        'upgrade': ['card'],
        'heal': ['hp'],
        'get': ['card', 'potion', 'relic'],
    }

    # Condition patterns for structured requirements
    CONDITION_PATTERNS = [
        # Relic requirements
        (r'[Rr]equires?:?\s*([\w\s]+)(?:\s+relic)?', 'has_relic'),
        (r'[Ii]f (?:the )?player (?:does not )?has?\s*([\w\s]+)', 'has_relic'),
        (r'[Hh]ave\s*([\w\s]+)(?:\s+relic)?', 'has_relic'),
        
        # Gold requirements
        (r'[Rr]equires?\s*(\d+)\s*[Gg]old', 'min_gold'),
        (r'(?:at least|minimum)\s*(\d+)\s*[Gg]old', 'min_gold'),
        
        # HP requirements
        (r'[Rr]equires?\s*(\d+)\s*HP', 'min_hp'),
        (r'(?:at least|minimum)\s*(\d+)\s*HP', 'min_hp'),
        
        # Card requirements
        (r'[Hh]ave (?:a |an )?([\w\s]+) card', 'has_card_type'),
        (r'[Rr]equires?:?\s*(?:a |an )?([\w\s]+) card', 'has_card_type'),
    ]

    def __init__(self):
        self.stats = {
            'events_processed': 0,
            'options_processed': 0,
            'ascension_values_extracted': 0,
            'costs_extracted': 0,
            'rewards_extracted': 0,
            'requirements_structured': 0,
        }

    def extract_ascension_value(self, text: str) -> Tuple[Optional[int], Optional[int], str]:
        """
        Extract base and A15+ values from text.
        Returns (base_value, ascension_value, remaining_text)
        """
        for pattern, extractor in self.ASCENSION_PATTERNS:
            match = re.search(pattern, text)
            if match:
                try:
                    base, asc = extractor(match)
                    remaining = re.sub(pattern, '{VALUE}', text, count=1)
                    self.stats['ascension_values_extracted'] += 1
                    return base, asc, remaining
                except (ValueError, IndexError):
                    continue
        return None, None, text

    def parse_cost_from_description(self, desc: str) -> Dict[str, Any]:
        """Extract structured costs from description text."""
        costs = {}
        
        # HP loss patterns
        hp_patterns = [
            r'[Ll]ose\s*(\d+)\s*(?:\(A15\+?:\s*(\d+)\))?\s*HP',
            r'[Ll]ose\s*(\d+)%\s*(?:\(A15\+?:\s*(\d+)%\))?\s*(?:of )?(?:your |Max )?HP',
            r'[Tt]ake\s*(\d+)\s*(?:\(A15\+?:\s*(\d+)\))?\s*damage',
        ]
        
        for pattern in hp_patterns:
            match = re.search(pattern, desc)
            if match:
                base = int(match.group(1))
                asc = int(match.group(2)) if match.group(2) else None
                if '%' in pattern or '%' in desc[match.start():match.end()]:
                    costs['hp_percent'] = base
                    if asc:
                        costs['hp_percent_ascension'] = asc
                else:
                    costs['hp'] = base
                    if asc:
                        costs['hp_ascension'] = asc
                self.stats['costs_extracted'] += 1
                break
        
        # Gold loss patterns
        gold_patterns = [
            r'[Ll]ose\s*(\d+)\s*(?:\(A15\+?:\s*(\d+)\))?\s*[Gg]old',
            r'[Pp]ay\s*(\d+)\s*(?:\(A15\+?:\s*(\d+)\))?\s*[Gg]old',
        ]
        
        for pattern in gold_patterns:
            match = re.search(pattern, desc)
            if match:
                base = int(match.group(1))
                asc = int(match.group(2)) if match.group(2) else None
                costs['gold'] = base
                if asc:
                    costs['gold_ascension'] = asc
                self.stats['costs_extracted'] += 1
                break
        
        # Lose all gold
        if re.search(r'[Ll]ose\s+ALL\s+(?:of\s+)?(?:your\s+)?[Gg]old', desc):
            costs['gold'] = 'all'
            self.stats['costs_extracted'] += 1
        
        # Max HP loss
        max_hp_match = re.search(r'[Ll]ose\s*(\d+)\s*(?:\(A15\+?:\s*(\d+)\))?\s*[Mm]ax\s*HP', desc)
        if max_hp_match:
            costs['max_hp'] = int(max_hp_match.group(1))
            if max_hp_match.group(2):
                costs['max_hp_ascension'] = int(max_hp_match.group(2))
            self.stats['costs_extracted'] += 1
        
        # Card removal
        if re.search(r'[Rr]emove\s+(?:a\s+)?card', desc):
            costs['remove_card'] = 1
            self.stats['costs_extracted'] += 1
        
        # Relic loss
        relic_match = re.search(r'[Ll]ose\s+([\w\s]+?)(?:\s+relic)?\.', desc, re.IGNORECASE)
        if relic_match and 'HP' not in relic_match.group(1) and 'Gold' not in relic_match.group(1):
            relic_name = relic_match.group(1).strip()
            if len(relic_name) > 2:  # Avoid false positives
                costs['relic'] = relic_name
                self.stats['costs_extracted'] += 1
        
        return costs

    def parse_reward_from_description(self, desc: str) -> Dict[str, Any]:
        """Extract structured rewards from description text."""
        rewards = {}
        
        # Gold gain patterns
        gold_patterns = [
            r'[Gg]ain\s*(\d+)\s*(?:-\s*(\d+))?\s*(?:\(A15\+?:\s*(\d+)(?:\s*-\s*(\d+))?\))?\s*[Gg]old',
            r'[Rr]eceive\s*(\d+)\s*(?:-\s*(\d+))?\s*[Gg]old',
            r'[Oo]btain\s*(\d+)\s*(?:-\s*(\d+))?\s*[Gg]old',
        ]
        
        for pattern in gold_patterns:
            match = re.search(pattern, desc)
            if match:
                if match.group(2):  # Range
                    rewards['gold'] = {'min': int(match.group(1)), 'max': int(match.group(2))}
                else:
                    rewards['gold'] = int(match.group(1))
                if match.lastindex >= 3 and match.group(3):
                    if match.lastindex >= 4 and match.group(4):
                        rewards['gold_ascension'] = {'min': int(match.group(3)), 'max': int(match.group(4))}
                    else:
                        rewards['gold_ascension'] = int(match.group(3))
                self.stats['rewards_extracted'] += 1
                break
        
        # HP gain patterns
        hp_patterns = [
            r'[Gg]ain\s*(\d+)\s*(?:\(A15\+?:\s*(\d+)\))?\s*HP(?!\s*Max)',
            r'[Hh]eal\s*(\d+)\s*(?:\(A15\+?:\s*(\d+)\))?\s*HP',
            r'[Rr]estore\s*(\d+)\s*(?:\(A15\+?:\s*(\d+)\))?\s*HP',
        ]
        
        for pattern in hp_patterns:
            match = re.search(pattern, desc)
            if match:
                rewards['hp'] = int(match.group(1))
                if match.group(2):
                    rewards['hp_ascension'] = int(match.group(2))
                self.stats['rewards_extracted'] += 1
                break
        
        # Max HP gain
        max_hp_match = re.search(r'[Gg]ain\s*(\d+)\s*(?:\(A15\+?:\s*(\d+)\))?\s*[Mm]ax\s*HP', desc)
        if max_hp_match:
            rewards['max_hp'] = int(max_hp_match.group(1))
            if max_hp_match.group(2):
                rewards['max_hp_ascension'] = int(max_hp_match.group(2))
            self.stats['rewards_extracted'] += 1
        
        # Card rewards
        card_patterns = [
            (r'[Oo]btain\s+(?:a\s+)?([\w\s]+)\s+card', 'specific'),
            (r'[Gg]et\s+(?:a\s+)?([\w\s]+)\s+card', 'specific'),
            (r'[Rr]eceive\s+(?:a\s+)?([\w\s]+)\s+card', 'specific'),
            (r'[Aa]dd\s+(?:a\s+)?([\w\s]+)\s+(?:to|card)', 'specific'),
            (r'[Uu]pgrade\s+(?:a\s+)?card', 'upgrade'),
            (r'[Tt]ransform\s+(?:a\s+)?card', 'transform'),
        ]
        
        for pattern, card_type in card_patterns:
            match = re.search(pattern, desc)
            if match:
                if card_type == 'upgrade':
                    rewards['upgrade_card'] = 1
                elif card_type == 'transform':
                    rewards['transform_card'] = 1
                else:
                    rewards['card'] = match.group(1).strip() if match.groups() else 'random'
                self.stats['rewards_extracted'] += 1
                break
        
        # Relic rewards
        relic_patterns = [
            r'[Oo]btain\s+(?:a\s+)?(?:special\s+)?[Rr]elic',
            r'[Gg]ain\s+(?:a\s+)?(?:random\s+)?[Rr]elic',
            r'[Rr]eceive\s+([\w\s]+?)(?:\s+relic)',
        ]
        
        for pattern in relic_patterns:
            match = re.search(pattern, desc)
            if match:
                if match.groups():
                    rewards['relic'] = match.group(1).strip()
                else:
                    rewards['relic'] = 'random'
                self.stats['rewards_extracted'] += 1
                break
        
        # Potion rewards
        if re.search(r'[Gg]et\s+(?:a\s+)?[Pp]otion|[Oo]btain\s+(?:a\s+)?[Pp]otion', desc):
            rewards['potion'] = 'random'
            self.stats['rewards_extracted'] += 1
        
        # Curse
        curse_match = re.search(r'[Bb]ecome\s+[Cc]ursed\s*[-–]\s*([\w\s]+)', desc)
        if curse_match:
            rewards['curse'] = curse_match.group(1).strip()
            self.stats['rewards_extracted'] += 1
        elif re.search(r'[Cc]urse', desc):
            rewards['curse'] = 'random'
            self.stats['rewards_extracted'] += 1
        
        return rewards

    def parse_requirements(self, conditions: List[str], desc: str) -> Dict[str, Any]:
        """Extract structured requirements from conditions."""
        requirements = {}
        
        all_text = ' '.join(conditions) + ' ' + desc
        
        # Check for relic requirements
        relic_patterns = [
            r'[Rr]equires?:?\s*([\w\s]+?)(?:\s+relic|\.|$)',
            r'[Hh]ave\s+([\w\s]+?)(?:\s+relic|\s+to|\.|$)',
            r'[Nn]eeds?\s+([\w\s]+?)(?:\s+relic|\.|$)',
        ]
        
        for pattern in relic_patterns:
            match = re.search(pattern, all_text)
            if match:
                relic = match.group(1).strip()
                # Filter out non-relic matches
                if relic and len(relic) > 2 and not any(x in relic.lower() for x in ['hp', 'gold', 'card', 'the player']):
                    requirements['has_relic'] = relic
                    self.stats['requirements_structured'] += 1
                    break
        
        # Check for gold requirements
        gold_match = re.search(r'[Rr]equires?\s*(\d+)\s*[Gg]old|[Nn]eeds?\s*(\d+)\s*[Gg]old|[Hh]ave\s*(\d+)\s*[Gg]old', all_text)
        if gold_match:
            gold = gold_match.group(1) or gold_match.group(2) or gold_match.group(3)
            requirements['min_gold'] = int(gold)
            self.stats['requirements_structured'] += 1
        
        # Check for HP requirements  
        hp_match = re.search(r'[Rr]equires?\s*(\d+)\s*HP|[Nn]eeds?\s*(\d+)\s*HP|[Hh]ave\s*(\d+)\s*HP', all_text)
        if hp_match:
            hp = hp_match.group(1) or hp_match.group(2) or hp_match.group(3)
            requirements['min_hp'] = int(hp)
            self.stats['requirements_structured'] += 1
        
        # Check for card type requirements
        card_patterns = [
            r'[Hh]ave\s+(?:a\s+|an\s+)?([\w]+)\s+card\s+in',
            r'[Rr]equires?\s+(?:a\s+|an\s+)?([\w]+)\s+card',
        ]
        
        for pattern in card_patterns:
            match = re.search(pattern, all_text)
            if match:
                requirements['has_card_type'] = match.group(1).lower()
                self.stats['requirements_structured'] += 1
                break
        
        return requirements

    def structure_option(self, option: Dict[str, Any]) -> Dict[str, Any]:
        """Transform a single option into structured format."""
        self.stats['options_processed'] += 1
        
        structured = {
            'label': option.get('label', ''),
            'description': option.get('description', ''),
        }
        
        # Parse costs
        desc = option.get('description', '')
        costs = self.parse_cost_from_description(desc)
        
        # Merge with existing costs if any
        if 'costs' in option:
            costs.update(option['costs'])
        
        if costs:
            structured['costs'] = costs
        
        # Parse rewards
        rewards = self.parse_reward_from_description(desc)
        
        # Also check effects for rewards
        for effect in option.get('effects', []):
            effect_rewards = self.parse_reward_from_description(effect)
            rewards.update(effect_rewards)
        
        # Merge with existing rewards if any
        if 'rewards' in option:
            if isinstance(option['rewards'], dict):
                rewards.update(option['rewards'])
        
        if rewards:
            structured['rewards'] = rewards
        
        # Parse requirements
        conditions = option.get('conditions', [])
        requirements = self.parse_requirements(conditions, desc)
        
        # Merge with existing requirements
        if 'requirements' in option:
            requirements.update(option['requirements'])
        
        if requirements:
            structured['requirements'] = requirements
        
        # Keep effects that provide additional context
        effects = option.get('effects', [])
        if effects:
            structured['effects'] = effects
        
        # Keep conditions that weren't parsed into requirements
        if conditions and not requirements:
            structured['conditions'] = conditions
        
        # Copy over special fields
        for key in ['loop_action', 'transition_to', 'notes', 'random_outcomes']:
            if key in option:
                structured[key] = option[key]
        
        return structured

    def structure_event(self, event: Dict[str, Any]) -> Dict[str, Any]:
        """Transform a single event into structured format."""
        self.stats['events_processed'] += 1
        
        structured = {
            'wiki_id': event.get('wiki_id', ''),
            'name': event.get('name', ''),
            'category': event.get('category', ''),
        }
        
        # Copy event type and special mechanics
        for key in ['event_type', 'multi_phase', 'loop_mechanic', 'minigame', 'phases']:
            if key in event:
                structured[key] = event[key]
        
        # Structure options
        options = event.get('options', [])
        structured['options'] = [self.structure_option(opt) for opt in options]
        
        # Keep notes
        if event.get('notes'):
            structured['notes'] = event['notes']
        
        # Keep raw data for reference (optional)
        # structured['_raw_options'] = event.get('raw_options', '')
        
        return structured

    def process_file(self, input_path: Path) -> List[Dict[str, Any]]:
        """Process a single JSON file."""
        with open(input_path, 'r', encoding='utf-8') as f:
            events = json.load(f)
        
        return [self.structure_event(event) for event in events]

    def run(self):
        """Process all event files."""
        # Ensure output directory exists
        OUTPUT_DIR.mkdir(parents=True, exist_ok=True)
        
        input_files = ['act1.json', 'act2.json', 'act3.json', 'shrines.json']
        
        for filename in input_files:
            input_path = DATA_DIR / filename
            output_path = OUTPUT_DIR / filename
            
            if not input_path.exists():
                print(f"Warning: {input_path} not found, skipping...")
                continue
            
            print(f"\nProcessing {filename}...")
            structured_events = self.process_file(input_path)
            
            with open(output_path, 'w', encoding='utf-8') as f:
                json.dump(structured_events, f, indent=2, ensure_ascii=False)
            
            print(f"  → Wrote {len(structured_events)} events to {output_path}")
        
        # Print statistics
        print("\n" + "="*50)
        print("Logic Structuring Statistics:")
        print("="*50)
        for key, value in self.stats.items():
            print(f"  {key}: {value}")


def validate_structured_output():
    """Validate the structured output files."""
    print("\n" + "="*50)
    print("Validating Output...")
    print("="*50)
    
    required_fields = ['wiki_id', 'name', 'category', 'options']
    option_fields = ['label']
    
    for filename in ['act1.json', 'act2.json', 'act3.json', 'shrines.json']:
        filepath = OUTPUT_DIR / filename
        if not filepath.exists():
            print(f"  ❌ {filename} not found")
            continue
        
        with open(filepath, 'r', encoding='utf-8') as f:
            events = json.load(f)
        
        errors = []
        for event in events:
            for field in required_fields:
                if field not in event:
                    errors.append(f"{event.get('wiki_id', 'UNKNOWN')}: missing {field}")
            
            for i, opt in enumerate(event.get('options', [])):
                for field in option_fields:
                    if field not in opt:
                        errors.append(f"{event.get('wiki_id', 'UNKNOWN')} option {i}: missing {field}")
        
        if errors:
            print(f"  ⚠️ {filename}: {len(errors)} issues")
            for err in errors[:5]:
                print(f"     - {err}")
            if len(errors) > 5:
                print(f"     ... and {len(errors) - 5} more")
        else:
            print(f"  ✅ {filename}: {len(events)} events validated")


if __name__ == '__main__':
    structurer = LogicStructurer()
    structurer.run()
    validate_structured_output()
