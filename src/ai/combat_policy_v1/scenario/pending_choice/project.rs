use std::collections::{BTreeMap, BTreeSet};

use crate::content::cards::java_id;
use crate::runtime::action::CardDestination;
use crate::runtime::combat::{CombatCard, CombatState};
use crate::state::core::{
    ClientInput, GridSelectReason, HandSelectReason, PendingChoice, PileType,
};
use crate::state::selection::SelectionScope;

use super::super::types::{CombatPublicActionV1, CombatScenarioPolicyErrorV1};
use super::types::{
    CombatPublicCardDestinationV1, CombatPublicCardMultiplicityV1,
    CombatPublicCardSelectionContextV1, CombatPublicGeneratedCardOptionV1,
    CombatPublicGeneratedChoiceKindV1, CombatPublicGridSelectionReasonV1,
    CombatPublicHandSelectionReasonV1, CombatPublicPendingChoiceV1, CombatPublicPileV1,
    CombatPublicStanceV1,
};

pub(in crate::ai::combat_policy_v1::scenario) fn public_pending_choice_observation(
    combat: &CombatState,
    choice: &PendingChoice,
) -> Result<CombatPublicPendingChoiceV1, String> {
    match choice {
        PendingChoice::HandSelect {
            candidate_uuids,
            min_cards,
            max_cards,
            can_cancel,
            reason,
        } => Ok(CombatPublicPendingChoiceV1::HandSelect {
            context: hand_context(*reason),
            min_cards: *min_cards,
            max_cards: *max_cards,
            can_cancel: *can_cancel,
            candidates: candidate_multiplicities(
                &combat.zones.hand,
                candidate_uuids,
                "hand selection",
            )?,
        }),
        PendingChoice::GridSelect {
            source_pile,
            candidate_uuids,
            min_cards,
            max_cards,
            can_cancel,
            reason,
        } => Ok(CombatPublicPendingChoiceV1::GridSelect {
            context: grid_context(*source_pile, *reason),
            min_cards: *min_cards,
            max_cards: *max_cards,
            can_cancel: *can_cancel,
            candidates: candidate_multiplicities(
                source_pile_cards(combat, *source_pile),
                candidate_uuids,
                "grid selection",
            )?,
        }),
        PendingChoice::DiscoverySelect(choice) => {
            Ok(CombatPublicPendingChoiceV1::DiscoverySelect {
                amount: choice.amount,
                can_skip: choice.can_skip,
                options: generated_options(&choice.cards, 0),
            })
        }
        PendingChoice::ScrySelect { cards, card_uuids } => {
            if cards.len() != card_uuids.len() {
                return Err("scry card and UUID counts differ".to_string());
            }
            let mut revealed_cards = Vec::with_capacity(cards.len());
            for (expected_id, uuid) in cards.iter().zip(card_uuids) {
                let card = combat
                    .zones
                    .draw_pile
                    .iter()
                    .find(|card| card.uuid == *uuid)
                    .ok_or_else(|| {
                        format!("scry candidate UUID {uuid} is absent from draw pile")
                    })?;
                if card.id != *expected_id {
                    return Err(format!(
                        "scry candidate UUID {uuid} has card {:?}, expected {expected_id:?}",
                        card.id
                    ));
                }
                revealed_cards.push(crate::ai::combat_policy_v1::combat_policy_card_v1(card));
            }
            Ok(CombatPublicPendingChoiceV1::ScrySelect { revealed_cards })
        }
        PendingChoice::CardRewardSelect {
            cards,
            destination,
            can_skip,
        } => Ok(CombatPublicPendingChoiceV1::CardRewardSelect {
            destination: card_destination(*destination),
            can_skip: *can_skip,
            options: generated_options(cards, 0),
        }),
        PendingChoice::ForeignInfluenceSelect { cards, upgraded } => {
            Ok(CombatPublicPendingChoiceV1::ForeignInfluenceSelect {
                upgraded: *upgraded,
                options: generated_options(cards, u8::from(*upgraded)),
            })
        }
        PendingChoice::ChooseOneSelect { choices } => {
            Ok(CombatPublicPendingChoiceV1::ChooseOneSelect {
                options: choices
                    .iter()
                    .enumerate()
                    .map(|(option_index, choice)| CombatPublicGeneratedCardOptionV1 {
                        option_index,
                        card_id: java_id(choice.card_id).to_string(),
                        upgrades: choice.upgrades,
                    })
                    .collect(),
            })
        }
        PendingChoice::StanceChoice => Ok(CombatPublicPendingChoiceV1::StanceChoice {
            options: vec![CombatPublicStanceV1::Wrath, CombatPublicStanceV1::Calm],
        }),
    }
}

