use super::*;

pub fn legal_actions(
    engine_state: &EngineState,
    run_state: &RunState,
    combat_state: &Option<CombatState>,
) -> Vec<ClientInput> {
    match engine_state {
        EngineState::CombatPlayerTurn | EngineState::PendingChoice(_) => combat_state
            .as_ref()
            .map(|combat| crate::bot::combat::legal_moves_for_audit(engine_state, combat))
            .unwrap_or_default(),
        EngineState::MapNavigation => legal_map_actions(run_state),
        EngineState::EventRoom => crate::engine::event_handler::get_event_options(run_state)
            .into_iter()
            .enumerate()
            .filter(|(_, option)| !option.ui.disabled)
            .map(|(idx, _)| ClientInput::EventChoice(idx))
            .collect(),
        EngineState::RewardScreen(reward_state) => legal_reward_actions(run_state, reward_state),
        EngineState::BossRelicSelect(state) => {
            let mut actions = (0..state.relics.len())
                .map(ClientInput::SubmitRelicChoice)
                .collect::<Vec<_>>();
            actions.push(ClientInput::Proceed);
            actions
        }
        EngineState::Campfire => legal_campfire_actions(run_state),
        EngineState::Shop(shop) => legal_shop_actions(run_state, shop),
        EngineState::RunPendingChoice(choice) => {
            let request = choice.selection_request(run_state);
            legal_selection_actions(&request)
        }
        EngineState::CombatProcessing | EngineState::EventCombat(_) | EngineState::GameOver(_) => {
            Vec::new()
        }
    }
}

pub fn legal_map_actions(run_state: &RunState) -> Vec<ClientInput> {
    let next_y = if run_state.map.current_y == -1 {
        0
    } else {
        run_state.map.current_y + 1
    };
    if run_state.map.current_y == 14 {
        return vec![ClientInput::SelectMapNode(0)];
    }

    let mut actions = Vec::new();
    if next_y <= run_state.map.graph.len() as i32 {
        for x in 0..7 {
            if run_state.map.can_travel_to(x, next_y, false) {
                actions.push(ClientInput::SelectMapNode(x as usize));
            }
        }
    }
    actions
}

pub fn legal_reward_actions(run_state: &RunState, reward_state: &RewardState) -> Vec<ClientInput> {
    if let Some(cards) = &reward_state.pending_card_choice {
        let mut actions = (0..cards.len())
            .map(ClientInput::SelectCard)
            .collect::<Vec<_>>();
        if run_state
            .relics
            .iter()
            .any(|relic| relic.id == RelicId::SingingBowl)
        {
            actions.push(ClientInput::SelectCard(cards.len()));
        }
        actions.push(ClientInput::Proceed);
        return actions;
    }

    let mut actions = Vec::new();
    for (idx, item) in reward_state.items.iter().enumerate() {
        if reward_item_claimable(run_state, item) {
            actions.push(ClientInput::ClaimReward(idx));
        }
    }
    actions.push(ClientInput::Proceed);
    actions
}

pub fn legal_shop_actions(run_state: &RunState, shop: &crate::shop::ShopState) -> Vec<ClientInput> {
    let mut actions = vec![ClientInput::Proceed];
    for (idx, card) in shop.cards.iter().enumerate() {
        if card.can_buy && run_state.gold >= card.price {
            actions.push(ClientInput::BuyCard(idx));
        }
    }
    for (idx, relic) in shop.relics.iter().enumerate() {
        if relic.can_buy && run_state.gold >= relic.price {
            actions.push(ClientInput::BuyRelic(idx));
        }
    }
    let has_empty_potion_slot = run_state.potions.iter().any(Option::is_none);
    let has_sozu = run_state
        .relics
        .iter()
        .any(|relic| relic.id == RelicId::Sozu);
    for (idx, potion) in shop.potions.iter().enumerate() {
        if potion.can_buy && run_state.gold >= potion.price && (has_empty_potion_slot || has_sozu) {
            actions.push(ClientInput::BuyPotion(idx));
        }
    }
    if shop.purge_available && run_state.gold >= shop.purge_cost {
        for idx in 0..run_state.master_deck.len() {
            actions.push(ClientInput::PurgeCard(idx));
        }
    }
    actions
}

