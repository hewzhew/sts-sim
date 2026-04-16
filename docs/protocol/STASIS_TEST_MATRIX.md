# Stasis Test Matrix

This matrix tracks what is currently proven for Bronze Orb `Stasis` behavior,
and what is still only protocol-level truth.

## Oracle

- Java source:
  - `cardcrawl/actions/unique/ApplyStasisAction.java`
  - `cardcrawl/powers/StasisPower.java`
- Rust behavior tests:
  - [tests/stasis_behavior.rs](../../tests/stasis_behavior.rs)
- Protocol truth fixture:
  - [tests/protocol_truth_samples/stasis/frame.json](../../tests/protocol_truth_samples/stasis/frame.json)

## Proven By Current Coverage

- Bronze Orb captures a real card from draw pile or discard pile, not an invented
  placeholder.
- The captured card is removed from its original pile and placed into `limbo`.
- `Stasis` power runtime data tracks the captured card UUID.
- When the orb dies and hand has space, the captured card returns as a copy to
  hand.
- When the orb dies and hand is full, the captured card returns as a copy to
  discard.
- The stasis-held limbo copy is removed after the return action resolves.

## Not Yet Proven

- Java parity for rarity-priority selection when more than one eligible card is
  present in draw or discard.
- Behavior when draw pile is empty and discard pile contains multiple rarity
  buckets.
- Full live parity for the visible `ShowCardAction` sequencing.
- Interactions with edge cases such as simultaneous orb death ordering in larger
  action queues.