pub(in crate::ai::combat_policy_v1::scenario) fn public_pending_choice_action(
    scenario_id: &str,
    combat: &CombatState,
    choice: &PendingChoice,
    input: &ClientInput,
) -> Result<CombatPublicActionV1, CombatScenarioPolicyErrorV1> {
    match (choice, input) {
        (
            PendingChoice::HandSelect {
                candidate_uuids,
                reason,
                ..
            },
            ClientInput::SubmitSelection(resolution),
        ) if resolution.scope == SelectionScope::Hand => Ok(CombatPublicActionV1::SelectCards {
            context: hand_context(*reason),
            selected: selected_multiplicities(
                scenario_id,
                &combat.zones.hand,
                candidate_uuids,
                &resolution.selected_card_uuids(),
                input,
            )?,
        }),
        (
            PendingChoice::GridSelect {
                source_pile,
                candidate_uuids,
                reason,
                ..
            },
            ClientInput::SubmitSelection(resolution),
        ) if resolution.scope == SelectionScope::Grid => Ok(CombatPublicActionV1::SelectCards {
            context: grid_context(*source_pile, *reason),
            selected: selected_multiplicities(
                scenario_id,
                source_pile_cards(combat, *source_pile),
                candidate_uuids,
                &resolution.selected_card_uuids(),
                input,
            )?,
        }),
        (PendingChoice::ScrySelect { cards, .. }, ClientInput::SubmitScryDiscard(indices)) => {
            let unique = indices.iter().copied().collect::<BTreeSet<_>>();
            if unique.len() != indices.len() || indices.iter().any(|index| *index >= cards.len()) {
                return invalid_action(
                    scenario_id,
                    input,
                    "scry discard indices are duplicated or out of range",
                );
            }
            Ok(CombatPublicActionV1::ScryDiscard {
                revealed_indices: indices.clone(),
            })
        }
        (PendingChoice::DiscoverySelect(choice), ClientInput::SubmitDiscoverChoice(index)) => {
            generated_action(
                scenario_id,
                input,
                CombatPublicGeneratedChoiceKindV1::Discovery,
                &choice.cards,
                *index,
                0,
            )
        }
        (
            PendingChoice::CardRewardSelect { cards, .. },
            ClientInput::SubmitDiscoverChoice(index),
        ) => generated_action(
            scenario_id,
            input,
            CombatPublicGeneratedChoiceKindV1::CardReward,
            cards,
            *index,
            0,
        ),
        (
            PendingChoice::ForeignInfluenceSelect { cards, upgraded },
            ClientInput::SubmitDiscoverChoice(index),
        ) => generated_action(
            scenario_id,
            input,
            CombatPublicGeneratedChoiceKindV1::ForeignInfluence,
            cards,
            *index,
            u8::from(*upgraded),
        ),
        (PendingChoice::ChooseOneSelect { choices }, ClientInput::SubmitDiscoverChoice(index)) => {
            let option = choices.get(*index).ok_or_else(|| {
                CombatScenarioPolicyErrorV1::InvalidLegalAction {
                    scenario_id: scenario_id.to_string(),
                    input: format!("{input:?}"),
                    reason: format!("choose-one option {index} is absent"),
                }
            })?;
            Ok(CombatPublicActionV1::ChooseGeneratedCard {
                choice_kind: CombatPublicGeneratedChoiceKindV1::ChooseOne,
                option_index: *index,
                card_id: java_id(option.card_id).to_string(),
                upgrades: option.upgrades,
            })
        }
        (PendingChoice::StanceChoice, ClientInput::SubmitDiscoverChoice(0)) => {
            Ok(CombatPublicActionV1::ChooseStance {
                stance: CombatPublicStanceV1::Wrath,
            })
        }
        (PendingChoice::StanceChoice, ClientInput::SubmitDiscoverChoice(1)) => {
            Ok(CombatPublicActionV1::ChooseStance {
                stance: CombatPublicStanceV1::Calm,
            })
        }
        (_, ClientInput::Cancel) => Ok(CombatPublicActionV1::Cancel),
        _ => Err(CombatScenarioPolicyErrorV1::UnsupportedAction {
            scenario_id: scenario_id.to_string(),
            input: format!("{input:?}"),
        }),
    }
}

