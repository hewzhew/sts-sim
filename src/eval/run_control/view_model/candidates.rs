use crate::content::potions::get_potion_definition;
use crate::content::relics::RelicId;
use crate::eval::event_boundary_classifier_v1::classify_event_option_boundary_v1;
use crate::eval::run_control::RunDecisionAction;
use crate::runtime::combat::CombatCard;
use crate::sim::combat_legal_actions::get_legal_moves;
use crate::state::core::{CampfireChoice, ClientInput, EngineState, PendingChoice, PileType};
use crate::state::events::{EventOption, EventOptionTransition};
use crate::state::rewards::{BossRelicChoiceState, RewardState};
use crate::state::selection::{SelectionResolution, SelectionScope};
use std::collections::BTreeMap;

use super::labels::{
    candidate, clean_event_label, combat_card_label, monster_name, reward_card_label,
    reward_item_label, room_type_label, shop_block_note, unavailable_candidate,
};
use super::{CandidateResolution, DecisionCandidate, DecisionCandidateKey, RunControlSession};

pub(super) fn decision_candidates(session: &RunControlSession) -> Vec<DecisionCandidate> {
    match &session.engine_state {
        EngineState::EventRoom => event_candidates(session),
        EngineState::MapNavigation | EngineState::MapOverlay { .. } => map_candidates(session),
        EngineState::RewardScreen(reward) => reward_candidates(session, reward, None),
        EngineState::RewardOverlay {
            reward_state,
            return_state,
        } => reward_candidates(session, reward_state, Some(return_state.as_ref())),
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
    let event_state = session.run_state.event_state.as_ref();
    options
        .iter()
        .enumerate()
        .map(|(idx, option)| {
            let label = clean_event_label(&option.ui.text);
            let resolution = CandidateResolution::from_event_option(option);
            let note = event_option_note(option, options.len());
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
                candidate.key = event_state.map(|event_state| DecisionCandidateKey::EventOption {
                    event_id: event_state.id,
                    screen: event_state.current_screen,
                    option_index: idx,
                    action: option.semantics.action,
                });
                candidate.resolution = resolution;
                candidate
            }
        })
        .collect()
}

fn event_option_note(option: &EventOption, option_count: usize) -> Option<String> {
    if option.ui.disabled {
        return Some(format!(
            "locked: {}",
            option.ui.disabled_reason.as_deref().unwrap_or("disabled")
        ));
    }
    if option_count == 1 {
        if let Some(note) = classify_event_option_boundary_v1(option).single_candidate_note() {
            return Some(note.to_string());
        }
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

fn reward_candidates(
    session: &RunControlSession,
    reward: &RewardState,
    overlay_return_state: Option<&EngineState>,
) -> Vec<DecisionCandidate> {
    if let Some(cards) = reward.pending_card_choice.as_ref() {
        let reward_item_index = reward.pending_card_reward_index;
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
                if let Some(reward_item_index) = reward_item_index {
                    candidate.key = Some(DecisionCandidateKey::CardRewardPick {
                        reward_item_index,
                        option_index: idx,
                        card: card.id,
                        upgrades: card.upgrades,
                    });
                }
                candidate.resolution = Some(CandidateResolution::from_reward_card(card));
                candidate
            })
            .collect::<Vec<_>>();
        if session
            .run_state
            .relics
            .iter()
            .any(|relic| relic.id == RelicId::SingingBowl)
        {
            candidates.push(candidate(
                "bowl",
                "Singing Bowl | gain 2 max HP",
                ClientInput::SelectCard(cards.len()),
                Some("consume this card reward instead of taking a card"),
            ));
            if let Some(candidate) = candidates.last_mut() {
                if let Some(reward_item_index) = reward_item_index {
                    candidate.key = Some(DecisionCandidateKey::CardRewardSingingBowl {
                        reward_item_index,
                        option_index: cards.len(),
                    });
                }
            }
        }
        if let Some(reward_item_index) = reward.pending_card_reward_index {
            let mut skip = candidate(
                "skip-card-reward",
                "Skip card reward",
                RunDecisionAction::SkipCardReward { reward_item_index },
                Some("consume this card reward without taking a card"),
            );
            skip.key = Some(DecisionCandidateKey::CardRewardSkip { reward_item_index });
            candidates.push(skip);
        }
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
            if matches!(item, crate::state::rewards::RewardItem::Card { .. }) {
                candidate.key = Some(DecisionCandidateKey::CardRewardOpen {
                    reward_item_index: idx,
                });
            }
            candidate.resolution =
                CandidateResolution::from_reward_item(item, reward, &session.run_state);
            candidate
        })
        .collect::<Vec<_>>();
    if reward.has_card_reward_item()
        && session
            .run_state
            .relics
            .iter()
            .any(|relic| relic.id == RelicId::SingingBowl)
    {
        if let Some(reward_item_index) = reward
            .items
            .iter()
            .position(|item| matches!(item, crate::state::rewards::RewardItem::Card { .. }))
        {
            candidates.push(candidate(
                "bowl",
                "Singing Bowl | gain 2 max HP",
                RunDecisionAction::SingingBowlCardReward { reward_item_index },
                Some("consume the first visible card reward instead of taking a card"),
            ));
            if let Some(candidate) = candidates.last_mut() {
                candidate.key = Some(DecisionCandidateKey::CardRewardSingingBowl {
                    reward_item_index,
                    option_index: 0,
                });
            }
        }
    }
    if reward.skippable {
        let (id, label, note, input) = if let Some(return_state) = overlay_return_state {
            (
                "back",
                reward_overlay_return_label(return_state),
                if reward.items.is_empty() {
                    Some("routine")
                } else if matches!(return_state, EngineState::Shop(_)) {
                    Some("unclaimed overlay rewards remain available in the shop")
                } else {
                    Some("returns to previous screen with unclaimed overlay rewards")
                },
                ClientInput::Cancel,
            )
        } else if reward.items.is_empty() {
            (
                "skip",
                "Leave reward screen".to_string(),
                Some("routine"),
                ClientInput::Proceed,
            )
        } else {
            (
                "skip",
                "Open map preview".to_string(),
                Some("unclaimed rewards remain until a path is chosen"),
                ClientInput::Proceed,
            )
        };
        candidates.push(candidate(id, label, input, note));
        if let Some(candidate) = candidates.last_mut() {
            if candidate.id == "skip" && reward.has_card_reward_item() {
                if let Some(reward_item_index) = reward
                    .items
                    .iter()
                    .position(|item| matches!(item, crate::state::rewards::RewardItem::Card { .. }))
                {
                    candidate.key =
                        Some(DecisionCandidateKey::CardRewardSkip { reward_item_index });
                }
            }
        }
    }
    candidates
}

