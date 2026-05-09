use super::*;

pub fn choose_action(
    policy: &mut EpisodePolicy,
    ctx: &EpisodeContext,
    legal_actions: &[ClientInput],
) -> Result<(usize, ClientInput), String> {
    match policy {
        EpisodePolicy::RandomMasked { rng } => {
            let idx = if legal_actions.len() == 1 {
                0
            } else {
                rng.random_range(0, legal_actions.len() as i32 - 1) as usize
            };
            Ok((idx, legal_actions[idx].clone()))
        }
        EpisodePolicy::RuleBaselineV0 => {
            let idx = choose_rule_baseline_action(ctx, legal_actions);
            Ok((idx, legal_actions[idx].clone()))
        }
        EpisodePolicy::Replay { actions, cursor } => {
            let action = actions
                .get(*cursor)
                .cloned()
                .ok_or_else(|| format!("replay trace exhausted at action {}", cursor))?;
            *cursor += 1;
            if let Some(index) = legal_actions
                .iter()
                .position(|legal_action| legal_action == &action)
            {
                Ok((index, action))
            } else {
                Err(format!(
                    "replay action {:?} is not legal; legal_count={}",
                    action,
                    legal_actions.len()
                ))
            }
        }
    }
}

pub fn choose_rule_baseline_action(ctx: &EpisodeContext, legal_actions: &[ClientInput]) -> usize {
    let mut best_index = 0usize;
    let mut best_score = i32::MIN;
    for (index, action) in legal_actions.iter().enumerate() {
        let score = rule_baseline_score(ctx, action);
        if score > best_score {
            best_index = index;
            best_score = score;
        }
    }
    best_index
}

pub fn episode_reward(
    result: &str,
    floor: i32,
    combat_win_count: usize,
    current_hp: i32,
    max_hp: i32,
) -> f32 {
    let terminal = match result {
        "victory" => 100.0,
        "defeat" => -10.0,
        "crash" => -100.0,
        _ => -2.0,
    };
    let hp_fraction = if max_hp > 0 {
        current_hp.max(0) as f32 / max_hp as f32
    } else {
        0.0
    };
    floor.max(0) as f32 + combat_win_count as f32 * 2.0 + hp_fraction + terminal
}

pub fn rule_baseline_score(ctx: &EpisodeContext, action: &ClientInput) -> i32 {
    match &ctx.engine_state {
        EngineState::CombatPlayerTurn | EngineState::PendingChoice(_) => {
            score_combat_action(ctx, action)
        }
        EngineState::RewardScreen(reward_state) => {
            score_reward_action(&ctx.run_state, reward_state, action)
        }
        EngineState::MapNavigation => score_map_action(&ctx.run_state, action),
        EngineState::EventRoom => score_event_action(action),
        EngineState::BossRelicSelect(state) => score_boss_relic_action(state, action),
        EngineState::Campfire => score_campfire_action(&ctx.run_state, action),
        EngineState::Shop(shop) => score_shop_action(&ctx.run_state, shop, action),
        EngineState::RunPendingChoice(choice) => {
            let request = choice.selection_request(&ctx.run_state);
            score_run_selection_action(&ctx.run_state, &request, action)
        }
        EngineState::CombatProcessing | EngineState::EventCombat(_) | EngineState::GameOver(_) => 0,
    }
}

pub fn score_combat_action(ctx: &EpisodeContext, action: &ClientInput) -> i32 {
    let Some(combat) = ctx.combat_state.as_ref() else {
        return score_noncombat_fallback(action);
    };
    match (&ctx.engine_state, action) {
        (
            EngineState::PendingChoice(PendingChoice::DiscoverySelect(cards)),
            ClientInput::SubmitDiscoverChoice(index),
        )
        | (
            EngineState::PendingChoice(PendingChoice::CardRewardSelect { cards, .. }),
            ClientInput::SubmitDiscoverChoice(index),
        ) => cards
            .get(*index)
            .map(|card_id| 100 + rule_card_offer_score(*card_id, &ctx.run_state))
            .unwrap_or(-1_000),
        (
            EngineState::PendingChoice(PendingChoice::CardRewardSelect { can_skip: true, .. }),
            ClientInput::Cancel,
        ) => 10,
        (
            EngineState::PendingChoice(PendingChoice::ScrySelect { .. }),
            ClientInput::SubmitScryDiscard(indices),
        ) => 10 + indices.len() as i32 * 8,
        (
            EngineState::PendingChoice(PendingChoice::StanceChoice),
            ClientInput::SubmitDiscoverChoice(index),
        ) => {
            let unblocked = visible_unblocked_damage(combat);
            match *index {
                1 if unblocked > 0 => 100,
                0 if unblocked == 0 => 80,
                _ => 20,
            }
        }
        (_, ClientInput::PlayCard { card_index, target }) => {
            score_play_card_action(combat, *card_index, *target)
        }
        (_, ClientInput::UsePotion { .. }) => -1_000,
        (_, ClientInput::DiscardPotion { .. }) => -50,
        (_, ClientInput::EndTurn) => {
            let playable_cards = combat
                .zones
                .hand
                .iter()
                .filter(|card| crate::content::cards::can_play_card(card, combat).is_ok())
                .count();
            if playable_cards == 0 {
                20
            } else {
                -200 - visible_unblocked_damage(combat) * 4
            }
        }
        _ => score_noncombat_fallback(action),
    }
}

