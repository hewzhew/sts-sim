use crate::content::potions::get_potion_definition;
use crate::runtime::combat::CombatCard;
use crate::sim::combat_legal_actions::get_legal_moves;
use crate::state::core::{CampfireChoice, ClientInput, EngineState, PendingChoice, PileType};
use crate::state::events::{EventOption, EventOptionTransition};
use crate::state::rewards::{BossRelicChoiceState, RewardState};
use std::collections::BTreeMap;

use super::labels::{
    candidate, clean_event_label, combat_card_label, monster_name, reward_card_label,
    reward_item_label, room_type_label, shop_block_note, unavailable_candidate,
};
use super::{CandidateResolution, DecisionCandidate, RunControlSession};

pub(super) fn decision_candidates(session: &RunControlSession) -> Vec<DecisionCandidate> {
    match &session.engine_state {
        EngineState::EventRoom => event_candidates(session),
        EngineState::MapNavigation | EngineState::MapOverlay { .. } => map_candidates(session),
        EngineState::RewardScreen(reward) => reward_candidates(session, reward),
        EngineState::TreasureRoom(_) => {
            vec![candidate(
                "open",
                "Open chest",
                ClientInput::OpenChest,
                Some("routine"),
            )]
        }
        EngineState::Campfire => campfire_candidates(session),
        EngineState::Shop(shop) => shop_candidates(session, shop),
        EngineState::CombatStart(_) => Vec::new(),
        EngineState::CombatPlayerTurn
        | EngineState::CombatProcessing
        | EngineState::PendingChoice(_) => combat_candidates(session),
        EngineState::RunPendingChoice(choice) => run_choice_candidates(session, choice),
        EngineState::BossRelicSelect(choice) => boss_relic_candidates(choice),
        EngineState::GameOver(_) => Vec::new(),
    }
}

fn event_candidates(session: &RunControlSession) -> Vec<DecisionCandidate> {
    let options = crate::engine::event_handler::get_event_options(&session.run_state);
    options
        .iter()
        .enumerate()
        .map(|(idx, option)| {
            let label = clean_event_label(&option.ui.text);
            let resolution = CandidateResolution::from_event_option(option);
            let note = event_option_note(option, options.len(), resolution.as_ref());
            if option.ui.disabled {
                unavailable_candidate(
                    idx.to_string(),
                    label,
                    option
                        .ui
                        .disabled_reason
                        .clone()
                        .unwrap_or_else(|| "disabled".to_string()),
                    note,
                )
            } else {
                let mut candidate =
                    candidate(idx.to_string(), label, ClientInput::EventChoice(idx), note);
                candidate.resolution = resolution;
                candidate
            }
        })
        .collect()
}

fn event_option_note(
    option: &EventOption,
    option_count: usize,
    resolution: Option<&CandidateResolution>,
) -> Option<String> {
    if option.ui.disabled {
        return Some(format!(
            "locked: {}",
            option.ui.disabled_reason.as_deref().unwrap_or("disabled")
        ));
    }
    if option_count == 1
        && matches!(
            option.semantics.transition,
            EventOptionTransition::AdvanceScreen | EventOptionTransition::Complete
        )
    {
        return Some("routine".to_string());
    }
    if let Some(note) = resolution.and_then(CandidateResolution::main_note) {
        return Some(note);
    }
    match option.semantics.transition {
        EventOptionTransition::OpenSelection(kind) => {
            Some(format!("opens {} selection", selection_kind_label(kind)))
        }
        EventOptionTransition::OpenReward => Some("opens reward".to_string()),
        EventOptionTransition::StartCombat => Some("starts combat".to_string()),
        EventOptionTransition::AdvanceScreen => Some("advances event".to_string()),
        EventOptionTransition::Complete => Some("leaves event".to_string()),
        EventOptionTransition::None => None,
    }
}

