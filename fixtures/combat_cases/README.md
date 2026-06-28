# Combat Search Cases

This folder holds small, fixed combat-search cases that are useful for local
search work. A case is not a policy verdict or a campaign artifact. It is a
reproducible combat boundary with enough state to compare search behavior.

## A2F23 Slavers Threshold Case

Case:

```text
fixtures/combat_cases/seed1700000123_a2f23_slavers_b0034.json
```

Context:

```text
seed=1700000123
ascension=0
act=2 floor=23
enemy=Blue Slaver + Taskmaster + Red Slaver
hp=65/85
deck_size=16
hand=Strike, Defend, Clothesline+, Armaments+, Second Wind+
potions=Blessing of the Forge, Blessing of the Forge, Block Potion
```

Baseline checks:

```powershell
cargo run --quiet --bin combat_gap_probe -- --case fixtures\combat_cases\seed1700000123_a2f23_slavers_b0034.json --nodes 100000 --ms 1000 --accept-any-win --search-only
cargo run --quiet --bin combat_gap_probe -- --case fixtures\combat_cases\seed1700000123_a2f23_slavers_b0034.json --nodes 100000 --ms 1000 --accept-any-win --search-only --turn-plan-policy diagnostic_only
cargo run --quiet --bin combat_gap_probe -- --case fixtures\combat_cases\seed1700000123_a2f23_slavers_b0034.json --nodes 100000 --ms 2000 --accept-any-win --search-only
```

Expected current behavior:

```text
1000ms default tactical turn-plan seed: no complete win; turn-plan seeding consumes the budget
1000ms diagnostic_only: no-potion win found; final_hp ~= 32
2000ms: no-potion win found; final_hp ~= 32; hp_loss ~= 33
```

Use this case to improve first-win discovery. Do not treat raising the global
budget as progress. A useful search change should preserve the 1000ms
diagnostic win while reducing the need for expensive turn-plan seeding in
first-win search paths.

Reference directions:

- [Nested Monte-Carlo Search](https://www.ijcai.org/Proceedings/09/Papers/083.pdf):
  useful when there is no strong static evaluator and the immediate need is to
  find any complete action sequence.
- [Nested Rollout Policy Adaptation](https://www.ijcai.org/Proceedings/11/Papers/115.pdf):
  useful later if we start learning local action preferences from found winning
  lines.
- [Beam Monte-Carlo Tree Search](https://dke.maastrichtuniversity.nl/m.winands/documents/CIG2012_paper_32.pdf):
  useful if a bounded width of promising partial lines can replace broad root
  fanout without hiding critical lines.