pub fn score_play_card_action(
    combat: &CombatState,
    card_index: usize,
    target: Option<usize>,
) -> i32 {
    let Some(card) = combat.zones.hand.get(card_index) else {
        return -1_000;
    };
    let def = crate::content::cards::get_card_definition(card.id);
    let evaluated = crate::content::cards::evaluate_card_for_play(card, combat, target);
    let incoming = visible_incoming_damage(combat);
    let unblocked = (incoming - combat.entities.player.block).max(0);
    let hp = combat.entities.player.current_hp.max(1);
    let danger = unblocked >= hp / 3 || unblocked >= 12;
    let mut score = 20 - evaluated.get_cost().max(0) as i32 * 12;

    let damage = estimated_card_damage(combat, &evaluated, target);
    let block = evaluated
        .base_block_mut
        .max(def.base_block + card.upgrades as i32 * def.upgrade_block);
    if damage > 0 {
        score += damage * if danger { 8 } else { 11 };
        if estimated_action_kills_all(combat, &evaluated, target) {
            score += 900;
        } else if target
            .and_then(|target_id| alive_monster_by_id(combat, target_id))
            .is_some_and(|monster| damage >= monster.current_hp + monster.block)
        {
            score += 180;
        }
    }
    if block > 0 {
        let useful_block = block.min(unblocked.max(0));
        score += useful_block * if danger { 18 } else { 6 };
        score += (block - useful_block).max(0) * 2;
    }

    let specific_bonus = match card.id {
        CardId::Bash | CardId::Uppercut | CardId::Shockwave => 45,
        CardId::Disarm => 70,
        CardId::Inflame | CardId::DemonForm | CardId::FeelNoPain | CardId::DarkEmbrace => 55,
        CardId::ShrugItOff | CardId::PommelStrike | CardId::BattleTrance => 35,
        CardId::Offering | CardId::Adrenaline => 80,
        CardId::Immolate | CardId::Feed | CardId::Reaper => 65,
        CardId::Flex | CardId::Bloodletting | CardId::SeeingRed | CardId::SpotWeakness => 35,
        CardId::Defend if danger => 25,
        CardId::Strike if !danger => 8,
        _ => 0,
    };

    match def.card_type {
        crate::content::cards::CardType::Power => {
            score += if danger && unblocked >= hp { -20 } else { 8 };
        }
        crate::content::cards::CardType::Skill => {
            score += 12;
        }
        crate::content::cards::CardType::Attack => {
            score += if incoming == 0 { 20 } else { 8 };
        }
        crate::content::cards::CardType::Status | crate::content::cards::CardType::Curse => {
            score -= 80;
        }
    }

    score += specific_bonus;
    if damage == 0 && block == 0 && specific_bonus <= 0 {
        score -= 350;
    }

    score
}

pub fn estimated_card_damage(
    combat: &CombatState,
    card: &crate::runtime::combat::CombatCard,
    target: Option<usize>,
) -> i32 {
    let def = crate::content::cards::get_card_definition(card.id);
    if def.is_multi_damage || matches!(def.target, crate::content::cards::CardTarget::AllEnemy) {
        if !card.multi_damage.is_empty() {
            return card
                .multi_damage
                .iter()
                .take(alive_monster_count(combat))
                .copied()
                .sum();
        }
        return card.base_damage_mut.max(0) * alive_monster_count(combat) as i32;
    }

    let damage = card.base_damage_mut.max(0);
    if let Some(target_id) = target {
        if let Some(monster) = alive_monster_by_id(combat, target_id) {
            return damage.min(monster.current_hp + monster.block);
        }
    }
    damage
}