fn selection_kind_label(kind: crate::state::events::EventSelectionKind) -> &'static str {
    match kind {
        crate::state::events::EventSelectionKind::None => "unknown",
        crate::state::events::EventSelectionKind::RemoveCard => "remove card",
        crate::state::events::EventSelectionKind::UpgradeCard => "upgrade card",
        crate::state::events::EventSelectionKind::TransformCard => "transform card",
        crate::state::events::EventSelectionKind::DuplicateCard => "duplicate card",
        crate::state::events::EventSelectionKind::OfferCard => "offer card",
    }
}

fn map_candidates(session: &RunControlSession) -> Vec<DecisionCandidate> {
    let target_y = if session.run_state.map.current_y == -1 {
        0
    } else {
        session.run_state.map.current_y + 1
    };
    if target_y == 15 {
        return with_map_overlay_back_candidate(
            session,
            vec![candidate(
                "0",
                "Boss room",
                ClientInput::SelectMapNode(0),
                Some("boss"),
            )],
        );
    }
    let Some(row) = session.run_state.map.graph.get(target_y as usize) else {
        return with_map_overlay_back_candidate(session, Vec::new());
    };
    let candidates = row
        .iter()
        .filter(|node| session.run_state.map.can_travel_to(node.x, node.y, false))
        .map(|node| {
            candidate(
                node.x.to_string(),
                format!("y={} {}", node.y, room_type_label(node.class)),
                ClientInput::SelectMapNode(node.x as usize),
                node.has_emerald_key.then_some("emerald elite"),
            )
        })
        .collect();
    with_map_overlay_back_candidate(session, candidates)
}

fn with_map_overlay_back_candidate(
    session: &RunControlSession,
    mut candidates: Vec<DecisionCandidate>,
) -> Vec<DecisionCandidate> {
    if matches!(session.engine_state, EngineState::MapOverlay { .. }) {
        candidates.push(candidate(
            "back",
            "Back to reward screen",
            ClientInput::Cancel,
            Some("unclaimed rewards remain"),
        ));
    }
    candidates
}

fn reward_candidates(session: &RunControlSession, reward: &RewardState) -> Vec<DecisionCandidate> {
    if let Some(cards) = reward.pending_card_choice.as_ref() {
        let mut candidates = cards
            .iter()
            .enumerate()
            .map(|(idx, card)| {
                let mut candidate = candidate(
                    idx.to_string(),
                    reward_card_label(card.id, card.upgrades),
                    ClientInput::SelectCard(idx),
                    None::<String>,
                );
                candidate.resolution = Some(CandidateResolution::from_reward_card(card));
                candidate
            })
            .collect::<Vec<_>>();
        candidates.push(candidate(
            "back",
            "Back to reward screen",
            ClientInput::Cancel,
            Some("card reward remains"),
        ));
        return candidates;
    }

    let mut candidates = reward
        .items
        .iter()
        .enumerate()
        .map(|(idx, item)| {
            let mut candidate = candidate(
                idx.to_string(),
                reward_item_label(item),
                ClientInput::ClaimReward(idx),
                None::<String>,
            );
            candidate.resolution =
                CandidateResolution::from_reward_item(item, reward, &session.run_state);
            candidate
        })
        .collect::<Vec<_>>();
    if reward.skippable {
        let (label, note) = if reward.items.is_empty() {
            ("Leave reward screen", Some("routine"))
        } else {
            (
                "Open map preview",
                Some("unclaimed rewards remain until a path is chosen"),
            )
        };
        candidates.push(candidate("skip", label, ClientInput::Proceed, note));
    }
    candidates
}

fn campfire_candidates(session: &RunControlSession) -> Vec<DecisionCandidate> {
    let mut candidates = vec![candidate(
        "rest",
        "Rest",
        ClientInput::CampfireOption(CampfireChoice::Rest),
        None::<String>,
    )];
    let upgradeable = session
        .run_state
        .master_deck
        .iter()
        .enumerate()
        .filter(|(_, card)| crate::state::core::master_deck_card_can_upgrade(card))
        .map(|(idx, card)| {
            candidate(
                format!("smith-{idx}"),
                format!("Smith {}", combat_card_label(card)),
                ClientInput::CampfireOption(CampfireChoice::Smith(idx)),
                None::<String>,
            )
        });
    candidates.extend(upgradeable);
    candidates.push(candidate(
        "recall",
        "Recall ruby key",
        ClientInput::CampfireOption(CampfireChoice::Recall),
        None::<String>,
    ));
    candidates
}

