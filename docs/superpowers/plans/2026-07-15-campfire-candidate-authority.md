# Campfire Candidate Authority Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Establish one engine-owned enumeration of every legal Campfire candidate, with Smith and Toke targets bound to stable card UUIDs and safely resolved to current deck indices.

**Architecture:** Add a focused `engine::campfire_candidates` module as the legality and target-identity authority. Keep `CampfireChoice` as the index-based input protocol, derive its family placeholders from canonical candidates, and make the existing handler delegate availability checks to that module. This slice changes no AI ranking, owner behavior, RNG, or prospect model.

**Tech Stack:** Rust 2021, Cargo, serde, existing `RunState`, `CombatCard.uuid`, `CampfireChoice`, and Campfire engine tests.

## Global Constraints

- Work in the stable checkout at `D:\rust\sts_simulator`; do not create a worktree.
- Do not run `cargo clean`.
- Do not change Campfire mechanics, candidate ordering, AI ranking, owner-audit behavior, route policy, reward policy, RNG streams, or run objectives.
- `CombatCard.uuid` is the stable target identity; deck indices remain input/display data only.
- Enumerate every legal Smith and Toke target; do not truncate or score candidates.
- Preserve the existing Java-compatible family order: Rest, Smith, relic options in acquisition order, Recall.
- Use focused tests during red/green work, then run the full library and `architecture_runtime_boundaries` suites.

---

### Task 1: Add Stable Engine Candidate Enumeration

**Files:**
- Create: `src/engine/campfire_candidates.rs`
- Modify: `src/engine/mod.rs`

**Interfaces:**
- Produces: `CampfireCandidate`, `CampfireCandidateResolutionError`, `legal_campfire_candidates(&RunState)`, `campfire_candidate_for_choice(&RunState, CampfireChoice)`, and `resolve_campfire_candidate(&RunState, CampfireCandidate)`.
- Preserves: `CampfireChoice` as the executable input type and every existing Campfire mechanic.

- [ ] **Step 1: Write failing enumeration and stable-resolution tests**

Create `src/engine/campfire_candidates.rs` with imports and tests that reference the planned API before its implementation, and add `pub mod campfire_candidates;` to `src/engine/mod.rs` so Cargo compiles the new test module:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::cards::CardId;
    use crate::content::relics::{RelicId, RelicState};
    use crate::runtime::combat::CombatCard;
    use crate::state::core::CampfireChoice;
    use crate::state::run::RunState;

    fn candidate_run() -> RunState {
        let mut run = RunState::new(17, 0, true, "Ironclad");
        run.master_deck = vec![
            CombatCard::new(CardId::Strike, 101),
            CombatCard::new(CardId::Defend, 102),
            CombatCard::new(CardId::AscendersBane, 103),
        ];
        run.relics = vec![
            RelicState::new(RelicId::Girya),
            RelicState::new(RelicId::Shovel),
            RelicState::new(RelicId::PeacePipe),
        ];
        run.keys[0] = false;
        run
    }

    #[test]
    fn legal_candidates_expand_every_smith_and_toke_target_by_uuid() {
        let candidates = legal_campfire_candidates(&candidate_run());

        assert_eq!(
            candidates,
            vec![
                CampfireCandidate::Rest,
                CampfireCandidate::Smith { card_uuid: 101 },
                CampfireCandidate::Smith { card_uuid: 102 },
                CampfireCandidate::Lift,
                CampfireCandidate::Dig,
                CampfireCandidate::Toke { card_uuid: 101 },
                CampfireCandidate::Toke { card_uuid: 102 },
                CampfireCandidate::Recall,
            ]
        );
    }

    #[test]
    fn stable_target_resolution_tracks_uuid_after_deck_reordering() {
        let mut run = candidate_run();
        run.master_deck.swap(0, 1);

        assert_eq!(
            resolve_campfire_candidate(
                &run,
                CampfireCandidate::Smith { card_uuid: 101 }
            ),
            Ok(CampfireChoice::Smith(1))
        );
        assert_eq!(
            resolve_campfire_candidate(
                &run,
                CampfireCandidate::Toke { card_uuid: 102 }
            ),
            Ok(CampfireChoice::Toke(0))
        );
    }

    #[test]
    fn removed_target_fails_resolution_instead_of_retargeting_an_index() {
        let mut run = candidate_run();
        run.master_deck.retain(|card| card.uuid != 101);

        assert_eq!(
            resolve_campfire_candidate(
                &run,
                CampfireCandidate::Smith { card_uuid: 101 }
            ),
            Err(CampfireCandidateResolutionError::MissingCardUuid {
                card_uuid: 101,
            })
        );
    }
}
```

- [ ] **Step 2: Run the focused test target and verify RED**

Run:

```powershell
cargo test -p sts_simulator engine::campfire_candidates::tests --lib
```

Expected: compilation fails because the candidate types and functions do not yet exist.

- [ ] **Step 3: Implement candidate types, complete enumeration, and stable resolution**

Add this production surface above the tests in `src/engine/campfire_candidates.rs`:

```rust
use serde::{Deserialize, Serialize};