pub fn estimated_action_kills_all(
    combat: &CombatState,
    card: &crate::runtime::combat::CombatCard,
    target: Option<usize>,
) -> bool {
    let alive = combat
        .entities
        .monsters
        .iter()
        .filter(|monster| !monster.is_dying && !monster.is_escaped && !monster.half_dead)
        .collect::<Vec<_>>();
    if alive.is_empty() {
        return false;
    }
    let def = crate::content::cards::get_card_definition(card.id);
    if def.is_multi_damage || matches!(def.target, crate::content::cards::CardTarget::AllEnemy) {
        if !card.multi_damage.is_empty() {
            return alive.iter().enumerate().all(|(idx, monster)| {
                card.multi_damage.get(idx).copied().unwrap_or(0)
                    >= monster.current_hp + monster.block
            });
        }
        return alive
            .iter()
            .all(|monster| card.base_damage_mut >= monster.current_hp + monster.block);
    }
    if alive.len() == 1 {
        return target
            .and_then(|target_id| alive_monster_by_id(combat, target_id))
            .is_some_and(|monster| card.base_damage_mut >= monster.current_hp + monster.block);
    }
    false
}

pub fn alive_monster_by_id(
    combat: &CombatState,
    target_id: usize,
) -> Option<&crate::runtime::combat::MonsterEntity> {
    combat.entities.monsters.iter().find(|monster| {
        monster.id == target_id && !monster.is_dying && !monster.is_escaped && !monster.half_dead
    })
}

pub fn alive_monster_count(combat: &CombatState) -> usize {
    combat
        .entities
        .monsters
        .iter()
        .filter(|monster| !monster.is_dying && !monster.is_escaped && !monster.half_dead)
        .count()
}

pub fn total_alive_monster_hp(combat: &CombatState) -> i32 {
    combat
        .entities
        .monsters
        .iter()
        .filter(|monster| !monster.is_dying && !monster.is_escaped && !monster.half_dead)
        .map(|monster| monster.current_hp.max(0) + monster.block.max(0))
        .sum()
}

pub fn visible_unblocked_damage(combat: &CombatState) -> i32 {
    (visible_incoming_damage(combat) - combat.entities.player.block).max(0)
}

pub fn visible_incoming_damage(combat: &CombatState) -> i32 {
    combat
        .entities
        .monsters
        .iter()
        .filter(|monster| !monster.is_dying && !monster.is_escaped && !monster.half_dead)
        .map(|monster| {
            crate::projection::combat::monster_preview_total_damage_in_combat(combat, monster)
        })
        .sum()
}

pub fn score_reward_action(
    run_state: &RunState,
    reward_state: &RewardState,
    action: &ClientInput,
) -> i32 {
    if let Some(cards) = &reward_state.pending_card_choice {
        return match action {
            ClientInput::SelectCard(index) => cards
                .get(*index)
                .map(|card| rule_card_offer_score(card.id, run_state))
                .unwrap_or(-1_000),
            ClientInput::Proceed => 5,
            _ => -100,
        };
    }

    match action {
        ClientInput::ClaimReward(index) => reward_state
            .items
            .get(*index)
            .map(|item| match item {
                RewardItem::Potion { .. } if reward_item_likely_waste(run_state, item) => -10,
                _ => reward_item_claim_score(run_state, item),
            })
            .unwrap_or(-1_000),
        ClientInput::Proceed => 0,
        _ => -100,
    }
}

pub fn score_map_action(run_state: &RunState, action: &ClientInput) -> i32 {
    let ClientInput::SelectMapNode(x) = action else {
        return score_noncombat_fallback(action);
    };
    let target_y = if run_state.map.current_y == -1 {
        0
    } else if run_state.map.current_y == 14 {
        15
    } else {
        run_state.map.current_y + 1
    };
    if target_y == 15 {
        return 200;
    }
    let room_type = run_state
        .map
        .graph
        .get(target_y as usize)
        .and_then(|row| row.get(*x))
        .and_then(|node| node.class);
    let hp_ratio = run_state.current_hp * 100 / run_state.max_hp.max(1);
    match room_type {
        Some(RoomType::MonsterRoomElite) if hp_ratio >= 70 => 70,
        Some(RoomType::MonsterRoomElite) => -20,
        Some(RoomType::RestRoom) if hp_ratio < 70 => 90,
        Some(RoomType::RestRoom) => 45,
        Some(RoomType::TreasureRoom) => 80,
        Some(RoomType::ShopRoom) if run_state.gold >= 150 => 75,
        Some(RoomType::ShopRoom) => 25,
        Some(RoomType::EventRoom) => 55,
        Some(RoomType::MonsterRoom) => 50,
        Some(RoomType::MonsterRoomBoss) => 200,
        Some(RoomType::TrueVictoryRoom) => 300,
        None => 0,
    }
}

