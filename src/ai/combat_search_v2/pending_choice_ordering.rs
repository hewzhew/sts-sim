use super::*;
use crate::content::cards::{self, CardId, CardType};
use crate::runtime::combat::CombatCard;
use crate::state::core::{GridSelectReason, HandSelectReason, PendingChoice, PileType};

const UNDESIRABLE_CARD_KEEP_VALUE: i32 = -1_000;
const UNDESIRABLE_CARD_REMOVAL_VALUE: i32 = 1_000;
const ATTACK_BASE_KEEP_VALUE: i32 = 300;
const SKILL_BASE_KEEP_VALUE: i32 = 275;
const POWER_BASE_KEEP_VALUE: i32 = 325;
const DAMAGE_KEEP_VALUE_FACTOR: i32 = 4;
const BLOCK_KEEP_VALUE_FACTOR: i32 = 4;
const MAGIC_KEEP_VALUE_FACTOR: i32 = 2;
const POWER_MAGIC_KEEP_VALUE_FACTOR: i32 = 3;
const COST_KEEP_VALUE_PENALTY: i32 = 10;
const RECYCLE_ENERGY_FACTOR: i32 = 10;
const SETUP_EXPENSIVE_CARD_BONUS: i32 = 25;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum PendingChoiceOrderingRole {
    ValueSelection,
    RemovalSelection,
    NeutralSelection,
    Cancel,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct PendingChoiceOrderingHint {
    pub(super) primary: i32,
    pub(super) secondary: i32,
    pub(super) selected_count_tiebreak: i32,
    pub(super) role: PendingChoiceOrderingRole,
}

#[derive(Clone, Copy, Debug, Default)]
struct CardSelectionFacts {
    keep_value: i32,
    removal_value: i32,
}

pub(super) fn pending_choice_ordering_hint(
    engine: &EngineState,
    combat: &CombatState,
    input: &ClientInput,
) -> Option<PendingChoiceOrderingHint> {
    let EngineState::PendingChoice(choice) = engine else {
        return None;
    };

    match (choice, input) {
        (_, ClientInput::Cancel) => Some(PendingChoiceOrderingHint {
            role: PendingChoiceOrderingRole::Cancel,
            primary: 0,
            secondary: 0,
            selected_count_tiebreak: 0,
        }),
        (
            PendingChoice::HandSelect {
                candidate_uuids,
                reason,
                ..
            },
            ClientInput::SubmitHandSelect(uuids),
        ) if selection_is_subset(uuids, candidate_uuids) => {
            let cards = uuids
                .iter()
                .filter_map(|uuid| find_card_by_uuid(&combat.zones.hand, *uuid))
                .collect::<Vec<_>>();
            Some(selection_hint_for_hand_reason(*reason, &cards, uuids.len()))
        }
        (
            PendingChoice::GridSelect {
                source_pile,
                candidate_uuids,
                reason,
                ..
            },
            ClientInput::SubmitGridSelect(uuids),
        ) if selection_is_subset(uuids, candidate_uuids) => {
            let cards = uuids
                .iter()
                .filter_map(|uuid| find_card_by_uuid(pile_cards(combat, *source_pile), *uuid))
                .collect::<Vec<_>>();
            Some(selection_hint_for_grid_reason(*reason, &cards, uuids.len()))
        }
        (PendingChoice::ScrySelect { cards, .. }, ClientInput::SubmitScryDiscard(indices)) => {
            let selected_cards = indices
                .iter()
                .filter_map(|idx| cards.get(*idx).copied())
                .collect::<Vec<_>>();
            Some(removal_selection_hint_from_card_ids(
                &selected_cards,
                indices.len(),
            ))
        }
        (PendingChoice::DiscoverySelect(choice), ClientInput::SubmitDiscoverChoice(idx))
            if *idx < choice.cards.len() =>
        {
            Some(value_selection_hint_from_card_id(choice.cards[*idx], 1))
        }
        (PendingChoice::CardRewardSelect { cards, .. }, ClientInput::SubmitDiscoverChoice(idx))
            if *idx < cards.len() =>
        {
            Some(value_selection_hint_from_card_id(cards[*idx], 1))
        }
        (
            PendingChoice::ForeignInfluenceSelect { cards, .. },
            ClientInput::SubmitDiscoverChoice(idx),
        ) if *idx < cards.len() => Some(value_selection_hint_from_card_id(cards[*idx], 1)),
        (PendingChoice::ChooseOneSelect { choices }, ClientInput::SubmitDiscoverChoice(idx))
            if *idx < choices.len() =>
        {
            Some(value_selection_hint_from_card_id(choices[*idx].card_id, 1))
        }
        (PendingChoice::StanceChoice, ClientInput::SubmitDiscoverChoice(idx)) if *idx <= 1 => {
            Some(PendingChoiceOrderingHint {
                role: PendingChoiceOrderingRole::NeutralSelection,
                primary: -(*idx as i32),
                secondary: 0,
                selected_count_tiebreak: -1,
            })
        }
        _ => None,
    }
}

fn selection_hint_for_hand_reason(
    reason: HandSelectReason,
    cards: &[&CombatCard],
    selected_count: usize,
) -> PendingChoiceOrderingHint {
    match reason {
        HandSelectReason::Discard | HandSelectReason::Exhaust | HandSelectReason::GamblingChip => {
            removal_selection_hint(cards, selected_count)
        }
        HandSelectReason::Recycle => recycle_selection_hint(cards, selected_count),
        HandSelectReason::Upgrade
        | HandSelectReason::Copy { .. }
        | HandSelectReason::Nightmare { .. }
        | HandSelectReason::Retain => value_selection_hint(cards, selected_count),
        HandSelectReason::PutOnDrawPile
        | HandSelectReason::PutToBottomOfDraw
        | HandSelectReason::Setup => draw_pile_setup_selection_hint(cards, selected_count),
    }
}

fn selection_hint_for_grid_reason(
    reason: GridSelectReason,
    cards: &[&CombatCard],
    selected_count: usize,
) -> PendingChoiceOrderingHint {
    match reason {
        GridSelectReason::MoveToDrawPile
        | GridSelectReason::DrawPileToHand
        | GridSelectReason::SkillFromDeckToHand
        | GridSelectReason::AttackFromDeckToHand
        | GridSelectReason::DiscardToHand
        | GridSelectReason::DiscardToHandNoCostChange
        | GridSelectReason::DiscardToHandRetain
        | GridSelectReason::Exhume { .. }
        | GridSelectReason::Omniscience { .. } => value_selection_hint(cards, selected_count),
    }
}

fn value_selection_hint(cards: &[&CombatCard], selected_count: usize) -> PendingChoiceOrderingHint {
    let facts = aggregate_card_facts(cards.iter().copied().map(CardSelectionFacts::from_card));
    PendingChoiceOrderingHint {
        role: PendingChoiceOrderingRole::ValueSelection,
        primary: facts.keep_value,
        secondary: -facts.removal_value,
        selected_count_tiebreak: -(selected_count as i32),
    }
}

fn value_selection_hint_from_card_id(
    card_id: CardId,
    selected_count: usize,
) -> PendingChoiceOrderingHint {
    let facts = CardSelectionFacts::from_card_id(card_id);
    PendingChoiceOrderingHint {
        role: PendingChoiceOrderingRole::ValueSelection,
        primary: facts.keep_value,
        secondary: -facts.removal_value,
        selected_count_tiebreak: -(selected_count as i32),
    }
}

fn removal_selection_hint(
    cards: &[&CombatCard],
    selected_count: usize,
) -> PendingChoiceOrderingHint {
    let facts = aggregate_card_facts(cards.iter().copied().map(CardSelectionFacts::from_card));
    PendingChoiceOrderingHint {
        role: PendingChoiceOrderingRole::RemovalSelection,
        primary: facts.removal_value,
        secondary: -facts.keep_value,
        selected_count_tiebreak: -(selected_count as i32),
    }
}

fn removal_selection_hint_from_card_ids(
    card_ids: &[CardId],
    selected_count: usize,
) -> PendingChoiceOrderingHint {
    let facts = aggregate_card_facts(
        card_ids
            .iter()
            .copied()
            .map(CardSelectionFacts::from_card_id),
    );
    PendingChoiceOrderingHint {
        role: PendingChoiceOrderingRole::RemovalSelection,
        primary: facts.removal_value,
        secondary: -facts.keep_value,
        selected_count_tiebreak: -(selected_count as i32),
    }
}

fn recycle_selection_hint(
    cards: &[&CombatCard],
    selected_count: usize,
) -> PendingChoiceOrderingHint {
    let facts = aggregate_card_facts(cards.iter().copied().map(CardSelectionFacts::from_card));
    let energy_return = cards
        .iter()
        .map(|card| card.combat_cost_without_turn_override_java().max(0))
        .sum::<i32>();
    PendingChoiceOrderingHint {
        role: PendingChoiceOrderingRole::RemovalSelection,
        primary: energy_return
            .saturating_mul(RECYCLE_ENERGY_FACTOR)
            .saturating_add(facts.removal_value),
        secondary: -facts.keep_value,
        selected_count_tiebreak: -(selected_count as i32),
    }
}

fn draw_pile_setup_selection_hint(
    cards: &[&CombatCard],
    selected_count: usize,
) -> PendingChoiceOrderingHint {
    let facts = aggregate_card_facts(cards.iter().copied().map(CardSelectionFacts::from_card));
    let currently_expensive = cards
        .iter()
        .filter(|card| card.cost_for_turn_java() > 0)
        .count() as i32;
    PendingChoiceOrderingHint {
        role: PendingChoiceOrderingRole::ValueSelection,
        primary: facts
            .keep_value
            .saturating_add(currently_expensive.saturating_mul(SETUP_EXPENSIVE_CARD_BONUS)),
        secondary: -facts.removal_value,
        selected_count_tiebreak: -(selected_count as i32),
    }
}

fn aggregate_card_facts(facts: impl Iterator<Item = CardSelectionFacts>) -> CardSelectionFacts {
    facts.fold(CardSelectionFacts::default(), |mut acc, fact| {
        acc.keep_value = acc.keep_value.saturating_add(fact.keep_value);
        acc.removal_value = acc.removal_value.saturating_add(fact.removal_value);
        acc
    })
}

fn selection_is_subset(selected: &[u32], candidates: &[u32]) -> bool {
    selected.iter().all(|uuid| candidates.contains(uuid))
}

fn find_card_by_uuid(cards: &[CombatCard], uuid: u32) -> Option<&CombatCard> {
    cards.iter().find(|card| card.uuid == uuid)
}

fn pile_cards(combat: &CombatState, pile: PileType) -> &[CombatCard] {
    match pile {
        PileType::Draw => &combat.zones.draw_pile,
        PileType::Discard => &combat.zones.discard_pile,
        PileType::Exhaust => &combat.zones.exhaust_pile,
        PileType::Hand => &combat.zones.hand,
        PileType::Limbo => &combat.zones.limbo,
        PileType::MasterDeck => &[],
    }
}

impl CardSelectionFacts {
    fn from_card(card: &CombatCard) -> Self {
        let def = cards::get_card_definition(card.id);
        let damage = card
            .base_damage_override
            .unwrap_or(def.base_damage + def.upgrade_damage * card.upgrades as i32)
            .max(0);
        let block = card
            .base_block_override
            .unwrap_or(def.base_block + def.upgrade_block * card.upgrades as i32)
            .max(0);
        let magic = (def.base_magic + def.upgrade_magic * card.upgrades as i32).max(0);
        let cost = card.cost_for_turn_java().max(0);
        Self::from_parts(def.card_type, damage, block, magic, cost)
    }

    fn from_card_id(card_id: CardId) -> Self {
        let def = cards::get_card_definition(card_id);
        Self::from_parts(
            def.card_type,
            def.base_damage.max(0),
            def.base_block.max(0),
            def.base_magic.max(0),
            (def.cost as i32).max(0),
        )
    }

    fn from_parts(card_type: CardType, damage: i32, block: i32, magic: i32, cost: i32) -> Self {
        match card_type {
            CardType::Status | CardType::Curse => Self {
                keep_value: UNDESIRABLE_CARD_KEEP_VALUE,
                removal_value: UNDESIRABLE_CARD_REMOVAL_VALUE,
            },
            CardType::Attack => Self {
                keep_value: ATTACK_BASE_KEEP_VALUE
                    + damage.saturating_mul(DAMAGE_KEEP_VALUE_FACTOR)
                    - cost.saturating_mul(COST_KEEP_VALUE_PENALTY),
                removal_value: 0,
            },
            CardType::Skill => Self {
                keep_value: SKILL_BASE_KEEP_VALUE
                    + block.saturating_mul(BLOCK_KEEP_VALUE_FACTOR)
                    + magic.saturating_mul(MAGIC_KEEP_VALUE_FACTOR)
                    - cost.saturating_mul(COST_KEEP_VALUE_PENALTY),
                removal_value: 0,
            },
            CardType::Power => Self {
                keep_value: POWER_BASE_KEEP_VALUE
                    + magic.saturating_mul(POWER_MAGIC_KEEP_VALUE_FACTOR)
                    - cost.saturating_mul(COST_KEEP_VALUE_PENALTY),
                removal_value: 0,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::combat::CombatCard;
    use crate::test_support::blank_test_combat;

    #[test]
    fn move_to_draw_prefers_higher_value_card() {
        let mut combat = blank_test_combat();
        combat.zones.discard_pile = vec![
            CombatCard::new(CardId::Strike, 10),
            CombatCard::new(CardId::Carnage, 20),
        ];
        let engine = EngineState::PendingChoice(PendingChoice::GridSelect {
            source_pile: PileType::Discard,
            candidate_uuids: vec![10, 20],
            min_cards: 1,
            max_cards: 1,
            can_cancel: false,
            reason: GridSelectReason::MoveToDrawPile,
        });

        let strike = pending_choice_ordering_hint(
            &engine,
            &combat,
            &ClientInput::SubmitGridSelect(vec![10]),
        )
        .expect("strike candidate should rank");
        let carnage = pending_choice_ordering_hint(
            &engine,
            &combat,
            &ClientInput::SubmitGridSelect(vec![20]),
        )
        .expect("carnage candidate should rank");

        assert!(carnage.primary > strike.primary);
    }

    #[test]
    fn scry_discard_prefers_status_over_empty_selection() {
        let combat = blank_test_combat();
        let engine = EngineState::PendingChoice(PendingChoice::ScrySelect {
            cards: vec![CardId::Slimed, CardId::Bash],
            card_uuids: vec![10, 20],
        });

        let keep_all =
            pending_choice_ordering_hint(&engine, &combat, &ClientInput::SubmitScryDiscard(vec![]))
                .expect("empty scry discard should rank");
        let discard_slimed = pending_choice_ordering_hint(
            &engine,
            &combat,
            &ClientInput::SubmitScryDiscard(vec![0]),
        )
        .expect("slimed scry discard should rank");

        assert!(discard_slimed.primary > keep_all.primary);
        assert_eq!(
            discard_slimed.role,
            PendingChoiceOrderingRole::RemovalSelection
        );
    }

    #[test]
    fn cancel_is_explicitly_low_priority_but_still_ranked() {
        let combat = blank_test_combat();
        let engine = EngineState::PendingChoice(PendingChoice::DiscoverySelect(
            crate::state::core::DiscoveryChoiceState {
                cards: vec![CardId::Carnage],
                colorless: false,
                card_type: None,
                amount: 1,
                can_skip: true,
            },
        ));

        let cancel = pending_choice_ordering_hint(&engine, &combat, &ClientInput::Cancel)
            .expect("cancel should rank");
        let pick =
            pending_choice_ordering_hint(&engine, &combat, &ClientInput::SubmitDiscoverChoice(0))
                .expect("pick should rank");

        assert_eq!(cancel.role, PendingChoiceOrderingRole::Cancel);
        assert!(pick.primary > cancel.primary);
    }
}
