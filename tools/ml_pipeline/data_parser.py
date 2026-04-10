import json
import os
import glob
from typing import List, Dict, Any

class StateVector:
    def __init__(self, floor: int, deck: List[str], relics: List[str], current_hp: int, max_hp: int, gold: int):
        self.floor = floor
        self.deck = deck.copy()
        self.relics = relics.copy()
        self.current_hp = current_hp
        self.max_hp = max_hp
        self.gold = gold

    def to_dict(self):
        return {
            "floor": self.floor,
            "deck": self.deck,
            "relics": self.relics,
            "hp_percent": self.current_hp / max(1, self.max_hp),
            "gold": self.gold
        }

class Transition:
    def __init__(self, state: StateVector, action_type: str, action_details: Any, reward: float):
        self.state = state
        self.action_type = action_type        # 'card_choice', 'event_choice', 'campfire_choice', 'shop_choice'
        self.action_details = action_details  # 'Demon Form', or 'Skip', or 'Smith Strike'
        self.reward = reward                  # e.g., 1.0 for Win, 0.0 for Loss

def parse_run_file(filepath: str) -> List[Transition]:
    with open(filepath, 'r', encoding='utf-8') as f:
        data = json.load(f)
    
    # Check metric format wrapper (sometimes wrapped in 'event' or just raw)
    # Commonly it's direct JSON of the run.
    run_data = data if 'floor_reached' in data else data.get('event', {})
    
    floor_reached = run_data.get('floor_reached', 0)
    victory = run_data.get('victory', False)
    reward = 1.0 if victory else 0.0
    
    # We will reconstruct the trajectory loosely (SpireLogs doesn't give frame-by-frame exact state naturally, 
    # but summarizes choices with their floors).
    transitions = []
    
    # Starter mock state (would ideally reconstruct event-by-event)
    card_choices = run_data.get('card_choices', [])
    # Sort card choices by floor
    card_choices.sort(key=lambda x: x.get('floor', 0))
    
    # In a fully realized parser, we simulate the run from Floor 1 to Floor N, modifying 'deck' and 'relics' locally.
    # For now, we will extract just the final reward mapping to specific choices for MVP.
    
    # Example logic for parsing card choices
    for choice in card_choices:
        floor = choice.get('floor')
        picked = choice.get('picked', 'SKIP')
        not_picked = choice.get('not_picked', [])
        
        # We pretend we know the deck at this floor (Simplified for now - will need sequential reconstruction)
        dummy_state = StateVector(floor, [], [], 80, 80, 99) 
        
        t = Transition(dummy_state, "card_choice", {"picked": picked, "offered": not_picked + [picked]}, reward)
        transitions.append(t)
        
    return transitions

if __name__ == "__main__":
    import sys
    print("STS Data Parser Initialized.")
    if len(sys.argv) > 1:
        run_file = sys.argv[1]
        t = parse_run_file(run_file)
        print(f"Parsed {len(t)} transitions from {run_file}")
