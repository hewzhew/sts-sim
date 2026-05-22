use crate::sim::combat_action::combat_action_key;
use crate::sim::combat_legal_actions::get_legal_moves;
use crate::state::core::EngineState;
use crate::state::events::{EventOption, EventOptionTransition};
use crate::state::rewards::{BossRelicChoiceState, RewardState};

use super::labels::{
    candidate, clean_event_label, combat_card_label, describe_combat_input, event_effect_summary,
    reward_card_label, reward_item_label, room_type_label, shop_block_note,
};
use super::{DecisionCandidate, RunControlSession};

pub(super) fn decision_candidates(session: &RunControlSession) -> Vec<DecisionCandidate> {
    match &session.engine_state {
        EngineState::EventRoom => event_candidates(session),
        EngineState::MapNavigation => map_candidates(session),
        EngineState::RewardScreen(reward) => reward_candidates(reward),
        EngineState::TreasureRoom(_) => {
            vec![candidate("open", "Open chest", "open", Some("routine"))]
        }
        EngineState::Campfire => campfire_candidates(session),
        EngineState::Shop(shop) => shop_candidates(shop),
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
            let mut label = clean_event_label(&option.ui.text);
            if let Some(effect_summary) = event_effect_summary(&option.semantics.effects) {
                label = format!("{label} | {effect_summary}");
            }
            candidate(
                idx.to_string(),
                label,
                format!("event {idx}"),
                event_option_note(option, options.len()),
            )
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
    if option_count == 1
        && matches!(
            option.semantics.transition,
            EventOptionTransition::AdvanceScreen | EventOptionTransition::Complete
        )
    {
        return Some("routine".to_string());
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

fn map_candidates(session: &RunControlSession) -> Vec<DecisionCandidate> {
    let target_y = if session.run_state.map.current_y == -1 {
        0
    } else {
        session.run_state.map.current_y + 1
    };
    if target_y == 15 {
        return vec![candidate("0", "Boss room", "go 0", Some("boss"))];
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
                format!("go {}", node.x),
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
                    format!("pick {idx}"),
                    None::<String>,
                )
            })
            .collect::<Vec<_>>();
        candidates.push(candidate(
            "skip",
            "Skip card reward",
            "proceed",
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
                format!("claim {idx}"),
                None::<String>,
            )
        })
        .collect::<Vec<_>>();
    if reward.skippable {
        candidates.push(candidate(
            "skip",
            "Leave reward screen",
            "proceed",
            None::<String>,
        ));
    }
    candidates
}

fn campfire_candidates(session: &RunControlSession) -> Vec<DecisionCandidate> {
    let mut candidates = vec![candidate("rest", "Rest", "rest", None::<String>)];
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
                format!("smith {idx}"),
                None::<String>,
            )
        });
    candidates.extend(upgradeable);
    candidates.push(candidate(
        "recall",
        "Recall ruby key",
        "recall",
        None::<String>,
    ));
    candidates
}

fn shop_candidates(shop: &crate::state::shop::ShopState) -> Vec<DecisionCandidate> {
    let mut candidates = Vec::new();
    candidates.extend(shop.cards.iter().enumerate().map(|(idx, card)| {
        candidate(
            format!("card-{idx}"),
            format!(
                "{} | {} gold",
                reward_card_label(card.card_id, card.upgrades),
                card.price
            ),
            format!("buy card {idx}"),
            shop_block_note(card.can_buy, card.blocked_reason.as_deref()),
        )
    }));
    candidates.extend(shop.relics.iter().enumerate().map(|(idx, relic)| {
        candidate(
            format!("relic-{idx}"),
            format!("{:?} | {} gold", relic.relic_id, relic.price),
            format!("buy relic {idx}"),
            shop_block_note(relic.can_buy, relic.blocked_reason.as_deref()),
        )
    }));
    candidates.extend(shop.potions.iter().enumerate().map(|(idx, potion)| {
        candidate(
            format!("potion-{idx}"),
            format!("{:?} | {} gold", potion.potion_id, potion.price),
            format!("buy potion {idx}"),
            shop_block_note(potion.can_buy, potion.blocked_reason.as_deref()),
        )
    }));
    candidates.push(candidate(
        "purge",
        format!("Remove card | {} gold", shop.purge_cost),
        "purge <deck_idx>",
        (!shop.purge_available).then_some("locked"),
    ));
    candidates.push(candidate("leave", "Leave shop", "proceed", None::<String>));
    candidates
}

fn combat_candidates(session: &RunControlSession) -> Vec<DecisionCandidate> {
    let Ok(position) = session.current_combat_position_for_actions() else {
        return Vec::new();
    };
    get_legal_moves(&position.engine, &position.combat)
        .iter()
        .enumerate()
        .map(|(idx, action)| {
            candidate(
                idx.to_string(),
                describe_combat_input(&position.combat, action),
                format!("action {idx}"),
                Some(combat_action_key(&position.combat, action)),
            )
        })
        .collect()
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
    session
        .run_state
        .master_deck
        .iter()
        .enumerate()
        .filter(|(_, card)| uuids.contains(&card.uuid))
        .map(|(idx, card)| {
            candidate(
                idx.to_string(),
                combat_card_label(card),
                format!("select {idx}"),
                None::<String>,
            )
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
                format!("relic {idx}"),
                None::<String>,
            )
        })
        .collect()
}