pub fn score_event_action(action: &ClientInput) -> i32 {
    match action {
        ClientInput::EventChoice(index) => 30 - *index as i32,
        ClientInput::Proceed => 5,
        _ => score_noncombat_fallback(action),
    }
}

pub fn score_boss_relic_action(
    state: &crate::rewards::state::BossRelicChoiceState,
    action: &ClientInput,
) -> i32 {
    match action {
        ClientInput::SubmitRelicChoice(index) => state
            .relics
            .get(*index)
            .map(|relic| 80 + rule_relic_score(*relic))
            .unwrap_or(-1_000),
        ClientInput::Proceed => -40,
        _ => score_noncombat_fallback(action),
    }
}

pub fn score_campfire_action(run_state: &RunState, action: &ClientInput) -> i32 {
    match action {
        ClientInput::CampfireOption(CampfireChoice::Rest) => {
            let hp_ratio = run_state.current_hp * 100 / run_state.max_hp.max(1);
            if hp_ratio < 45 {
                160
            } else if hp_ratio < 70 {
                90
            } else {
                10
            }
        }
        ClientInput::CampfireOption(CampfireChoice::Smith(index)) => run_state
            .master_deck
            .get(*index)
            .map(|card| rule_upgrade_score(card.id))
            .unwrap_or(-1_000),
        ClientInput::CampfireOption(CampfireChoice::Toke(index)) => run_state
            .master_deck
            .get(*index)
            .map(|card| 60 + rule_remove_score(card.id, run_state))
            .unwrap_or(-1_000),
        ClientInput::CampfireOption(CampfireChoice::Dig) => 75,
        ClientInput::CampfireOption(CampfireChoice::Lift) => 55,
        ClientInput::CampfireOption(CampfireChoice::Recall) => -20,
        _ => score_noncombat_fallback(action),
    }
}

pub fn score_shop_action(
    run_state: &RunState,
    shop: &crate::shop::ShopState,
    action: &ClientInput,
) -> i32 {
    match action {
        ClientInput::PurgeCard(index) => run_state
            .master_deck
            .get(*index)
            .map(|card| 100 + rule_remove_score(card.id, run_state))
            .unwrap_or(-1_000),
        ClientInput::BuyCard(index) => shop
            .cards
            .get(*index)
            .map(|card| rule_card_offer_score(card.card_id, run_state) - card.price / 5)
            .unwrap_or(-1_000),
        ClientInput::BuyRelic(index) => shop
            .relics
            .get(*index)
            .map(|relic| 70 + rule_relic_score(relic.relic_id) - relic.price / 8)
            .unwrap_or(-1_000),
        ClientInput::BuyPotion(index) => shop
            .potions
            .get(*index)
            .map(|potion| {
                if run_state
                    .relics
                    .iter()
                    .any(|relic| relic.id == RelicId::Sozu)
                {
                    -80 - potion.price / 4
                } else {
                    35 - potion.price / 8
                }
            })
            .unwrap_or(-1_000),
        ClientInput::Proceed => 0,
        _ => score_noncombat_fallback(action),
    }
}

pub fn score_run_selection_action(
    run_state: &RunState,
    request: &crate::state::selection::SelectionRequest,
    action: &ClientInput,
) -> i32 {
    match action {
        ClientInput::SubmitSelection(selection) => {
            let mut score = 20 + selection.selected.len() as i32 * 5;
            for selected in &selection.selected {
                let SelectionTargetRef::CardUuid(uuid) = selected;
                if let Some(card) = run_state.master_deck.iter().find(|card| card.uuid == *uuid) {
                    score += rule_remove_score(card.id, run_state).max(0) / 2;
                }
            }
            score
        }
        ClientInput::Cancel if request.can_cancel => 5,
        _ => score_noncombat_fallback(action),
    }
}

