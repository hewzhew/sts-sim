use super::*;

pub fn build_action_candidates(
    legal_actions: &[ClientInput],
    ctx: Option<&EpisodeContext>,
) -> Vec<RunActionCandidate> {
    let combat = ctx.and_then(|ctx| ctx.combat_state.as_ref());
    legal_actions
        .iter()
        .enumerate()
        .map(|(action_index, action)| {
            let action_key = action_key_for_input(action, combat);
            let card = ctx.and_then(|ctx| card_feature_for_action(action, ctx));
            let plan_delta = ctx
                .and_then(|ctx| {
                    let delta = candidate_plan_delta_for_action(action, ctx);
                    if delta == empty_candidate_plan_delta() { None } else { Some(delta) }
                });
            let reward_structure = ctx
                .and_then(|ctx| {
                    let rs = reward_action_structure_for_action(action, ctx);
                    if rs == empty_reward_action_structure() { None } else { Some(rs) }
                });
            RunActionCandidate {
                action_index,
                action_id: stable_action_id(&action_key),
                action_key,
                action: trace_input_from_client_input(action),
                card,
                plan_delta,
                reward_structure,
            }
        })
        .collect()
}

pub fn empty_reward_action_structure() -> RewardActionStructureV0 {
    RewardActionStructureV0 {
        score_kind: "heuristic".to_string(),
        screen_phase: "none".to_string(),
        ..RewardActionStructureV0::default()
    }
}

pub fn reward_action_structure_for_action(
    action: &ClientInput,
    ctx: &EpisodeContext,
) -> RewardActionStructureV0 {
    let EngineState::RewardScreen(reward_state) = &ctx.engine_state else {
        return empty_reward_action_structure();
    };
    if reward_state.pending_card_choice.is_some() {
        return RewardActionStructureV0 {
            score_kind: "heuristic".to_string(),
            screen_phase: "card_choice".to_string(),
            is_reward_action: matches!(action, ClientInput::SelectCard(_) | ClientInput::Proceed),
            skip_card_choice: matches!(action, ClientInput::Proceed),
            proceed_is_cleanup: false,
            ..RewardActionStructureV0::default()
        };
    }

    let unclaimed_reward_count = reward_state.items.len();
    let unclaimed_card_reward_count = reward_state
        .items
        .iter()
        .filter(|item| matches!(item, RewardItem::Card { .. }))
        .count();
    match action {
        ClientInput::ClaimReward(index) => reward_state
            .items
            .get(*index)
            .map(|item| {
                let item_obs = reward_item_observation(&ctx.run_state, *index, item);
                RewardActionStructureV0 {
                    score_kind: "heuristic".to_string(),
                    screen_phase: "claim_items".to_string(),
                    is_reward_action: true,
                    unclaimed_reward_count,
                    unclaimed_card_reward_count,
                    claim_reward_item_type: Some(item_obs.item_type),
                    claim_opens_card_choice: item_obs.opens_card_choice,
                    claim_free_value_score: item_obs.free_value_score,
                    claim_likely_waste: item_obs.likely_waste,
                    claim_capacity_blocked: item_obs.capacity_blocked,
                    ..RewardActionStructureV0::default()
                }
            })
            .unwrap_or_else(empty_reward_action_structure),
        ClientInput::Proceed => RewardActionStructureV0 {
            score_kind: "heuristic".to_string(),
            screen_phase: if unclaimed_reward_count > 0 {
                "claim_items".to_string()
            } else {
                "cleanup".to_string()
            },
            is_reward_action: true,
            is_proceed_with_unclaimed_rewards: unclaimed_reward_count > 0,
            unclaimed_reward_count,
            unclaimed_card_reward_count,
            proceed_is_cleanup: unclaimed_reward_count == 0,
            ..RewardActionStructureV0::default()
        },
        _ => empty_reward_action_structure(),
    }
}

pub fn card_feature_for_action(action: &ClientInput, ctx: &EpisodeContext) -> Option<RunCardFeatureV0> {
    match action {
        ClientInput::PlayCard { card_index, .. } => ctx
            .combat_state
            .as_ref()
            .and_then(|combat| combat.zones.hand.get(*card_index))
            .map(|card| build_card_feature(card.id, card.upgrades, &ctx.run_state)),
        ClientInput::SelectCard(index) => match &ctx.engine_state {
            EngineState::RewardScreen(reward_state) => reward_state
                .pending_card_choice
                .as_ref()
                .and_then(|cards| cards.get(*index))
                .map(|card| build_card_feature(card.id, card.upgrades, &ctx.run_state)),
            _ => None,
        },
        ClientInput::BuyCard(index) => match &ctx.engine_state {
            EngineState::Shop(shop) => shop
                .cards
                .get(*index)
                .map(|card| build_card_feature(card.card_id, 0, &ctx.run_state)),
            _ => None,
        },
        ClientInput::CampfireOption(CampfireChoice::Smith(index))
        | ClientInput::CampfireOption(CampfireChoice::Toke(index))
        | ClientInput::PurgeCard(index) => ctx
            .run_state
            .master_deck
            .get(*index)
            .map(|card| build_card_feature(card.id, card.upgrades, &ctx.run_state)),
        _ => None,
    }
}

