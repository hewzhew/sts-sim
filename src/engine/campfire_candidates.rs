use serde::{Deserialize, Serialize};

use crate::content::relics::RelicId;
use crate::state::core::{
    master_deck_card_can_upgrade, master_deck_card_is_bottled, master_deck_card_is_purgeable,
    CampfireChoice,
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
            resolve_campfire_candidate(&run, CampfireCandidate::Smith { card_uuid: 101 }),
            Ok(CampfireChoice::Smith(1))
        );
        assert_eq!(
            resolve_campfire_candidate(&run, CampfireCandidate::Toke { card_uuid: 102 }),
            Ok(CampfireChoice::Toke(0))
        );
    }

    #[test]
    fn removed_target_fails_resolution_instead_of_retargeting_an_index() {
        let mut run = candidate_run();
        run.master_deck.retain(|card| card.uuid != 101);

        assert_eq!(
            resolve_campfire_candidate(&run, CampfireCandidate::Smith { card_uuid: 101 }),
            Err(CampfireCandidateResolutionError::MissingCardUuid { card_uuid: 101 })
        );
    }
}