fn shop_candidates(
    session: &RunControlSession,
    shop: &crate::state::shop::ShopState,
) -> Vec<DecisionCandidate> {
    let mut candidates = Vec::new();
    candidates.extend(shop.cards.iter().enumerate().map(|(idx, card)| {
        let label = format!(
            "{} | {} gold",
            reward_card_label(card.card_id, card.upgrades),
            card.price
        );
        let note = shop_block_note(card.can_buy, card.blocked_reason.as_deref());
        if card.can_buy {
            candidate(
                format!("card-{idx}"),
                label,
                ClientInput::BuyCard(idx),
                note,
            )
        } else {
            unavailable_candidate(
                format!("card-{idx}"),
                label,
                card.blocked_reason
                    .clone()
                    .unwrap_or_else(|| "cannot buy".to_string()),
                note,
            )
        }
    }));
    candidates.extend(shop.relics.iter().enumerate().map(|(idx, relic)| {
        let label = format!("{:?} | {} gold", relic.relic_id, relic.price);
        let note = shop_block_note(relic.can_buy, relic.blocked_reason.as_deref());
        if relic.can_buy {
            candidate(
                format!("relic-{idx}"),
                label,
                ClientInput::BuyRelic(idx),
                note,
            )
        } else {
            unavailable_candidate(
                format!("relic-{idx}"),
                label,
                relic
                    .blocked_reason
                    .clone()
                    .unwrap_or_else(|| "cannot buy".to_string()),
                note,
            )
        }
    }));
    candidates.extend(shop.potions.iter().enumerate().map(|(idx, potion)| {
        let label = format!(
            "{} | {} gold",
            get_potion_definition(potion.potion_id).name,
            potion.price
        );
        let note = shop_block_note(potion.can_buy, potion.blocked_reason.as_deref());
        if potion.can_buy {
            candidate(
                format!("potion-{idx}"),
                label,
                ClientInput::BuyPotion(idx),
                note,
            )
        } else {
            unavailable_candidate(
                format!("potion-{idx}"),
                label,
                potion
                    .blocked_reason
                    .clone()
                    .unwrap_or_else(|| "cannot buy".to_string()),
                note,
            )
        }
    }));
    let purge_block = shop_purge_block_reason(session, shop);
    if purge_block.is_none() {
        candidates.push(candidate(
            "purge",
            format!("Remove card | {} gold", shop.purge_cost),
            "purge <deck_idx>",
            None::<String>,
        ));
    } else {
        candidates.push(unavailable_candidate(
            "purge",
            format!("Remove card | {} gold", shop.purge_cost),
            purge_block.unwrap_or("locked"),
            purge_block.map(|reason| format!("locked: {reason}")),
        ));
    }
    candidates.push(candidate(
        "leave",
        "Leave shop",
        ClientInput::Proceed,
        None::<String>,
    ));
    candidates
}

fn shop_purge_block_reason(
    session: &RunControlSession,
    shop: &crate::state::shop::ShopState,
) -> Option<&'static str> {
    if !shop.purge_available {
        return Some("already used");
    }
    if session.run_state.gold < shop.purge_cost {
        return Some("not enough gold");
    }
    let has_eligible_card = session.run_state.master_deck.iter().any(|card| {
        crate::state::core::master_deck_card_is_purgeable(card)
            && !crate::state::core::master_deck_card_is_bottled(card, &session.run_state.relics)
    });
    if !has_eligible_card {
        return Some("no eligible cards");
    }
    None
}