pub fn candidate_plan_delta_for_action(
    action: &ClientInput,
    ctx: &EpisodeContext,
) -> CandidatePlanDeltaV0 {
    match action {
        ClientInput::SelectCard(index) => match &ctx.engine_state {
            EngineState::RewardScreen(reward_state) => reward_state
                .pending_card_choice
                .as_ref()
                .and_then(|cards| cards.get(*index))
                .map(|card| add_card_plan_delta(card.id, card.upgrades, &ctx.run_state))
                .unwrap_or_else(empty_candidate_plan_delta),
            _ => empty_candidate_plan_delta(),
        },
        ClientInput::BuyCard(index) => match &ctx.engine_state {
            EngineState::Shop(shop) => shop
                .cards
                .get(*index)
                .map(|card| add_card_plan_delta(card.card_id, 0, &ctx.run_state))
                .unwrap_or_else(empty_candidate_plan_delta),
            _ => empty_candidate_plan_delta(),
        },
        ClientInput::CampfireOption(CampfireChoice::Smith(index)) => ctx
            .run_state
            .master_deck
            .get(*index)
            .map(|card| upgrade_card_plan_delta(card.id, card.upgrades, &ctx.run_state))
            .unwrap_or_else(empty_candidate_plan_delta),
        ClientInput::CampfireOption(CampfireChoice::Toke(index))
        | ClientInput::PurgeCard(index) => ctx
            .run_state
            .master_deck
            .get(*index)
            .map(|card| remove_card_plan_delta(card.id, card.upgrades, &ctx.run_state))
            .unwrap_or_else(empty_candidate_plan_delta),
        _ => empty_candidate_plan_delta(),
    }
}

pub fn empty_candidate_plan_delta() -> CandidatePlanDeltaV0 {
    CandidatePlanDeltaV0 {
        score_kind: "heuristic".to_string(),
        ..CandidatePlanDeltaV0::default()
    }
}

pub fn add_card_plan_delta(
    card_id: CardId,
    upgrades: u8,
    run_state: &RunState,
) -> CandidatePlanDeltaV0 {
    let affordance = card_plan_affordance(card_id, upgrades);
    let profile = build_deck_plan_profile(run_state);
    let deck_deficit_bonus = deck_deficit_bonus(&profile, affordance, run_state);
    let bloat_penalty = deck_bloat_penalty(card_id, affordance, run_state);
    let duplicate_penalty = plan_duplicate_penalty(card_id, run_state);
    let rule_score = rule_card_offer_score(card_id, run_state);
    delta_from_affordance(
        affordance,
        0,
        deck_deficit_bonus,
        bloat_penalty,
        duplicate_penalty,
        rule_score + deck_deficit_bonus + bloat_penalty + duplicate_penalty,
    )
}

pub fn upgrade_card_plan_delta(
    card_id: CardId,
    upgrades: u8,
    run_state: &RunState,
) -> CandidatePlanDeltaV0 {
    let before = card_plan_affordance(card_id, upgrades);
    let after = card_plan_affordance(card_id, upgrades.saturating_add(1));
    let affordance = after.subtract(before);
    let profile = build_deck_plan_profile(run_state);
    let deck_deficit_bonus = deck_deficit_bonus(&profile, affordance, run_state);
    let rule_score = rule_upgrade_score(card_id);
    delta_from_affordance(
        affordance,
        0,
        deck_deficit_bonus,
        0,
        0,
        rule_score + deck_deficit_bonus,
    )
}