use crate::content::relics::RelicId;
use crate::state::core::{
    master_deck_card_can_upgrade, master_deck_card_is_bottled,
    master_deck_card_is_purgeable, CampfireChoice,
};
use crate::state::run::RunState;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CampfireCandidate {
    Rest,
    Smith { card_uuid: u32 },
    Dig,
    Lift,
    Toke { card_uuid: u32 },
    Recall,
}

impl CampfireCandidate {
    pub fn family_placeholder_choice(self) -> CampfireChoice {
        match self {
            Self::Rest => CampfireChoice::Rest,
            Self::Smith { .. } => CampfireChoice::Smith(0),
            Self::Dig => CampfireChoice::Dig,
            Self::Lift => CampfireChoice::Lift,
            Self::Toke { .. } => CampfireChoice::Toke(0),
            Self::Recall => CampfireChoice::Recall,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CampfireCandidateResolutionError {
    MissingCardUuid { card_uuid: u32 },
    AmbiguousCardUuid { card_uuid: u32 },
    Unavailable { candidate: CampfireCandidate },
}

pub fn legal_campfire_candidates(run_state: &RunState) -> Vec<CampfireCandidate> {
    let mut candidates = Vec::new();
    let has_relic = |id| run_state.relics.iter().any(|relic| relic.id == id);

    if !has_relic(RelicId::CoffeeDripper) {
        candidates.push(CampfireCandidate::Rest);
    }
    if !has_relic(RelicId::FusionHammer) {
        candidates.extend(
            run_state
                .master_deck
                .iter()
                .filter(|card| master_deck_card_can_upgrade(card))
                .map(|card| CampfireCandidate::Smith {
                    card_uuid: card.uuid,
                }),
        );
    }
    for relic in &run_state.relics {
        match relic.id {
            RelicId::Girya if relic.counter < 3 => {
                push_unique(&mut candidates, CampfireCandidate::Lift);
            }
            RelicId::Shovel => {
                push_unique(&mut candidates, CampfireCandidate::Dig);
            }
            RelicId::PeacePipe => {
                let targets = run_state
                    .master_deck
                    .iter()
                    .filter(|card| {
                        master_deck_card_is_purgeable(card)
                            && !master_deck_card_is_bottled(card, &run_state.relics)
                    })
                    .map(|card| CampfireCandidate::Toke {
                        card_uuid: card.uuid,
                    })
                    .collect::<Vec<_>>();
                for target in targets {
                    push_unique(&mut candidates, target);
                }
            }
            _ => {}
        }
    }
    if run_state.is_final_act_available && !run_state.keys[0] {
        candidates.push(CampfireCandidate::Recall);
    }
    candidates
}

pub fn campfire_candidate_for_choice(
    run_state: &RunState,
    choice: CampfireChoice,
) -> Option<CampfireCandidate> {
    Some(match choice {
        CampfireChoice::Rest => CampfireCandidate::Rest,
        CampfireChoice::Smith(index) => CampfireCandidate::Smith {
            card_uuid: run_state.master_deck.get(index)?.uuid,
        },
        CampfireChoice::Dig => CampfireCandidate::Dig,
        CampfireChoice::Lift => CampfireCandidate::Lift,
        CampfireChoice::Toke(index) => CampfireCandidate::Toke {
            card_uuid: run_state.master_deck.get(index)?.uuid,
        },
        CampfireChoice::Recall => CampfireCandidate::Recall,
    })
}

pub fn resolve_campfire_candidate(
    run_state: &RunState,
    candidate: CampfireCandidate,
) -> Result<CampfireChoice, CampfireCandidateResolutionError> {
    let choice = match candidate {
        CampfireCandidate::Rest => CampfireChoice::Rest,
        CampfireCandidate::Smith { card_uuid } => {
            CampfireChoice::Smith(unique_card_index(run_state, card_uuid)?)
        }
        CampfireCandidate::Dig => CampfireChoice::Dig,
        CampfireCandidate::Lift => CampfireChoice::Lift,
        CampfireCandidate::Toke { card_uuid } => {
            CampfireChoice::Toke(unique_card_index(run_state, card_uuid)?)
        }
        CampfireCandidate::Recall => CampfireChoice::Recall,
    };
    if legal_campfire_candidates(run_state).contains(&candidate) {
        Ok(choice)
    } else {
        Err(CampfireCandidateResolutionError::Unavailable { candidate })
    }
}

fn unique_card_index(
    run_state: &RunState,
    card_uuid: u32,
) -> Result<usize, CampfireCandidateResolutionError> {
    let mut matches = run_state
        .master_deck
        .iter()
        .enumerate()
        .filter(|(_, card)| card.uuid == card_uuid)
        .map(|(index, _)| index);
    let Some(index) = matches.next() else {
        return Err(CampfireCandidateResolutionError::MissingCardUuid { card_uuid });
    };
    if matches.next().is_some() {
        return Err(CampfireCandidateResolutionError::AmbiguousCardUuid { card_uuid });
    }
    Ok(index)
}

fn push_unique(candidates: &mut Vec<CampfireCandidate>, candidate: CampfireCandidate) {
    if !candidates.contains(&candidate) {
        candidates.push(candidate);
    }
}
```

Export the module in `src/engine/mod.rs`:

```rust
pub mod campfire_candidates;
```

- [ ] **Step 4: Run focused tests and verify GREEN**

Run:

```powershell
cargo test -p sts_simulator engine::campfire_candidates::tests --lib
```

Expected: the three new tests pass.

- [ ] **Step 5: Commit the engine candidate surface**

```powershell
git add src/engine/campfire_candidates.rs src/engine/mod.rs
git commit -m "feat: enumerate stable campfire candidates"
```

---

### Task 2: Make The Campfire Handler Consume The Candidate Authority

**Files:**
- Modify: `src/engine/campfire_handler.rs`
- Test: `src/engine/campfire_handler.rs`

**Interfaces:**
- Consumes: `legal_campfire_candidates` and `campfire_candidate_for_choice` from Task 1.
- Preserves: `get_available_options(&RunState) -> Vec<CampfireChoice>` and `campfire_choice_is_available(&RunState, CampfireChoice) -> bool` for existing UI and engine callers.

- [ ] **Step 1: Write failing handler-authority regression tests**

Add this test to the existing `campfire_handler.rs` test module:

```rust
#[test]
fn available_options_collapse_complete_target_sets_to_family_placeholders() {
    let mut run = RunState::new(19, 0, false, "Ironclad");
    run.master_deck = vec![
        CombatCard::new(CardId::Strike, 301),
        CombatCard::new(CardId::Defend, 302),
    ];
    run.relics = vec![RelicState::new(RelicId::PeacePipe)];

    let candidates = crate::engine::campfire_candidates::legal_campfire_candidates(&run);
    assert_eq!(
        candidates
            .iter()
            .filter(|candidate| matches!(candidate, crate::engine::campfire_candidates::CampfireCandidate::Smith { .. }))
            .count(),
        2
    );
    assert_eq!(
        candidates
            .iter()
            .filter(|candidate| matches!(candidate, crate::engine::campfire_candidates::CampfireCandidate::Toke { .. }))
            .count(),
        2
    );
    assert_eq!(
        super::get_available_options(&run),
        vec![
            CampfireChoice::Rest,
            CampfireChoice::Smith(0),
            CampfireChoice::Toke(0),
        ]
    );
}
```

Add a regression proving the family surface cannot claim Smith exists when the authoritative target set is empty:

```rust
#[test]
fn available_options_do_not_offer_smith_for_only_nonupgradable_cards() {
    let mut run = RunState::new(23, 0, false, "Ironclad");
    run.master_deck = vec![CombatCard::new(CardId::AscendersBane, 401)];

    assert_eq!(super::get_available_options(&run), vec![CampfireChoice::Rest]);
}
```

- [ ] **Step 2: Run handler tests and verify RED**

Run before changing the handler:

```powershell
cargo test -p sts_simulator engine::campfire_handler::tests --lib
```

Expected: `available_options_do_not_offer_smith_for_only_nonupgradable_cards` fails because the old family-level Smith check does not use the authoritative card-upgrade rule.

- [ ] **Step 3: Delegate option projection and availability checks**

Import the candidate authority:

```rust
use crate::engine::campfire_candidates::{
    campfire_candidate_for_choice, legal_campfire_candidates,
};
```

Replace `campfire_choice_is_available` with:

```rust
pub fn campfire_choice_is_available(run_state: &RunState, choice: CampfireChoice) -> bool {
    campfire_candidate_for_choice(run_state, choice)
        .is_some_and(|candidate| legal_campfire_candidates(run_state).contains(&candidate))
}
```

Replace `get_available_options` with a projection that preserves one placeholder per family:

```rust
pub fn get_available_options(run_state: &RunState) -> Vec<CampfireChoice> {
    let mut options = Vec::new();
    for candidate in legal_campfire_candidates(run_state) {
        let option = candidate.family_placeholder_choice();
        if !options.contains(&option) {
            options.push(option);
        }
    }
    options
}
```

Delete the old Coffee Dripper, Fusion Hammer, Smith-target, Peace Pipe, Girya, Shovel, and Recall availability rules from `campfire_handler.rs`; their only production home is now `campfire_candidates.rs`.

- [ ] **Step 4: Run candidate and handler tests and verify GREEN**

Run:

```powershell
cargo test -p sts_simulator engine::campfire_candidates::tests --lib
cargo test -p sts_simulator engine::campfire_handler::tests --lib
```

Expected: all candidate and existing Campfire handler tests pass with unchanged mechanics and option ordering.

- [ ] **Step 5: Commit the handler delegation**

```powershell
git add src/engine/campfire_handler.rs
git commit -m "refactor: centralize campfire candidate legality"
```

---

### Task 3: Verify The Candidate-Authority Slice

**Files:**
- Verify only; no planned source changes.

**Interfaces:**
- Verifies: complete targeted candidate coverage, stable UUID resolution, handler compatibility, and repository architecture constraints.

- [ ] **Step 1: Format and run focused tests**

```powershell
cargo fmt --all -- --check
cargo test -p sts_simulator engine::campfire_candidates --lib
cargo test -p sts_simulator engine::campfire_handler --lib
```

Expected: formatting succeeds and both focused test groups pass.

- [ ] **Step 2: Run completion suites required by `AGENTS.md`**

```powershell
cargo test -p sts_simulator --lib
cargo test -p sts_simulator --test architecture_runtime_boundaries
```

Expected: the full library and architecture boundary suites pass.

- [ ] **Step 3: Confirm scope and history**

```powershell
git status --short --branch
git log -3 --oneline
```

Expected: no uncommitted source changes, and the two candidate-authority commits appear above the specification and plan commits.