pub fn score_noncombat_fallback(action: &ClientInput) -> i32 {
    match action {
        ClientInput::Proceed => 0,
        ClientInput::Cancel => -5,
        _ => 10,
    }
}

pub fn rule_card_offer_score(card_id: CardId, run_state: &RunState) -> i32 {
    let def = crate::content::cards::get_card_definition(card_id);
    if matches!(
        def.card_type,
        crate::content::cards::CardType::Curse | crate::content::cards::CardType::Status
    ) {
        return -120;
    }

    let mut score = match def.rarity {
        crate::content::cards::CardRarity::Basic => -60,
        crate::content::cards::CardRarity::Common => 25,
        crate::content::cards::CardRarity::Uncommon => 42,
        crate::content::cards::CardRarity::Rare => 58,
        crate::content::cards::CardRarity::Special => 20,
        crate::content::cards::CardRarity::Curse => -120,
    };
    score += match def.card_type {
        crate::content::cards::CardType::Attack => {
            if run_state.master_deck.len() <= 14 {
                20
            } else {
                5
            }
        }
        crate::content::cards::CardType::Skill => 18,
        crate::content::cards::CardType::Power => 28,
        crate::content::cards::CardType::Status | crate::content::cards::CardType::Curse => -100,
    };
    score += def.base_damage.max(0) + def.base_block.max(0);
    score += match card_id {
        CardId::ShrugItOff | CardId::PommelStrike | CardId::BattleTrance => 45,
        CardId::Disarm | CardId::Shockwave | CardId::Offering | CardId::Adrenaline => 65,
        CardId::Immolate | CardId::Feed | CardId::Reaper | CardId::Bludgeon => 55,
        CardId::Inflame | CardId::FeelNoPain | CardId::DarkEmbrace | CardId::DemonForm => 40,
        CardId::Bash | CardId::Defend | CardId::Strike => -80,
        CardId::PerfectedStrike | CardId::Clash => -45,
        CardId::TwinStrike | CardId::SwordBoomerang => -20,
        _ => 0,
    };
    let copies = run_state
        .master_deck
        .iter()
        .filter(|card| card.id == card_id)
        .count() as i32;
    score -= copies * 12;
    if run_state.master_deck.len() >= 22 && def.card_type == crate::content::cards::CardType::Attack
    {
        score -= 20;
    }
    score
}

pub fn rule_remove_score(card_id: CardId, run_state: &RunState) -> i32 {
    let def = crate::content::cards::get_card_definition(card_id);
    if def.card_type == crate::content::cards::CardType::Curse {
        return 180;
    }
    match card_id {
        CardId::Strike => 115,
        CardId::Defend => {
            let defend_count = run_state
                .master_deck
                .iter()
                .filter(|card| card.id == CardId::Defend)
                .count();
            if defend_count > 4 {
                75
            } else {
                35
            }
        }
        _ if crate::content::cards::is_starter_basic(card_id) => 70,
        _ if def.card_type == crate::content::cards::CardType::Status => 90,
        _ => -40,
    }
}

pub fn rule_upgrade_score(card_id: CardId) -> i32 {
    match card_id {
        CardId::Bash => 95,
        CardId::Inflame | CardId::ShrugItOff | CardId::PommelStrike | CardId::BattleTrance => 85,
        CardId::Immolate | CardId::Feed | CardId::Offering | CardId::Adrenaline => 82,
        CardId::Uppercut | CardId::Shockwave | CardId::Disarm => 78,
        CardId::Defend => 50,
        CardId::Strike => 20,
        _ => {
            let def = crate::content::cards::get_card_definition(card_id);
            35 + def.upgrade_damage.max(0) * 3
                + def.upgrade_block.max(0) * 3
                + def.upgrade_magic.max(0) * 4
        }
    }
}

pub fn rule_relic_score(relic_id: RelicId) -> i32 {
    match relic_id {
        RelicId::BurningBlood => 30,
        RelicId::QuestionCard | RelicId::SingingBowl | RelicId::MoltenEgg | RelicId::ToxicEgg => 45,
        RelicId::BagOfPreparation | RelicId::Anchor | RelicId::Lantern => 55,
        RelicId::CoffeeDripper | RelicId::RunicDome | RelicId::BustedCrown => -25,
        _ => 20,
    }
}