pub fn remove_card_plan_delta(
    card_id: CardId,
    upgrades: u8,
    run_state: &RunState,
) -> CandidatePlanDeltaV0 {
    let affordance = card_plan_affordance(card_id, upgrades);
    let burden_delta = if crate::content::cards::is_starter_basic(card_id) {
        -10
    } else {
        0
    };
    let mut out = delta_from_affordance(
        CardPlanAffordance {
            frontload: -affordance.frontload,
            block: -affordance.block,
            draw: -affordance.draw,
            scaling: -affordance.scaling,
            aoe: -affordance.aoe,
            exhaust: -affordance.exhaust,
            kill_window: -affordance.kill_window,
            setup_cashout_risk: -affordance.setup_cashout_risk,
        },
        burden_delta,
        0,
        0,
        0,
        rule_remove_score(card_id, run_state),
    );
    if burden_delta < 0 {
        out.deck_deficit_bonus += 25;
        out.plan_adjusted_score += 25;
    }
    if run_state.master_deck.len() <= 14 && affordance.frontload > 0 {
        out.deck_deficit_bonus -= 10;
        out.plan_adjusted_score -= 10;
    }
    out
}

pub fn delta_from_affordance(
    affordance: CardPlanAffordance,
    starter_basic_burden_delta: i32,
    deck_deficit_bonus: i32,
    bloat_penalty: i32,
    duplicate_penalty: i32,
    plan_adjusted_score: i32,
) -> CandidatePlanDeltaV0 {
    CandidatePlanDeltaV0 {
        score_kind: "heuristic".to_string(),
        frontload_delta: affordance.frontload,
        block_delta: affordance.block,
        draw_delta: affordance.draw,
        scaling_delta: affordance.scaling,
        aoe_delta: affordance.aoe,
        exhaust_delta: affordance.exhaust,
        kill_window_delta: affordance.kill_window,
        starter_basic_burden_delta,
        setup_cashout_risk_delta: affordance.setup_cashout_risk,
        deck_deficit_bonus,
        bloat_penalty,
        duplicate_penalty,
        plan_adjusted_score,
    }
}

pub fn deck_deficit_bonus(
    profile: &DeckPlanProfileV0,
    affordance: CardPlanAffordance,
    run_state: &RunState,
) -> i32 {
    let mut bonus = 0;
    if profile.frontload_supply < 70 {
        bonus += affordance.frontload;
    }
    if profile.block_supply < 50 {
        bonus += affordance.block;
    }
    if profile.draw_supply < 20 {
        bonus += affordance.draw * 2;
    } else if profile.draw_supply < 35 {
        bonus += affordance.draw;
    }
    if profile.scaling_supply < 20 {
        bonus += affordance.scaling * 2;
    } else if profile.scaling_supply < 35 {
        bonus += affordance.scaling;
    }
    if profile.aoe_supply < 18 && (run_state.act_num >= 2 || run_state.floor_num >= 7) {
        bonus += affordance.aoe * 2;
    } else if profile.aoe_supply < 18 {
        bonus += affordance.aoe;
    }
    if profile.exhaust_supply < 12 {
        bonus += affordance.exhaust;
    }
    if profile.kill_window_supply == 0 {
        bonus += affordance.kill_window / 2;
    }
    bonus
}

pub fn deck_bloat_penalty(
    card_id: CardId,
    affordance: CardPlanAffordance,
    run_state: &RunState,
) -> i32 {
    if run_state.master_deck.len() < 22 {
        return 0;
    }
    let high_value_plan_card = affordance.draw > 0
        || affordance.scaling > 0
        || affordance.aoe > 0
        || affordance.kill_window > 0
        || matches!(
            card_id,
            CardId::Disarm | CardId::Shockwave | CardId::Offering
        );
    if high_value_plan_card {
        -5
    } else {
        -18
    }
}

pub fn plan_duplicate_penalty(card_id: CardId, run_state: &RunState) -> i32 {
    let copies = run_state
        .master_deck
        .iter()
        .filter(|card| card.id == card_id)
        .count() as i32;
    -(copies * 5)
}

pub fn build_card_feature(card_id: CardId, upgrades: u8, run_state: &RunState) -> RunCardFeatureV0 {
    let def = crate::content::cards::get_card_definition(card_id);
    let deck_copies = run_state
        .master_deck
        .iter()
        .filter(|card| card.id == card_id)
        .count();
    RunCardFeatureV0 {
        card_id: format!("{card_id:?}"),
        card_id_hash: stable_action_id(&format!("card:{card_id:?}")),
        card_type_id: card_type_id(def.card_type),
        rarity_id: card_rarity_id(def.rarity),
        cost: def.cost,
        upgrades,
        base_damage: def.base_damage,
        base_block: def.base_block,
        base_magic: def.base_magic,
        upgraded_damage: def.base_damage + def.upgrade_damage * upgrades as i32,
        upgraded_block: def.base_block + def.upgrade_block * upgrades as i32,
        upgraded_magic: def.base_magic + def.upgrade_magic * upgrades as i32,
        exhaust: def.exhaust,
        ethereal: def.ethereal,
        innate: def.innate,
        aoe: matches!(def.target, crate::content::cards::CardTarget::AllEnemy),
        multi_damage: def.is_multi_damage || card_is_multi_hit(card_id),
        starter_basic: crate::content::cards::is_starter_basic(card_id),
        draws_cards: card_draws_cards(card_id),
        gains_energy: card_gains_energy(card_id),
        applies_weak: card_applies_weak(card_id),
        applies_vulnerable: card_applies_vulnerable(card_id),
        scaling_piece: card_is_scaling_piece(card_id),
        deck_copies,
        rule_score: rule_card_offer_score(card_id, run_state),
    }
}