fn candidate_multiplicities(
    pile: &[CombatCard],
    candidate_uuids: &[u32],
    context: &str,
) -> Result<Vec<CombatPublicCardMultiplicityV1>, String> {
    let mut cards = Vec::with_capacity(candidate_uuids.len());
    let mut seen = BTreeSet::new();
    for uuid in candidate_uuids {
        if !seen.insert(*uuid) {
            return Err(format!("{context} repeats candidate UUID {uuid}"));
        }
        let card = pile
            .iter()
            .find(|card| card.uuid == *uuid)
            .ok_or_else(|| format!("{context} candidate UUID {uuid} is absent"))?;
        cards.push(card);
    }
    card_multiplicities(cards)
}

fn selected_multiplicities(
    scenario_id: &str,
    pile: &[CombatCard],
    candidate_uuids: &[u32],
    selected_uuids: &[u32],
    input: &ClientInput,
) -> Result<Vec<CombatPublicCardMultiplicityV1>, CombatScenarioPolicyErrorV1> {
    let mut cards = Vec::with_capacity(selected_uuids.len());
    let mut seen = BTreeSet::new();
    for uuid in selected_uuids {
        if !seen.insert(*uuid) {
            return invalid_action(scenario_id, input, "selection repeats a card UUID");
        }
        if !candidate_uuids.contains(uuid) {
            return invalid_action(
                scenario_id,
                input,
                "selection contains a non-candidate card",
            );
        }
        let card = pile.iter().find(|card| card.uuid == *uuid).ok_or_else(|| {
            CombatScenarioPolicyErrorV1::InvalidLegalAction {
                scenario_id: scenario_id.to_string(),
                input: format!("{input:?}"),
                reason: format!("selected card UUID {uuid} is absent"),
            }
        })?;
        cards.push(card);
    }
    card_multiplicities(cards).map_err(|reason| CombatScenarioPolicyErrorV1::InvalidLegalAction {
        scenario_id: scenario_id.to_string(),
        input: format!("{input:?}"),
        reason,
    })
}

fn card_multiplicities(
    cards: Vec<&CombatCard>,
) -> Result<Vec<CombatPublicCardMultiplicityV1>, String> {
    let mut counts = BTreeMap::new();
    for card in cards {
        let public = crate::ai::combat_policy_v1::combat_policy_card_v1(card);
        let count = counts.entry(public).or_insert(0usize);
        *count = count.saturating_add(1);
    }
    counts
        .into_iter()
        .map(|(card, count)| {
            Ok(CombatPublicCardMultiplicityV1 {
                card,
                count: u8::try_from(count)
                    .map_err(|_| "public card multiplicity exceeds u8".to_string())?,
            })
        })
        .collect()
}

fn generated_action(
    scenario_id: &str,
    input: &ClientInput,
    choice_kind: CombatPublicGeneratedChoiceKindV1,
    cards: &[crate::content::cards::CardId],
    option_index: usize,
    upgrades: u8,
) -> Result<CombatPublicActionV1, CombatScenarioPolicyErrorV1> {
    let card_id =
        cards
            .get(option_index)
            .ok_or_else(|| CombatScenarioPolicyErrorV1::InvalidLegalAction {
                scenario_id: scenario_id.to_string(),
                input: format!("{input:?}"),
                reason: format!("generated card option {option_index} is absent"),
            })?;
    Ok(CombatPublicActionV1::ChooseGeneratedCard {
        choice_kind,
        option_index,
        card_id: java_id(*card_id).to_string(),
        upgrades,
    })
}

fn generated_options(
    cards: &[crate::content::cards::CardId],
    upgrades: u8,
) -> Vec<CombatPublicGeneratedCardOptionV1> {
    cards
        .iter()
        .enumerate()
        .map(
            |(option_index, card_id)| CombatPublicGeneratedCardOptionV1 {
                option_index,
                card_id: java_id(*card_id).to_string(),
                upgrades,
            },
        )
        .collect()
}

