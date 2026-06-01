# Silent Card Play Split Audit

Status: current as of this branch.

## Result

Silent card files are now structurally split by card:

- every `src/content/cards/silent/*.rs` file except `mod.rs` has its own
  `definition()`;
- every Silent card file has a local play entrypoint;
- `src/content/cards/silent/mod.rs` is only a module index;
- ordinary Silent play paths no longer directly read transient mutable card
  fields such as `card.base_magic_num_mut`, `card.base_damage_mut`, or
  `card.base_block_mut`;
- ordinary Silent play paths use `evaluate_card_for_play` when upgrade/rendered
  damage, block, or magic values matter.

Audit command:

```powershell
python tools/audit_silent_card_upgrade_reads.py
```

Current audit result:

```text
files=6 matches=6 unclassified=0
```

The six remaining direct reads are all classified as intentional special
upgrade semantics in
`docs/audits/SILENT_CARD_UPGRADE_READ_AUDIT.md`:

- `adrenaline.rs`
- `doppelganger.rs`
- `malaise.rs`
- `reflex.rs`
- `storm_of_steel.rs`
- `tactician.rs`

These are not ordinary rendered-value reads. They pass `upgraded` or compute
manual-discard semantics where Java uses the card's upgraded flag directly.

## Maintenance Rules

New Silent card code must follow this shape:

```text
src/content/cards/silent/<card>.rs
  definition() owns printed/base/upgrade numbers.
  <card>_play(...) owns special behavior.
  ordinary play values are read from evaluate_card_for_play(...).
```

Do not reintroduce a centralized `play_silent` or place card-specific behavior
in `runtime_impl.rs` beyond dispatch.

Do not use tests that manually prefill `base_magic_num_mut`,
`base_damage_mut`, or `base_block_mut` to represent upgrades. Upgrade through
`card.upgrades` or the normal card upgrade path, then let the play handler
evaluate the card.

## Verification Snapshot

The structural check used during this audit:

```text
Silent card files: 75
missing definition(): none
missing *_play(...): none
direct transient field reads: none
remaining direct card.upgrades reads: 6, all classified
```