pub fn legal_campfire_actions(run_state: &RunState) -> Vec<ClientInput> {
    let available = crate::engine::campfire_handler::get_available_options(run_state);
    let mut actions = Vec::new();
    for choice in available {
        match choice {
            CampfireChoice::Smith(_) => {
                for (idx, card) in run_state.master_deck.iter().enumerate() {
                    if card.id == CardId::SearingBlow || card.upgrades == 0 {
                        actions.push(ClientInput::CampfireOption(CampfireChoice::Smith(idx)));
                    }
                }
            }
            CampfireChoice::Toke(_) => {
                for idx in 0..run_state.master_deck.len() {
                    actions.push(ClientInput::CampfireOption(CampfireChoice::Toke(idx)));
                }
            }
            other => actions.push(ClientInput::CampfireOption(other)),
        }
    }
    actions
}

pub fn legal_selection_actions(
    request: &crate::state::selection::SelectionRequest,
) -> Vec<ClientInput> {
    let (min, max) = selection_bounds(request);
    let targets = request.targets.clone();
    let mut actions = Vec::new();
    if request.can_cancel || min == 0 {
        actions.push(ClientInput::Cancel);
    }
    let max_actions = 128usize;
    let max_take = max.min(targets.len());
    for take in min..=max_take {
        if take == 0 {
            continue;
        }
        let mut current = Vec::new();
        push_selection_combinations(
            request.scope,
            &targets,
            take,
            0,
            &mut current,
            &mut actions,
            max_actions,
        );
        if actions.len() >= max_actions {
            break;
        }
    }
    actions
}

pub fn selection_bounds(request: &crate::state::selection::SelectionRequest) -> (usize, usize) {
    match request.constraint {
        crate::state::selection::SelectionConstraint::Exactly(n) => (n, n),
        crate::state::selection::SelectionConstraint::Between { min, max } => (min, max),
        crate::state::selection::SelectionConstraint::UpToAvailable => (1, request.targets.len()),
        crate::state::selection::SelectionConstraint::OptionalUpToAvailable => {
            (0, request.targets.len())
        }
    }
}

pub fn push_selection_combinations(
    scope: SelectionScope,
    targets: &[SelectionTargetRef],
    take: usize,
    start: usize,
    current: &mut Vec<SelectionTargetRef>,
    out: &mut Vec<ClientInput>,
    max_actions: usize,
) {
    if out.len() >= max_actions {
        return;
    }
    if current.len() == take {
        out.push(ClientInput::SubmitSelection(SelectionResolution {
            scope,
            selected: current.clone(),
        }));
        return;
    }
    for idx in start..targets.len() {
        current.push(targets[idx]);
        push_selection_combinations(scope, targets, take, idx + 1, current, out, max_actions);
        current.pop();
        if out.len() >= max_actions {
            return;
        }
    }
}