fn combat_candidates(session: &RunControlSession) -> Vec<DecisionCandidate> {
    let Ok(position) = session.current_combat_position_for_actions() else {
        return Vec::new();
    };
    if let EngineState::PendingChoice(choice) = &position.engine {
        return pending_choice_candidates(choice, &position.combat);
    }
    let legal_moves = get_legal_moves(&position.engine, &position.combat);
    let mut playable: BTreeMap<usize, Vec<Option<usize>>> = BTreeMap::new();
    let mut end_turn = false;
    for action in &legal_moves {
        match action {
            ClientInput::PlayCard { card_index, target } => {
                playable.entry(*card_index).or_default().push(*target);
            }
            ClientInput::EndTurn => end_turn = true,
            _ => {}
        }
    }

    let mut candidates = Vec::new();
    for (card_index, targets) in playable {
        let Some(card) = position.combat.zones.hand.get(card_index) else {
            continue;
        };
        if targets.len() == 1 {
            let target = targets[0];
            let label = match target {
                Some(target_id) => format!(
                    "Play {} -> {}",
                    combat_card_label(card),
                    combat_target_label(&position.combat, target_id)
                ),
                None => format!("Play {}", combat_card_label(card)),
            };
            candidates.push(candidate(
                card_index.to_string(),
                label,
                ClientInput::PlayCard { card_index, target },
                None::<String>,
            ));
        } else {
            for target in targets {
                let Some(target_id) = target else {
                    continue;
                };
                let Some(slot) = combat_target_slot(&position.combat, target_id) else {
                    continue;
                };
                candidates.push(candidate(
                    format!("{card_index}.{slot}"),
                    format!(
                        "Play {} -> {}",
                        combat_card_label(card),
                        combat_target_label(&position.combat, target_id)
                    ),
                    ClientInput::PlayCard {
                        card_index,
                        target: Some(target_id),
                    },
                    None::<String>,
                ));
            }
        }
    }
    if end_turn {
        candidates.push(candidate(
            "end",
            "End turn",
            ClientInput::EndTurn,
            None::<String>,
        ));
    }
    candidates
}

fn pending_choice_candidates(
    choice: &PendingChoice,
    combat: &crate::runtime::combat::CombatState,
) -> Vec<DecisionCandidate> {
    let legal_moves = get_legal_moves(&EngineState::PendingChoice(choice.clone()), combat);
    legal_moves
        .into_iter()
        .enumerate()
        .map(|(idx, input)| {
            let label = pending_choice_input_label(choice, combat, &input);
            candidate(idx.to_string(), label, input, None::<String>)
        })
        .collect()
}

fn pending_choice_input_label(
    choice: &PendingChoice,
    combat: &crate::runtime::combat::CombatState,
    input: &ClientInput,
) -> String {
    match (choice, input) {
        (
            PendingChoice::GridSelect {
                source_pile,
                reason,
                ..
            },
            ClientInput::SubmitGridSelect(uuids),
        ) => format!(
            "{} {}",
            grid_reason_verb(*reason),
            selected_card_labels(grid_source_cards(combat, *source_pile), uuids).join(", ")
        ),
        (PendingChoice::HandSelect { reason, .. }, ClientInput::SubmitHandSelect(uuids)) => {
            format!(
                "{} {}",
                hand_reason_verb(*reason),
                selected_card_labels(&combat.zones.hand, uuids).join(", ")
            )
        }
        (PendingChoice::DiscoverySelect(choice), ClientInput::SubmitDiscoverChoice(idx)) => choice
            .cards
            .get(*idx)
            .map(|card| format!("Choose {}", reward_card_label(*card, 0)))
            .unwrap_or_else(|| format!("Choose {idx}")),
        (PendingChoice::ScrySelect { cards, .. }, ClientInput::SubmitScryDiscard(indices)) => {
            if indices.is_empty() {
                "Keep all".to_string()
            } else {
                let selected = indices
                    .iter()
                    .map(|idx| {
                        cards
                            .get(*idx)
                            .map(|card| reward_card_label(*card, 0))
                            .unwrap_or_else(|| format!("card {idx}"))
                    })
                    .collect::<Vec<_>>();
                format!("Discard {}", selected.join(", "))
            }
        }
        (PendingChoice::CardRewardSelect { cards, .. }, ClientInput::SubmitDiscoverChoice(idx)) => {
            cards
                .get(*idx)
                .map(|card| format!("Choose {}", reward_card_label(*card, 0)))
                .unwrap_or_else(|| format!("Choose {idx}"))
        }
        (
            PendingChoice::ForeignInfluenceSelect { cards, upgraded },
            ClientInput::SubmitDiscoverChoice(idx),
        ) => cards
            .get(*idx)
            .map(|card| format!("Choose {}", reward_card_label(*card, u8::from(*upgraded))))
            .unwrap_or_else(|| format!("Choose {idx}")),
        (PendingChoice::ChooseOneSelect { choices }, ClientInput::SubmitDiscoverChoice(idx)) => {
            choices
                .get(*idx)
                .map(|choice| {
                    format!(
                        "Choose {}",
                        reward_card_label(choice.card_id, choice.upgrades)
                    )
                })
                .unwrap_or_else(|| format!("Choose {idx}"))
        }
        (PendingChoice::StanceChoice, ClientInput::SubmitDiscoverChoice(0)) => {
            "Choose Wrath".to_string()
        }
        (PendingChoice::StanceChoice, ClientInput::SubmitDiscoverChoice(1)) => {
            "Choose Calm".to_string()
        }
        (_, ClientInput::Cancel) => "Cancel".to_string(),
        (_, other) => crate::eval::run_control::view_model::client_input_hint(other),
    }
}