pub fn card_type_id(card_type: CardType) -> u8 {
    match card_type {
        CardType::Attack => 1,
        CardType::Skill => 2,
        CardType::Power => 3,
        CardType::Status => 4,
        CardType::Curse => 5,
    }
}

pub fn card_rarity_id(rarity: CardRarity) -> u8 {
    match rarity {
        CardRarity::Basic => 1,
        CardRarity::Common => 2,
        CardRarity::Uncommon => 3,
        CardRarity::Rare => 4,
        CardRarity::Special => 5,
        CardRarity::Curse => 6,
    }
}

pub fn card_draws_cards(card_id: CardId) -> bool {
    matches!(
        card_id,
        CardId::BattleTrance
            | CardId::BurningPact
            | CardId::DarkEmbrace
            | CardId::DeepBreath
            | CardId::Dropkick
            | CardId::Evolve
            | CardId::Finesse
            | CardId::FlashOfSteel
            | CardId::GoodInstincts
            | CardId::MasterOfStrategy
            | CardId::Offering
            | CardId::PommelStrike
            | CardId::ShrugItOff
            | CardId::Warcry
            | CardId::Acrobatics
            | CardId::Backflip
            | CardId::Prepared
            | CardId::DaggerThrow
            | CardId::Adrenaline
    )
}

pub fn card_gains_energy(card_id: CardId) -> bool {
    matches!(
        card_id,
        CardId::Bloodletting
            | CardId::Berserk
            | CardId::Offering
            | CardId::SeeingRed
            | CardId::Sentinel
            | CardId::Adrenaline
    )
}

pub fn card_applies_weak(card_id: CardId) -> bool {
    matches!(
        card_id,
        CardId::Clothesline
            | CardId::Intimidate
            | CardId::Shockwave
            | CardId::Uppercut
            | CardId::Blind
    )
}

pub fn card_applies_vulnerable(card_id: CardId) -> bool {
    matches!(
        card_id,
        CardId::Bash | CardId::Shockwave | CardId::ThunderClap | CardId::Trip | CardId::Uppercut
    )
}

pub fn card_is_multi_hit(card_id: CardId) -> bool {
    matches!(
        card_id,
        CardId::Pummel
            | CardId::SwordBoomerang
            | CardId::TwinStrike
            | CardId::Whirlwind
            | CardId::Reaper
    )
}

pub fn card_exhausts_other_cards(card_id: CardId) -> bool {
    matches!(
        card_id,
        CardId::BurningPact
            | CardId::FiendFire
            | CardId::SecondWind
            | CardId::SeverSoul
            | CardId::TrueGrit
            | CardId::Purity
    )
}

pub fn card_is_scaling_piece(card_id: CardId) -> bool {
    matches!(
        card_id,
        CardId::DemonForm
            | CardId::Inflame
            | CardId::LimitBreak
            | CardId::Rupture
            | CardId::SpotWeakness
            | CardId::Barricade
            | CardId::Entrench
            | CardId::Juggernaut
            | CardId::Metallicize
            | CardId::FeelNoPain
            | CardId::DarkEmbrace
            | CardId::Corruption
            | CardId::Evolve
            | CardId::FireBreathing
            | CardId::Footwork
            | CardId::NoxiousFumes
            | CardId::AfterImage
    )
}

pub fn card_is_block_core(card_id: CardId) -> bool {
    matches!(
        card_id,
        CardId::Defend
            | CardId::DefendG
            | CardId::Apparition
            | CardId::GhostlyArmor
            | CardId::FlameBarrier
            | CardId::Impervious
            | CardId::PowerThrough
            | CardId::ShrugItOff
            | CardId::Backflip
            | CardId::CloakAndDagger
            | CardId::GoodInstincts
            | CardId::DarkShackles
    )
}

