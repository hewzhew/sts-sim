use crate::sim::combat_legal_actions::get_legal_moves;
use crate::state::core::{CampfireChoice, ClientInput, EngineState};
use crate::state::events::{EventEffect, EventOption, EventOptionTransition, EventRelicKind};
use crate::state::rewards::{BossRelicChoiceState, RewardState};
use std::collections::BTreeMap;

use super::labels::{
    candidate, clean_event_label, combat_card_label, event_effect_summary, monster_name,
    reward_card_label, reward_item_label, room_type_label, shop_block_note, unavailable_candidate,
};
use super::{DecisionCandidate, RunControlSession};

pub(super) fn decision_candidates(session: &RunControlSession) -> Vec<DecisionCandidate> {
    match &session.engine_state {
        EngineState::EventRoom => event_candidates(session),
        EngineState::MapNavigation => map_candidates(session),
        EngineState::RewardScreen(reward) => reward_candidates(reward),
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
            let effect_summary = event_effect_summary(&option.semantics.effects);
            let note = event_option_note(option, options.len(), effect_summary.as_deref());
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
                candidate(idx.to_string(), label, ClientInput::EventChoice(idx), note)
            }
        })
        .collect()
}

fn event_option_note(
    option: &EventOption,
    option_count: usize,
    effect_summary: Option<&str>,
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
    let prefix = if event_effects_are_partial(&option.semantics.effects)
        || matches!(
            option.semantics.transition,
            EventOptionTransition::OpenSelection(_) | EventOptionTransition::OpenReward
        ) {
        "partial"
    } else {
        "known"
    };
    if let Some(effect_summary) = effect_summary {
        let transition = match option.semantics.transition {
            EventOptionTransition::OpenSelection(kind) => format!("; opens {kind:?} selection"),
            EventOptionTransition::OpenReward => "; opens follow-up reward".to_string(),
            EventOptionTransition::StartCombat => "; starts combat".to_string(),
            _ => String::new(),
        };
        return Some(format!("{prefix}: {effect_summary}{transition}"));
    }
    match option.semantics.transition {
        EventOptionTransition::OpenSelection(kind) => Some(format!("opens {kind:?} selection")),
        EventOptionTransition::OpenReward => Some("opens reward".to_string()),
        EventOptionTransition::StartCombat => Some("starts combat".to_string()),
        EventOptionTransition::AdvanceScreen => Some("advances event".to_string()),
        EventOptionTransition::Complete => Some("leaves event".to_string()),
        EventOptionTransition::None => None,
    }
}

fn event_effects_are_partial(effects: &[EventEffect]) -> bool {
    effects.iter().any(|effect| {
        matches!(
            effect,
            EventEffect::ObtainRelic {
                kind: EventRelicKind::RandomRelic
                    | EventRelicKind::RandomBook
                    | EventRelicKind::RandomFace,
                ..
            } | EventEffect::ObtainPotion { .. }
                | EventEffect::ObtainCard { .. }
                | EventEffect::ObtainColorlessCard { .. }
                | EventEffect::ObtainCurse { .. }
                | EventEffect::TransformCard { .. }
        )
    })
}

fn map_candidates(session: &RunControlSession) -> Vec<DecisionCandidate> {
    let target_y = if session.run_state.map.current_y == -1 {
        0
    } else {
        session.run_state.map.current_y + 1
    };
    if target_y == 15 {
        return vec![candidate(
            "0",
            "Boss room",
            ClientInput::SelectMapNode(0),
            Some("boss"),
        )];
    }
    let Some(row) = session.run_state.map.graph.get(target_y as usize) else {
        return Vec::new();
    };
    row.iter()
        .filter(|node| session.run_state.map.can_travel_to(node.x, node.y, false))
        .map(|node| {
            candidate(
                node.x.to_string(),
                format!("y={} {}", node.y, room_type_label(node.class)),
                ClientInput::SelectMapNode(node.x as usize),
                node.has_emerald_key.then_some("emerald elite"),
            )
        })
        .collect()
}

fn reward_candidates(reward: &RewardState) -> Vec<DecisionCandidate> {
    if let Some(cards) = reward.pending_card_choice.as_ref() {
        let mut candidates = cards
            .iter()
            .enumerate()
            .map(|(idx, card)| {
                candidate(
                    idx.to_string(),
                    reward_card_label(card.id, card.upgrades),
                    ClientInput::SelectCard(idx),
                    None::<String>,
                )
            })
            .collect::<Vec<_>>();
        candidates.push(candidate(
            cards.len().to_string(),
            "Skip card reward",
            ClientInput::Proceed,
            None::<String>,
        ));
        return candidates;
    }

    let mut candidates = reward
        .items
        .iter()
        .enumerate()
        .map(|(idx, item)| {
            candidate(
                idx.to_string(),
                reward_item_label(item),
                ClientInput::ClaimReward(idx),
                None::<String>,
            )
        })
        .collect::<Vec<_>>();
    if reward.skippable {
        candidates.push(candidate(
            "skip",
            "Leave reward screen",
            ClientInput::Proceed,
            None::<String>,
        ));
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
        .take(8)
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
        let label = format!("{:?} | {} gold", potion.potion_id, potion.price);
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
            candidate(
                idx.to_string(),
                format!("{relic:?}"),
                ClientInput::SubmitRelicChoice(idx),
                None::<String>,
            )
        })
        .collect()
}