fn selected_card_labels(cards: &[CombatCard], uuids: &[u32]) -> Vec<String> {
    if uuids.is_empty() {
        return vec!["nothing".to_string()];
    }
    uuids
        .iter()
        .map(|uuid| {
            cards
                .iter()
                .find(|card| card.uuid == *uuid)
                .map(combat_card_label)
                .unwrap_or_else(|| format!("card uuid {uuid}"))
        })
        .collect()
}

fn grid_source_cards(
    combat: &crate::runtime::combat::CombatState,
    source_pile: PileType,
) -> &[CombatCard] {
    match source_pile {
        PileType::Draw => &combat.zones.draw_pile,
        PileType::Discard => &combat.zones.discard_pile,
        PileType::Exhaust => &combat.zones.exhaust_pile,
        PileType::Hand => &combat.zones.hand,
        PileType::Limbo => &combat.zones.limbo,
        PileType::MasterDeck => &combat.meta.master_deck_snapshot,
    }
}

fn grid_reason_verb(reason: crate::state::GridSelectReason) -> &'static str {
    match reason {
        crate::state::GridSelectReason::MoveToDrawPile => "Put on top of draw pile:",
        crate::state::GridSelectReason::Exhume { .. } => "Exhume:",
        crate::state::GridSelectReason::DrawPileToHand => "Add to hand:",
        crate::state::GridSelectReason::SkillFromDeckToHand => "Add skill to hand:",
        crate::state::GridSelectReason::AttackFromDeckToHand => "Add attack to hand:",
        crate::state::GridSelectReason::DiscardToHand => "Return to hand:",
        crate::state::GridSelectReason::DiscardToHandNoCostChange => "Return to hand:",
        crate::state::GridSelectReason::DiscardToHandRetain => "Return to hand and retain:",
        crate::state::GridSelectReason::Omniscience { .. } => "Choose for Omniscience:",
    }
}

fn hand_reason_verb(reason: crate::state::HandSelectReason) -> &'static str {
    match reason {
        crate::state::HandSelectReason::Exhaust => "Exhaust:",
        crate::state::HandSelectReason::Discard => "Discard:",
        crate::state::HandSelectReason::Retain => "Retain:",
        crate::state::HandSelectReason::PutOnDrawPile => "Put on top of draw pile:",
        crate::state::HandSelectReason::PutToBottomOfDraw => "Put on bottom of draw pile:",
        crate::state::HandSelectReason::Setup => "Set up:",
        crate::state::HandSelectReason::Copy { .. } => "Copy:",
        crate::state::HandSelectReason::Nightmare { .. } => "Nightmare:",
        crate::state::HandSelectReason::Upgrade => "Upgrade:",
        crate::state::HandSelectReason::GamblingChip => "Discard for Gambling Chip:",
        crate::state::HandSelectReason::Recycle => "Recycle:",
    }
}