pub fn action_key_for_input(input: &ClientInput, combat: Option<&CombatState>) -> String {
    match input {
        ClientInput::PlayCard { card_index, target } => {
            let card_label = combat
                .and_then(|combat| combat.zones.hand.get(*card_index))
                .map(|card| format!("{:?}", card.id))
                .unwrap_or_else(|| "unknown".to_string());
            format!(
                "combat/play_card/card:{card_label}/hand:{card_index}/target:{}",
                target_label(*target, combat)
            )
        }
        ClientInput::UsePotion {
            potion_index,
            target,
        } => format!(
            "combat/use_potion/slot:{potion_index}/target:{}",
            target_label(*target, combat)
        ),
        ClientInput::DiscardPotion(index) => format!("combat/discard_potion/slot:{index}"),
        ClientInput::EndTurn => "combat/end_turn".to_string(),
        ClientInput::SubmitCardChoice(indices) => format!("combat/card_choice/{indices:?}"),
        ClientInput::SubmitDiscoverChoice(index) => format!("choice/discover/{index}"),
        ClientInput::SelectMapNode(x) => format!("map/select_x/{x}"),
        ClientInput::FlyToNode(x, y) => format!("map/fly/x:{x}/y:{y}"),
        ClientInput::SelectEventOption(index) => format!("event/select_option/{index}"),
        ClientInput::CampfireOption(choice) => format!("campfire/{}", campfire_choice_key(choice)),
        ClientInput::EventChoice(index) => format!("event/choice/{index}"),
        ClientInput::SubmitScryDiscard(indices) => format!("combat/scry_discard/{indices:?}"),
        ClientInput::SubmitSelection(selection) => format!(
            "selection/{}/uuids:{}",
            selection_scope_key(selection.scope),
            selection_uuid_key(&selection.selected)
        ),
        ClientInput::SubmitHandSelect(uuids) => {
            format!("combat/hand_select/uuids:{}", uuid_list_key(uuids))
        }
        ClientInput::SubmitGridSelect(uuids) => {
            format!("combat/grid_select/uuids:{}", uuid_list_key(uuids))
        }
        ClientInput::SubmitDeckSelect(indices) => format!("deck/select_indices/{indices:?}"),
        ClientInput::ClaimReward(index) => format!("reward/claim/{index}"),
        ClientInput::SelectCard(index) => format!("reward/select_card/{index}"),
        ClientInput::BuyCard(index) => format!("shop/buy_card/{index}"),
        ClientInput::BuyRelic(index) => format!("shop/buy_relic/{index}"),
        ClientInput::BuyPotion(index) => format!("shop/buy_potion/{index}"),
        ClientInput::PurgeCard(index) => format!("shop/purge_card/{index}"),
        ClientInput::SubmitRelicChoice(index) => format!("boss_relic/select/{index}"),
        ClientInput::Proceed => "proceed".to_string(),
        ClientInput::Cancel => "cancel".to_string(),
    }
}

pub fn target_label(target: Option<usize>, combat: Option<&CombatState>) -> String {
    match target {
        None => "none".to_string(),
        Some(entity_id) => combat
            .and_then(|combat| {
                combat
                    .entities
                    .monsters
                    .iter()
                    .position(|monster| monster.id == entity_id)
            })
            .map(|slot| format!("monster_slot:{slot}"))
            .unwrap_or_else(|| format!("entity:{entity_id}")),
    }
}

pub fn campfire_choice_key(choice: &CampfireChoice) -> String {
    match choice {
        CampfireChoice::Rest => "rest".to_string(),
        CampfireChoice::Smith(idx) => format!("smith/{idx}"),
        CampfireChoice::Dig => "dig".to_string(),
        CampfireChoice::Lift => "lift".to_string(),
        CampfireChoice::Toke(idx) => format!("toke/{idx}"),
        CampfireChoice::Recall => "recall".to_string(),
    }
}

pub fn selection_scope_key(scope: SelectionScope) -> &'static str {
    match scope {
        SelectionScope::Hand => "hand",
        SelectionScope::Deck => "deck",
        SelectionScope::Grid => "grid",
    }
}

pub fn selection_uuid_key(selected: &[SelectionTargetRef]) -> String {
    let uuids = selected
        .iter()
        .map(|target| match target {
            SelectionTargetRef::CardUuid(uuid) => *uuid,
        })
        .collect::<Vec<_>>();
    uuid_list_key(&uuids)
}

pub fn uuid_list_key(uuids: &[u32]) -> String {
    uuids
        .iter()
        .map(u32::to_string)
        .collect::<Vec<_>>()
        .join(",")
}

pub fn stable_action_id(action_key: &str) -> u32 {
    let mut hash = 2_166_136_261u32;
    for byte in action_key.as_bytes() {
        hash ^= *byte as u32;
        hash = hash.wrapping_mul(16_777_619);
    }
    hash
}