fn reward_overlay_return_label(return_state: &EngineState) -> String {
    match return_state {
        EngineState::Shop(_) => "Return to shop".to_string(),
        EngineState::EventRoom => "Return to event".to_string(),
        EngineState::Campfire => "Return to campfire".to_string(),
        EngineState::RewardScreen(_) | EngineState::RewardOverlay { .. } => {
            "Return to reward screen".to_string()
        }
        _ => "Return to previous screen".to_string(),
    }
}

fn campfire_candidates(session: &RunControlSession) -> Vec<DecisionCandidate> {
    let mut candidates = Vec::new();
    for choice in crate::engine::campfire_handler::get_available_options(&session.run_state) {
        match choice {
            CampfireChoice::Rest => candidates.push(candidate(
                "rest",
                "Rest",
                ClientInput::CampfireOption(CampfireChoice::Rest),
                None::<String>,
            )),
            CampfireChoice::Smith(_) => {
                candidates.extend(
                    session
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
                        }),
                );
            }
            CampfireChoice::Dig => candidates.push(candidate(
                "dig",
                "Dig",
                ClientInput::CampfireOption(CampfireChoice::Dig),
                None::<String>,
            )),
            CampfireChoice::Lift => candidates.push(candidate(
                "lift",
                "Lift",
                ClientInput::CampfireOption(CampfireChoice::Lift),
                None::<String>,
            )),
            CampfireChoice::Toke(_) => {
                candidates.extend(
                    session
                        .run_state
                        .master_deck
                        .iter()
                        .enumerate()
                        .filter(|(_, card)| {
                            crate::state::core::master_deck_card_is_purgeable(card)
                                && !crate::state::core::master_deck_card_is_bottled(
                                    card,
                                    &session.run_state.relics,
                                )
                        })
                        .map(|(idx, card)| {
                            candidate(
                                format!("toke-{idx}"),
                                format!("Toke {}", combat_card_label(card)),
                                ClientInput::CampfireOption(CampfireChoice::Toke(idx)),
                                None::<String>,
                            )
                        }),
                );
            }
            CampfireChoice::Recall => candidates.push(candidate(
                "recall",
                "Recall ruby key",
                ClientInput::CampfireOption(CampfireChoice::Recall),
                None::<String>,
            )),
        }
    }
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
        let key = DecisionCandidateKey::ShopBuyCard {
            shop_slot: idx,
            card: card.card_id,
            upgrades: card.upgrades,
            price: card.price,
        };
        let mut candidate = if card.can_buy {
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
        };
        candidate.key = Some(key);
        candidate
    }));
    candidates.extend(shop.relics.iter().enumerate().map(|(idx, relic)| {
        let label = format!("{:?} | {} gold", relic.relic_id, relic.price);
        let note = shop_block_note(relic.can_buy, relic.blocked_reason.as_deref());
        let key = DecisionCandidateKey::ShopBuyRelic {
            shop_slot: idx,
            relic: relic.relic_id,
            price: relic.price,
        };
        let mut candidate = if relic.can_buy {
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
        };
        candidate.key = Some(key);
        candidate
    }));
    candidates.extend(shop.potions.iter().enumerate().map(|(idx, potion)| {
        let label = format!(
            "{} | {} gold",
            get_potion_definition(potion.potion_id).name,
            potion.price
        );
        let block_reason =
            super::super::shop_potion_purchase_block_reason_v1(&session.run_state, potion);
        let note = shop_block_note(block_reason.is_none(), block_reason.as_deref());
        let key = DecisionCandidateKey::ShopBuyPotion {
            shop_slot: idx,
            potion: potion.potion_id,
            price: potion.price,
        };
        let mut candidate = if block_reason.is_none() {
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
                block_reason.unwrap_or_else(|| "cannot buy".to_string()),
                note,
            )
        };
        candidate.key = Some(key);
        candidate
    }));
    let purge_block = shop_purge_block_reason(session, shop);
    if purge_block.is_none() {
        candidates.extend(session.run_state.master_deck.iter().enumerate().filter_map(
            |(deck_index, card)| {
                if !crate::state::core::master_deck_card_is_purgeable(card)
                    || crate::state::core::master_deck_card_is_bottled(
                        card,
                        &session.run_state.relics,
                    )
                {
                    return None;
                }
                let mut candidate = candidate(
                    format!("purge-{deck_index}"),
                    format!(
                        "Remove {} | {} gold",
                        combat_card_label(card),
                        shop.purge_cost
                    ),
                    ClientInput::PurgeCard(deck_index),
                    None::<String>,
                );
                candidate.key = Some(DecisionCandidateKey::ShopPurgeCard {
                    deck_index,
                    card: card.id,
                    upgrades: card.upgrades,
                });
                Some(candidate)
            },
        ));
    } else {
        candidates.push(unavailable_candidate(
            "purge",
            format!("Remove card | {} gold", shop.purge_cost),
            purge_block.unwrap_or("locked"),
            purge_block.map(|reason| format!("locked: {reason}")),
        ));
    }
    if shop.pending_reward_overlay.is_some() {
        let mut rewards = candidate(
            "rewards",
            "Open pending rewards",
            ClientInput::OpenRewardOverlay,
            Some("shop overlay rewards remain until leaving the shop"),
        );
        rewards.key = Some(DecisionCandidateKey::ShopOpenRewards);
        candidates.push(rewards);
    }
    let mut leave = candidate("leave", "Leave shop", ClientInput::Proceed, None::<String>);
    leave.key = Some(DecisionCandidateKey::ShopLeave);
    candidates.push(leave);
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
        return pending_choice_candidates(session, choice, &position.combat);
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
    session: &RunControlSession,
    choice: &PendingChoice,
    combat: &crate::runtime::combat::CombatState,
) -> Vec<DecisionCandidate> {
    if let Some(surface) =
        crate::eval::run_control::selection_surface::active_selection_surface(session)
    {
        let mut select = candidate(
            "select",
            format!("Submit selection with `{}`", surface.submit_hint),
            surface.submit_hint,
            Some(selection_surface_note(&surface)),
        );
        select.key = Some(DecisionCandidateKey::SelectionSubmit {
            scope: surface.scope,
            reason: surface.reason,
            min_choices: surface.min_choices,
            max_choices: surface.max_choices,
            item_count: surface.item_count,
        });
        let mut candidates = vec![select];
        if surface.can_cancel {
            candidates.push(candidate(
                "cancel",
                "Cancel selection",
                ClientInput::Cancel,
                Some("return without selecting cards"),
            ));
        }
        return candidates;
    }

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

fn selection_surface_note(
    surface: &crate::eval::run_control::selection_surface::SelectionSurface,
) -> String {
    if surface.min_choices == 0 {
        format!(
            "choose 0-{} from {} visible item(s); `select` chooses nothing",
            surface.max_choices, surface.item_count
        )
    } else {
        format!(
            "choose {}-{} from {} visible item(s)",
            surface.min_choices, surface.max_choices, surface.item_count
        )
    }
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
            ClientInput::SubmitSelection(resolution),
        ) => format!(
            "{} {}",
            grid_reason_verb(*reason),
            selected_card_labels(
                grid_source_cards(combat, *source_pile),
                &resolution.selected_card_uuids()
            )
            .join(", ")
        ),
        (PendingChoice::HandSelect { reason, .. }, ClientInput::SubmitSelection(resolution)) => {
            format!(
                "{} {}",
                hand_reason_verb(*reason),
                selected_card_labels(&combat.zones.hand, &resolution.selected_card_uuids())
                    .join(", ")
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
    let Some(surface) =
        crate::eval::run_control::selection_surface::active_selection_surface(session)
    else {
        return Vec::new();
    };
    let single_choice = choice.min_choices == 1 && choice.max_choices == 1;
    if single_choice {
        return surface
            .items
            .iter()
            .map(|item| {
                candidate(
                    item.visible_index.to_string(),
                    reward_card_label(item.card, item.upgrades),
                    ClientInput::SubmitSelection(SelectionResolution {
                        scope: SelectionScope::Deck,
                        selected: vec![item.target],
                    }),
                    None::<String>,
                )
            })
            .collect();
    }

    let mut select = candidate(
        "select",
        format!("Submit selection with `{}`", surface.submit_hint),
        surface.submit_hint,
        Some(selection_surface_note(&surface)),
    );
    select.key = Some(DecisionCandidateKey::SelectionSubmit {
        scope: surface.scope,
        reason: surface.reason,
        min_choices: surface.min_choices,
        max_choices: surface.max_choices,
        item_count: surface.item_count,
    });
    vec![select]
}

fn boss_relic_candidates(choice: &BossRelicChoiceState) -> Vec<DecisionCandidate> {
    let mut candidates = choice
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
            candidate.key = Some(DecisionCandidateKey::BossRelicPick {
                option_index: idx,
                relic: *relic,
            });
            candidate.resolution = Some(CandidateResolution::from_boss_relic(*relic));
            candidate
        })
        .collect::<Vec<_>>();
    let mut skip = candidate(
        "skip",
        "Skip boss relic",
        ClientInput::Cancel,
        Some("leaves boss chest without taking a boss relic"),
    );
    skip.key = Some(DecisionCandidateKey::BossRelicSkip);
    candidates.push(skip);
    candidates
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::cards::CardId;
    use crate::content::relics::{RelicId, RelicState};
    use crate::runtime::combat::CombatCard;
    use crate::state::core::{
        ActiveCombat, CombatContext, GridSelectReason, PendingChoice, RoomCombatContext,
    };
    use crate::state::map::node::RoomType;
    use crate::state::rewards::RewardItem;

    #[test]
    fn pending_grid_select_uses_compact_selection_command() {
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

        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].id, "select");
        assert!(candidates[0]
            .label
            .contains("Submit selection with `select <idx...>`"));
        assert!(candidates[0].action.executable_input().is_none());
    }

    #[test]
    fn singing_bowl_card_reward_candidate_is_visible_and_executable() {
        let mut session = RunControlSession::new(Default::default());
        session
            .run_state
            .relics
            .push(RelicState::new(RelicId::SingingBowl));
        let mut reward = RewardState::new();
        reward.pending_card_choice = Some(vec![crate::state::rewards::RewardCard::new(
            CardId::PommelStrike,
            0,
        )]);
        reward.pending_card_reward_index = Some(0);
        session.engine_state = EngineState::RewardScreen(reward);

        let candidates = decision_candidates(&session);

        let bowl = candidates
            .iter()
            .find(|candidate| candidate.id == "bowl")
            .expect("Singing Bowl should appear as a visible card reward option");
        assert_eq!(
            bowl.action.executable_input(),
            Some(ClientInput::SelectCard(1))
        );
        assert!(bowl.label.contains("gain 2 max HP"));
    }

    #[test]
    fn singing_bowl_unopened_card_reward_candidate_is_visible_as_command() {
        let mut session = RunControlSession::new(Default::default());
        session
            .run_state
            .relics
            .push(RelicState::new(RelicId::SingingBowl));
        let mut reward = RewardState::new();
        reward.items = vec![RewardItem::Card {
            cards: vec![crate::state::rewards::RewardCard::new(
                CardId::PommelStrike,
                0,
            )],
        }];
        session.engine_state = EngineState::RewardScreen(reward);

        let candidates = decision_candidates(&session);

        let bowl = candidates
            .iter()
            .find(|candidate| candidate.id == "bowl")
            .expect("Singing Bowl should be visible next to unopened card reward item");
        assert!(bowl.action.executable_input().is_none());
        assert!(bowl.label.contains("gain 2 max HP"));
    }

    #[test]
    fn reward_overlay_uses_return_candidate_instead_of_map_preview_skip() {
        let mut session = RunControlSession::new(Default::default());
        let mut reward = RewardState::new();
        reward.items = vec![RewardItem::Gold { amount: 25 }];
        session.engine_state = EngineState::reward_overlay(
            reward,
            EngineState::Shop(crate::state::shop::ShopState::new()),
        );

        let candidates = decision_candidates(&session);

        assert!(candidates.iter().any(|candidate| {
            candidate.id == "back"
                && candidate.label == "Return to shop"
                && candidate.action.executable_input() == Some(ClientInput::Cancel)
        }));
        assert!(!candidates.iter().any(|candidate| candidate.id == "skip"));
    }
}