fn combat_target_slot(
    combat: &crate::runtime::combat::CombatState,
    target_id: usize,
) -> Option<u8> {
    combat
        .entities
        .monsters
        .iter()
        .find(|monster| monster.id == target_id)
        .map(|monster| monster.slot)
}

fn combat_target_label(combat: &crate::runtime::combat::CombatState, target_id: usize) -> String {
    combat
        .entities
        .monsters
        .iter()
        .find(|monster| monster.id == target_id)
        .map(|monster| {
            format!(
                "{} slot {}",
                monster_name(monster.monster_type),
                monster.slot
            )
        })
        .unwrap_or_else(|| format!("entity {target_id}"))
}

fn run_choice_candidates(
    session: &RunControlSession,
    choice: &crate::state::core::RunPendingChoiceState,
) -> Vec<DecisionCandidate> {
    let request = choice.selection_request(&session.run_state);
    let uuids = request
        .targets
        .iter()
        .map(|target| match target {
            crate::state::selection::SelectionTargetRef::CardUuid(uuid) => *uuid,
        })
        .collect::<Vec<_>>();
    let single_choice = choice.min_choices == 1 && choice.max_choices == 1;
    session
        .run_state
        .master_deck
        .iter()
        .enumerate()
        .filter(|(_, card)| uuids.contains(&card.uuid))
        .map(|(idx, card)| {
            if single_choice {
                candidate(
                    idx.to_string(),
                    combat_card_label(card),
                    ClientInput::SubmitDeckSelect(vec![idx]),
                    None::<String>,
                )
            } else {
                candidate(
                    idx.to_string(),
                    combat_card_label(card),
                    "select <deck_idx...>",
                    Some("requires explicit multi-card selection"),
                )
            }
        })
        .collect()
}

fn boss_relic_candidates(choice: &BossRelicChoiceState) -> Vec<DecisionCandidate> {
    choice
        .relics
        .iter()
        .enumerate()
        .map(|(idx, relic)| {
            let mut candidate = candidate(
                idx.to_string(),
                format!("{relic:?}"),
                ClientInput::SubmitRelicChoice(idx),
                None::<String>,
            );
            candidate.resolution = Some(CandidateResolution::from_boss_relic(*relic));
            candidate
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::cards::CardId;
    use crate::runtime::combat::CombatCard;
    use crate::state::core::{
        ActiveCombat, CombatContext, GridSelectReason, PendingChoice, RoomCombatContext,
    };
    use crate::state::map::node::RoomType;

    #[test]
    fn pending_grid_select_candidates_are_visible_and_executable() {
        let mut session = RunControlSession::new(Default::default());
        let mut combat = crate::test_support::blank_test_combat();
        combat.zones.discard_pile = vec![
            CombatCard::new(CardId::Strike, 10),
            CombatCard::new(CardId::Defend, 20),
        ];
        let choice = PendingChoice::GridSelect {
            source_pile: PileType::Discard,
            candidate_uuids: vec![10, 20],
            min_cards: 1,
            max_cards: 1,
            can_cancel: false,
            reason: GridSelectReason::MoveToDrawPile,
        };
        session.engine_state = EngineState::PendingChoice(choice.clone());
        session.active_combat = Some(ActiveCombat::new(
            EngineState::PendingChoice(choice),
            combat,
            CombatContext::Room(RoomCombatContext {
                room_type: RoomType::MonsterRoom,
            }),
        ));

        let candidates = decision_candidates(&session);

        assert_eq!(candidates.len(), 2);
        assert_eq!(candidates[0].id, "0");
        assert!(candidates[0]
            .label
            .contains("Put on top of draw pile: Strike"));
        assert_eq!(
            candidates[0].action.executable_input(),
            Some(ClientInput::SubmitGridSelect(vec![10]))
        );
        assert_eq!(
            candidates[1].action.executable_input(),
            Some(ClientInput::SubmitGridSelect(vec![20]))
        );
    }
}