fn hand_context(reason: HandSelectReason) -> CombatPublicCardSelectionContextV1 {
    CombatPublicCardSelectionContextV1::Hand {
        reason: match reason {
            HandSelectReason::Exhaust => CombatPublicHandSelectionReasonV1::Exhaust,
            HandSelectReason::Discard => CombatPublicHandSelectionReasonV1::Discard,
            HandSelectReason::Retain => CombatPublicHandSelectionReasonV1::Retain,
            HandSelectReason::PutOnDrawPile => CombatPublicHandSelectionReasonV1::PutOnDrawPile,
            HandSelectReason::PutToBottomOfDraw => {
                CombatPublicHandSelectionReasonV1::PutToBottomOfDraw
            }
            HandSelectReason::Setup => CombatPublicHandSelectionReasonV1::Setup,
            HandSelectReason::Copy { amount } => CombatPublicHandSelectionReasonV1::Copy { amount },
            HandSelectReason::Nightmare { amount } => {
                CombatPublicHandSelectionReasonV1::Nightmare { amount }
            }
            HandSelectReason::Upgrade => CombatPublicHandSelectionReasonV1::Upgrade,
            HandSelectReason::GamblingChip => CombatPublicHandSelectionReasonV1::GamblingChip,
            HandSelectReason::Recycle => CombatPublicHandSelectionReasonV1::Recycle,
        },
    }
}

fn grid_context(
    source_pile: PileType,
    reason: GridSelectReason,
) -> CombatPublicCardSelectionContextV1 {
    CombatPublicCardSelectionContextV1::Grid {
        source_pile: public_pile(source_pile),
        reason: match reason {
            GridSelectReason::MoveToDrawPile => CombatPublicGridSelectionReasonV1::MoveToDrawPile,
            GridSelectReason::Exhume { upgrade } => {
                CombatPublicGridSelectionReasonV1::Exhume { upgrade }
            }
            GridSelectReason::DrawPileToHand => CombatPublicGridSelectionReasonV1::DrawPileToHand,
            GridSelectReason::SkillFromDeckToHand => {
                CombatPublicGridSelectionReasonV1::SkillFromDeckToHand
            }
            GridSelectReason::AttackFromDeckToHand => {
                CombatPublicGridSelectionReasonV1::AttackFromDeckToHand
            }
            GridSelectReason::DiscardToHand => CombatPublicGridSelectionReasonV1::DiscardToHand,
            GridSelectReason::DiscardToHandNoCostChange => {
                CombatPublicGridSelectionReasonV1::DiscardToHandNoCostChange
            }
            GridSelectReason::DiscardToHandRetain => {
                CombatPublicGridSelectionReasonV1::DiscardToHandRetain
            }
            GridSelectReason::Omniscience { play_amount } => {
                CombatPublicGridSelectionReasonV1::Omniscience { play_amount }
            }
        },
    }
}

fn public_pile(pile: PileType) -> CombatPublicPileV1 {
    match pile {
        PileType::Draw => CombatPublicPileV1::Draw,
        PileType::Discard => CombatPublicPileV1::Discard,
        PileType::Exhaust => CombatPublicPileV1::Exhaust,
        PileType::Hand => CombatPublicPileV1::Hand,
        PileType::Limbo => CombatPublicPileV1::Limbo,
        PileType::MasterDeck => CombatPublicPileV1::MasterDeck,
    }
}

fn source_pile_cards(combat: &CombatState, pile: PileType) -> &[CombatCard] {
    match pile {
        PileType::Draw => &combat.zones.draw_pile,
        PileType::Discard => &combat.zones.discard_pile,
        PileType::Exhaust => &combat.zones.exhaust_pile,
        PileType::Hand => &combat.zones.hand,
        PileType::Limbo => &combat.zones.limbo,
        PileType::MasterDeck => &combat.meta.master_deck_snapshot,
    }
}

fn card_destination(destination: CardDestination) -> CombatPublicCardDestinationV1 {
    match destination {
        CardDestination::Hand => CombatPublicCardDestinationV1::Hand,
        CardDestination::DrawPileRandom => CombatPublicCardDestinationV1::DrawPileRandom,
    }
}

fn invalid_action<T>(
    scenario_id: &str,
    input: &ClientInput,
    reason: &str,
) -> Result<T, CombatScenarioPolicyErrorV1> {
    Err(CombatScenarioPolicyErrorV1::InvalidLegalAction {
        scenario_id: scenario_id.to_string(),
        input: format!("{input:?}"),
        reason: reason.to_string(),
    })
}
