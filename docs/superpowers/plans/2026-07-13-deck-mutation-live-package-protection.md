# Deck mutation live-package protection

## Observed failure

At the Act 3 Bonfire Spirits boundary, a committed forced purge chose `Limit Break` from a deck that already had `Spot Weakness`. The mutation compiler treated the multiplier as an ordinary functional card and did not account for destroying the deck's only source-times-multiplier package. Equal candidates were also resolved through the rendered `select N` command, so lexical ordering could place index 11 before index 2.

## Contract

- A strength multiplier is not itself a strength source.
- Removing the final source or final multiplier from a live source-times-multiplier package is core functional loss.
- Equal mutation candidates use numeric deck indices as their stable final tie-break; rendered command text is not decision semantics.
- Forced mutations may still select protected cards when every legal target is protected.

## Verification

- Add a strategic-deficit regression for a multiplier without a source.
- Add an exact forced-purge regression based on the observed deck shape.
- Keep the existing deck-mutation compiler suite green, then rerun the bounded seed.
