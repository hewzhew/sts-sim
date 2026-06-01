# Deck Slice Provenance

Deck slices are inputs for combat-policy evaluation, not labels for card-pick
quality.

Do not treat synthetic or weak-policy decks as a real developing-run
distribution. Every slice must carry a provenance label:

- `live`: captured from an actual game state.
- `replay`: captured from a replay or saved CommunicationMod frame.
- `authored_probe`: hand-written probe deck for a specific combat question.
- `weak_policy`: produced by a known weak bot; usable only as behavior coverage,
  not as proof that a card choice was good.

Randomly inserting cards into a starter deck is not a valid source for
deckbuilding evaluation. It can only be used as an `authored_probe` when the
question is explicit, for example: "does the combat policy understand Cleave in
a two-target fight?"

Minimal JSONL record:

```json
{
  "schema": "sts.deck_slice.v0",
  "source_kind": "live",
  "source_path": "tools/replays/example.jsonl",
  "source_frame": 123,
  "character": "Ironclad",
  "act": 1,
  "floor": 6,
  "hp": 48,
  "max_hp": 80,
  "gold": 127,
  "deck": [
    {"id": "Strike_R", "upgrades": 0},
    {"id": "Bash", "upgrades": 0}
  ],
  "notes": []
}
```

Use:

- combat evaluation across real or clearly authored deck slices.
- card-mechanics coverage.
- identifying which combat skills fail under specific decks.

Do not use:

- training a reward-choice policy as if the deck source were expert behavior.
- proving that a card pick was good.
- replacing heldout full-run evaluation.
