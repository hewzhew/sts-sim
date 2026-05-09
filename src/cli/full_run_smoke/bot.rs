use super::*;
use crate::content::potions::PotionId;

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
        EpisodePolicy::RuleBaselineV0 | EpisodePolicy::RuleBaselineV1Candidate => {
            let idx = choose_rule_baseline_action(ctx, legal_actions);
            Ok((idx, legal_actions[idx].clone()))
        }
        EpisodePolicy::RuleBaselineV0Control => {
            let idx = choose_rule_baseline_v0_control_action(ctx, legal_actions);
            Ok((idx, legal_actions[idx].clone()))
        }
        EpisodePolicy::PlanQueryV0 => {
            let idx = choose_plan_query_action(ctx, legal_actions)
                .unwrap_or_else(|| choose_rule_baseline_action(ctx, legal_actions));
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

pub fn choose_rule_baseline_v0_control_action(
    ctx: &EpisodeContext,
    legal_actions: &[ClientInput],
) -> usize {
    let mut best_index = 0usize;
    let mut best_score = i32::MIN;
    for (index, action) in legal_actions.iter().enumerate() {
        let score = rule_baseline_v0_control_score(ctx, action);
        if score > best_score {
            best_index = index;
            best_score = score;
        }
    }
    best_index
}

pub fn choose_plan_query_action(
    ctx: &EpisodeContext,
    legal_actions: &[ClientInput],
) -> Option<usize> {
    let combat = ctx.combat_state.as_ref()?;
    if !matches!(
        ctx.engine_state,
        EngineState::CombatPlayerTurn | EngineState::PendingChoice(_)
    ) {
        return None;
    }
    let legal_by_key = legal_actions.iter().enumerate().fold(
        BTreeMap::<String, usize>::new(),
        |mut acc, (index, action)| {
            acc.entry(action_key_for_input(action, Some(combat)))
                .or_insert(index);
            acc
        },
    );
    if legal_by_key.is_empty() {
        return None;
    }

    let report = crate::bot::combat::probe_turn_plans(
        &ctx.engine_state,
        combat,
        crate::bot::combat::CombatTurnPlanProbeConfig {
            max_depth: 4,
            max_nodes: 500,
            beam_width: 16,
            max_engine_steps_per_action: 200,
        },
    );

    if let Some(index) = mapped_query_action(&report, &legal_by_key, "CanLethal", &["feasible"]) {
        return Some(index);
    }

    let incoming = visible_incoming_damage(combat);
    let unblocked = visible_unblocked_damage(combat);
    let hp = combat.entities.player.current_hp.max(1);
    let high_pressure = unblocked > 0 && (unblocked >= 8 || unblocked * 3 >= hp);
    let low_or_moderate_pressure = !high_pressure && (unblocked <= 6 || unblocked * 5 <= hp);
    let guarded_pressure = guarded_survival_pressure(combat, incoming, unblocked, hp);
    let multi_enemy_pressure = combat
        .entities
        .monsters
        .iter()
        .filter(|monster| monster.current_hp > 0 && !monster.is_dying)
        .count()
        >= 2;

    if resource_window_opened_this_turn(combat) {
        if guarded_pressure {
            if let Some(index) =
                guarded_survival_query_action(combat, legal_actions, &report, &legal_by_key)
            {
                return Some(index);
            }
        }
        if let Some(index) = resource_window_follow_through_action(&report, &legal_by_key, incoming)
        {
            return Some(index);
        }
    }

    if !high_pressure || unblocked * 5 <= hp {
        if let Some(index) = resource_window_opener_action(combat, legal_actions, unblocked, hp) {
            return Some(index);
        }
    }

    if guarded_pressure {
        if let Some(index) =
            guarded_survival_query_action(combat, legal_actions, &report, &legal_by_key)
        {
            return Some(index);
        }
    }

    if high_pressure {
        for (query, statuses) in [
            ("CanFullBlockThenMaxDamage", &["feasible"][..]),
            ("CanFullBlock", &["feasible"][..]),
            ("CanFullBlockThenMaxDamage", &["partial"][..]),
            ("CanFullBlock", &["partial"][..]),
        ] {
            if let Some(index) = mapped_query_action(&report, &legal_by_key, query, statuses) {
                return Some(index);
            }
        }
    }

    if guarded_pressure {
        return None;
    }

    if incoming > 0 && low_or_moderate_pressure {
        if let Some(index) = mapped_plan_action(&report, &legal_by_key, "KillThreateningEnemy") {
            return Some(index);
        }
        if multi_enemy_pressure {
            if let Some(index) = mapped_plan_action(&report, &legal_by_key, "MaxDamage") {
                return Some(index);
            }
        }
    }

    if incoming == 0 || unblocked == 0 {
        if let Some(index) = mapped_query_action(
            &report,
            &legal_by_key,
            "CanPlaySetupAndStillBlock",
            &["feasible"],
        ) {
            return Some(index);
        }
    }

    if incoming > 0 {
        if let Some(index) = mapped_query_action(
            &report,
            &legal_by_key,
            "CanFullBlockThenMaxDamage",
            &["feasible"],
        ) {
            return Some(index);
        }
    }

    for plan_name in ["MaxDamage", "SetupPowerOrScaling"] {
        if let Some(index) = mapped_plan_action(&report, &legal_by_key, plan_name) {
            return Some(index);
        }
    }

    None
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

pub fn guarded_survival_pressure(
    combat: &CombatState,
    incoming: i32,
    unblocked: i32,
    hp: i32,
) -> bool {
    if incoming <= 0 || unblocked <= 0 {
        return false;
    }
    let total_hp = total_alive_monster_hp(combat);
    let boss_or_long_race =
        combat.meta.is_boss_fight || (alive_monster_count(combat) == 1 && total_hp >= 120);
    let severe_attack_window = incoming >= 24 || unblocked * 2 >= hp;
    boss_or_long_race && (unblocked >= 8 || severe_attack_window)
}

pub fn guarded_survival_query_action(
    combat: &CombatState,
    legal_actions: &[ClientInput],
    report: &crate::bot::combat::CombatTurnPlanProbeReport,
    legal_by_key: &BTreeMap<String, usize>,
) -> Option<usize> {
    if let Some(index) = guarded_boss_race_action(combat, report, legal_by_key) {
        return Some(index);
    }
    for (query, statuses) in [
        ("CanFullBlockThenMaxDamage", &["feasible"][..]),
        ("CanFullBlock", &["feasible"][..]),
    ] {
        if let Some(index) = mapped_query_action(report, legal_by_key, query, statuses) {
            return Some(index);
        }
    }
    if let Some(index) = guarded_direct_block_action(combat, legal_actions) {
        return Some(index);
    }
    for (query, statuses) in [
        ("CanFullBlock", &["partial"][..]),
        ("CanFullBlockThenMaxDamage", &["partial"][..]),
    ] {
        if let Some(index) = mapped_query_action(report, legal_by_key, query, statuses) {
            return Some(index);
        }
    }
    None
}

pub fn guarded_boss_race_action(
    combat: &CombatState,
    report: &crate::bot::combat::CombatTurnPlanProbeReport,
    legal_by_key: &BTreeMap<String, usize>,
) -> Option<usize> {
    let hp = combat.entities.player.current_hp.max(1);
    let race = sequence_for_plan(report, "MaxDamage")?;
    let guard = guarded_partial_sequence(report);
    let guard_outcome = guard.map(|sequence| &sequence.outcome);
    let race_unblocked = race.outcome.projected_unblocked_damage;
    if hp - race_unblocked <= 6 {
        return None;
    }

    let guard_unblocked = guard_outcome
        .map(|outcome| outcome.projected_unblocked_damage)
        .unwrap_or_else(|| visible_unblocked_damage(combat));
    let guard_damage = guard_outcome
        .map(|outcome| outcome.damage_done)
        .unwrap_or_default();
    let extra_leak = (race_unblocked - guard_unblocked).max(0);
    let damage_gain = race.outcome.damage_done - guard_damage;
    let total_hp = total_alive_monster_hp(combat).max(1);
    let race_damage_share_milli = race.outcome.damage_done * 1000 / total_hp;

    let meaningful_boss_clock = race.outcome.enemy_deaths > 0
        || race.outcome.damage_done >= 45
        || damage_gain >= 30
        || race_damage_share_milli >= 250;
    let leak_is_acceptable = extra_leak <= 6 || race_unblocked <= 8 || guard_unblocked >= 16;
    let not_pure_chip = damage_gain >= 20 || race.outcome.damage_done >= 35;

    if meaningful_boss_clock && leak_is_acceptable && not_pure_chip {
        return first_mapped_action(&race.action_keys, legal_by_key);
    }
    None
}

pub fn sequence_for_plan<'a>(
    report: &'a crate::bot::combat::CombatTurnPlanProbeReport,
    plan_name: &str,
) -> Option<&'a crate::bot::combat::CombatPlanSequenceClass> {
    let key = report
        .plans
        .iter()
        .find(|plan| plan.plan_name == plan_name)?
        .best_sequence_key
        .as_ref()?;
    report
        .sequence_classes
        .iter()
        .find(|sequence| sequence.sequence_equivalence_key == *key)
}

pub fn sequence_for_query<'a>(
    report: &'a crate::bot::combat::CombatTurnPlanProbeReport,
    query_name: &str,
    allowed_statuses: &[&str],
) -> Option<&'a crate::bot::combat::CombatPlanSequenceClass> {
    let query = report
        .plan_queries
        .iter()
        .find(|query| query.query_name == query_name)?;
    if !allowed_statuses
        .iter()
        .any(|status| query.status.as_str() == *status)
    {
        return None;
    }
    let key = query.best_sequence_key.as_ref()?;
    report
        .sequence_classes
        .iter()
        .find(|sequence| sequence.sequence_equivalence_key == *key)
}

pub fn guarded_partial_sequence(
    report: &crate::bot::combat::CombatTurnPlanProbeReport,
) -> Option<&crate::bot::combat::CombatPlanSequenceClass> {
    sequence_for_query(report, "CanFullBlock", &["partial"])
        .or_else(|| sequence_for_query(report, "CanFullBlockThenMaxDamage", &["partial"]))
}

pub fn guarded_direct_block_action(
    combat: &CombatState,
    legal_actions: &[ClientInput],
) -> Option<usize> {
    legal_actions
        .iter()
        .enumerate()
        .filter_map(|(index, action)| {
            let ClientInput::PlayCard { card_index, target } = action else {
                return None;
            };
            if target.is_some() {
                return None;
            }
            let card = combat.zones.hand.get(*card_index)?;
            let def = crate::content::cards::get_card_definition(card.id);
            let evaluated = crate::content::cards::evaluate_card_for_play(card, combat, *target);
            let block = evaluated
                .base_block_mut
                .max(def.base_block + card.upgrades as i32 * def.upgrade_block);
            if block <= 0 && !card_is_block_core(card.id) {
                return None;
            }
            let cost = evaluated.get_cost().max(0) as i32;
            let block_score = block.max(8);
            let utility_bonus = match card.id {
                CardId::ShrugItOff | CardId::TrueGrit | CardId::FlameBarrier => 30,
                CardId::Impervious | CardId::PowerThrough => 20,
                _ => 0,
            };
            Some((index, block_score * 100 - cost * 10 + utility_bonus))
        })
        .max_by_key(|(_, score)| *score)
        .map(|(index, _)| index)
}

pub fn resource_window_opened_this_turn(combat: &CombatState) -> bool {
    combat
        .turn
        .counters
        .card_ids_played_this_turn
        .iter()
        .any(|card_id| is_resource_window_card(*card_id))
}

pub fn is_resource_window_card(card_id: CardId) -> bool {
    matches!(
        card_id,
        CardId::Offering
            | CardId::Adrenaline
            | CardId::BattleTrance
            | CardId::SeeingRed
            | CardId::Bloodletting
    )
}

pub fn resource_window_follow_through_action(
    report: &crate::bot::combat::CombatTurnPlanProbeReport,
    legal_by_key: &BTreeMap<String, usize>,
    incoming: i32,
) -> Option<usize> {
    if let Some(plan) = report
        .plans
        .iter()
        .find(|plan| plan.plan_name == "MaxDamage")
    {
        if let Some(score) = plan.best_score.as_ref() {
            if score.enemy_death_score > 0 || score.damage_score >= 180 {
                if let Some(index) = first_mapped_action(&plan.best_action_keys, legal_by_key) {
                    return Some(index);
                }
            }
        }
    }

    let mut best: Option<(usize, i32)> = None;
    for plan_name in [
        "KillThreateningEnemy",
        "MaxDamage",
        "BlockEnoughThenDamage",
        "SetupPowerOrScaling",
    ] {
        let Some(plan) = report.plans.iter().find(|plan| plan.plan_name == plan_name) else {
            continue;
        };
        let Some(index) = first_mapped_action(&plan.best_action_keys, legal_by_key) else {
            continue;
        };
        let Some(score) = plan.best_score.as_ref() else {
            continue;
        };
        let mut adjusted = score.total_score;
        match plan_name {
            "KillThreateningEnemy" => {
                adjusted += score.enemy_death_score * 2 + score.damage_score;
            }
            "MaxDamage" => {
                adjusted += score.damage_score * 2 + score.enemy_death_score;
            }
            "BlockEnoughThenDamage" => {
                if incoming <= 0 {
                    adjusted -= 80;
                }
                adjusted += score.block_score + score.damage_score;
            }
            "SetupPowerOrScaling" => {
                adjusted += score.setup_score * 2 + score.damage_score / 2;
            }
            _ => {}
        }
        if best
            .as_ref()
            .is_none_or(|(_, best_score)| adjusted > *best_score)
        {
            best = Some((index, adjusted));
        }
    }
    best.map(|(index, _)| index)
}

pub fn resource_window_opener_action(
    combat: &CombatState,
    legal_actions: &[ClientInput],
    unblocked: i32,
    hp: i32,
) -> Option<usize> {
    legal_actions
        .iter()
        .enumerate()
        .filter_map(|(index, action)| {
            let ClientInput::PlayCard { card_index, target } = action else {
                return None;
            };
            if target.is_some() {
                return None;
            }
            let score = resource_window_opener_score(combat, *card_index, unblocked, hp)?;
            Some((index, score))
        })
        .max_by_key(|(_, score)| *score)
        .map(|(index, _)| index)
}

pub fn resource_window_opener_score(
    combat: &CombatState,
    card_index: usize,
    unblocked: i32,
    hp: i32,
) -> Option<i32> {
    let card = combat.zones.hand.get(card_index)?;
    let (base, hp_cost, extra_energy, draw_count) = match card.id {
        CardId::Adrenaline => (240, 0, 1, 2),
        CardId::Offering => (230, 6, 2, 3),
        CardId::BattleTrance => (115, 0, 0, 3),
        CardId::SeeingRed => (90, 0, 2, 0),
        CardId::Bloodletting => (80, 3, 2 + card.upgrades as i32, 0),
        _ => return None,
    };
    if resource_window_opened_this_turn(combat) {
        return None;
    }
    if hp - hp_cost <= unblocked + 6 {
        return None;
    }
    let evaluated = crate::content::cards::evaluate_card_for_play(card, combat, None);
    let cost = evaluated.get_cost().max(0) as i32;
    let energy_after = combat.turn.energy as i32 - cost + extra_energy;
    if energy_after <= 0 && draw_count > 0 {
        return None;
    }

    let immediate_payoff = resource_window_immediate_payoff_score(combat, card_index, energy_after);
    let draw_payoff = if draw_count > 0 {
        resource_window_draw_payoff_score(combat, draw_count)
    } else {
        0
    };
    let payoff = immediate_payoff + draw_payoff;
    if payoff < 45 {
        return None;
    }

    Some(base + payoff - hp_cost * 14 - cost * 8)
}

pub fn resource_window_immediate_payoff_score(
    combat: &CombatState,
    resource_card_index: usize,
    energy_after: i32,
) -> i32 {
    combat
        .zones
        .hand
        .iter()
        .enumerate()
        .filter(|(index, _)| *index != resource_card_index)
        .map(|(_, card)| {
            let evaluated = crate::content::cards::evaluate_card_for_play(card, combat, None);
            let cost = evaluated.get_cost().max(0) as i32;
            resource_window_card_payoff_score(card.id, cost <= energy_after)
        })
        .sum()
}

pub fn resource_window_draw_payoff_score(combat: &CombatState, draw_count: usize) -> i32 {
    if combat.zones.draw_pile.is_empty() && combat.zones.discard_pile.is_empty() {
        return 0;
    }
    let mut scores = combat
        .zones
        .draw_pile
        .iter()
        .chain(combat.zones.discard_pile.iter())
        .map(|card| resource_window_card_payoff_score(card.id, true))
        .filter(|score| *score > 0)
        .collect::<Vec<_>>();
    scores.sort_unstable_by(|a, b| b.cmp(a));
    scores.into_iter().take(draw_count.max(1) * 2).sum::<i32>() / 2
}

pub fn resource_window_card_payoff_score(card_id: CardId, currently_playable: bool) -> i32 {
    let def = crate::content::cards::get_card_definition(card_id);
    let playable_multiplier = if currently_playable { 2 } else { 1 };
    let base = match card_id {
        CardId::Immolate | CardId::Bludgeon | CardId::FiendFire | CardId::Reaper => 55,
        CardId::Bash | CardId::Uppercut | CardId::Shockwave | CardId::Disarm => 42,
        CardId::Inflame | CardId::DemonForm | CardId::FeelNoPain | CardId::DarkEmbrace => 38,
        CardId::Cleave | CardId::Whirlwind | CardId::ThunderClap | CardId::Carnage => 34,
        CardId::PommelStrike | CardId::ShrugItOff | CardId::BattleTrance => 28,
        _ => match def.card_type {
            CardType::Attack if def.base_damage > 0 => 22,
            CardType::Skill if def.base_block > 0 => 18,
            CardType::Power => 24,
            _ => 0,
        },
    };
    base * playable_multiplier
}

pub fn mapped_query_action(
    report: &crate::bot::combat::CombatTurnPlanProbeReport,
    legal_by_key: &BTreeMap<String, usize>,
    query_name: &str,
    allowed_statuses: &[&str],
) -> Option<usize> {
    let query = report
        .plan_queries
        .iter()
        .find(|query| query.query_name == query_name)?;
    if !allowed_statuses
        .iter()
        .any(|status| query.status.as_str() == *status)
    {
        return None;
    }
    first_mapped_action(&query.best_action_keys, legal_by_key)
}

pub fn mapped_plan_action(
    report: &crate::bot::combat::CombatTurnPlanProbeReport,
    legal_by_key: &BTreeMap<String, usize>,
    plan_name: &str,
) -> Option<usize> {
    let plan = report
        .plans
        .iter()
        .find(|plan| plan.plan_name == plan_name)?;
    first_mapped_action(&plan.best_action_keys, legal_by_key)
}

pub fn first_mapped_action(
    action_keys: &[String],
    legal_by_key: &BTreeMap<String, usize>,
) -> Option<usize> {
    action_keys
        .iter()
        .find_map(|action_key| legal_by_key.get(action_key).copied())
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
        EngineState::EventRoom => score_event_action(&ctx.run_state, action),
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

pub fn rule_baseline_v0_control_score(ctx: &EpisodeContext, action: &ClientInput) -> i32 {
    match &ctx.engine_state {
        EngineState::CombatPlayerTurn | EngineState::PendingChoice(_) => {
            score_combat_action_v0_control(ctx, action)
        }
        EngineState::RewardScreen(reward_state) => {
            score_reward_action(&ctx.run_state, reward_state, action)
        }
        EngineState::MapNavigation => score_map_action_v0_control(&ctx.run_state, action),
        EngineState::EventRoom => score_event_action_v0_control(action),
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
        (
            _,
            ClientInput::UsePotion {
                potion_index,
                target,
            },
        ) => score_combat_potion_action(ctx, combat, *potion_index, *target),
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

pub fn score_combat_action_v0_control(ctx: &EpisodeContext, action: &ClientInput) -> i32 {
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
            score_play_card_action_v0_control(combat, *card_index, *target)
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

pub fn score_combat_potion_action(
    _ctx: &EpisodeContext,
    combat: &CombatState,
    potion_index: usize,
    target: Option<usize>,
) -> i32 {
    let Some(Some(potion)) = combat.entities.potions.get(potion_index) else {
        return -1_000;
    };
    let incoming = visible_incoming_damage(combat);
    let pressure = incoming + visible_end_turn_self_damage(combat);
    let unblocked = (pressure - combat.entities.player.block).max(0);
    let hp = combat.entities.player.current_hp.max(1);
    let key_fight = combat.meta.is_boss_fight || combat.meta.is_elite_fight;
    let danger = unblocked >= 10 || unblocked * 3 >= hp;
    let survival_emergency = unblocked >= hp || (hp <= 10 && unblocked > 0);
    let playable_cards = combat
        .zones
        .hand
        .iter()
        .filter(|card| crate::content::cards::can_play_card(card, combat).is_ok())
        .count() as i32;
    let energy_gap = (playable_cards - combat.turn.energy as i32).max(0);
    let mut score = if key_fight || survival_emergency {
        55
    } else {
        -600
    };

    match potion.id {
        PotionId::EnergyPotion => {
            if key_fight && energy_gap > 0 {
                score += 110 + energy_gap * 25;
            } else if danger && energy_gap > 0 && unblocked >= hp {
                score += 90 + energy_gap * 20;
            }
        }
        PotionId::BlessingOfTheForge => {
            let playable_hand = combat
                .zones
                .hand
                .iter()
                .filter(|card| crate::content::cards::can_play_card(card, combat).is_ok())
                .count() as i32;
            if key_fight && playable_hand >= 3 {
                score += 105 + playable_hand * 12;
            }
        }
        PotionId::BlockPotion => {
            if unblocked > 0 && (key_fight || survival_emergency) {
                score += 40 + unblocked.min(12) * 12;
                if survival_emergency {
                    score += 260;
                }
            }
        }
        PotionId::FirePotion | PotionId::ExplosivePotion => {
            let damage = if potion.id == PotionId::FirePotion {
                20
            } else {
                10 * alive_monster_count(combat) as i32
            };
            if damage >= total_alive_monster_hp(combat) {
                score += 900;
            } else if key_fight || survival_emergency {
                score += damage * 5;
            }
        }
        PotionId::FearPotion | PotionId::WeakenPotion => {
            if target
                .and_then(|target_id| alive_monster_by_id(combat, target_id))
                .is_some()
                && (key_fight || survival_emergency)
            {
                score += 110;
                if survival_emergency {
                    score += 120;
                }
            }
        }
        PotionId::StrengthPotion
        | PotionId::DexterityPotion
        | PotionId::SteroidPotion
        | PotionId::SpeedPotion
        | PotionId::EssenceOfSteel
        | PotionId::LiquidBronze
        | PotionId::HeartOfIron
        | PotionId::CultistPotion => {
            if key_fight || survival_emergency {
                score += 120;
                if survival_emergency {
                    score += 160;
                }
            }
            if matches!(potion.id, PotionId::DexterityPotion | PotionId::SpeedPotion) {
                let playable_block_cards = playable_block_card_count(combat) as i32;
                if playable_block_cards > 0 && (danger || survival_emergency) {
                    let dex_gain = if potion.id == PotionId::SpeedPotion {
                        5
                    } else {
                        2
                    };
                    let usable_block_cards =
                        playable_block_cards.min(combat.turn.energy.max(0) as i32);
                    let immediate_extra_block = dex_gain * usable_block_cards;
                    score += immediate_extra_block * if survival_emergency { 26 } else { 12 };
                    if survival_emergency {
                        score += 420;
                    }
                }
            }
        }
        PotionId::SwiftPotion
        | PotionId::AttackPotion
        | PotionId::SkillPotion
        | PotionId::PowerPotion => {
            if key_fight && combat.turn.energy > 0 {
                score += 70;
            }
        }
        PotionId::BloodPotion
        | PotionId::RegenPotion
        | PotionId::FruitJuice
        | PotionId::FairyPotion => {
            if (key_fight || survival_emergency)
                && (danger || hp * 2 <= combat.entities.player.max_hp)
            {
                score += 90;
            }
        }
        _ => {
            if key_fight && (danger || combat.turn.turn_count <= 1) {
                score += 60;
            }
        }
    }

    score
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
    let pressure = incoming + visible_end_turn_self_damage(combat);
    let unblocked = (pressure - combat.entities.player.block).max(0);
    let hp = combat.entities.player.current_hp.max(1);
    let danger = unblocked >= hp / 3 || unblocked >= 12;
    let mut score = 20 - evaluated.get_cost().max(0) as i32 * 12;

    let damage = estimated_card_damage(combat, &evaluated, target);
    let hp_damage = estimated_card_hp_damage(combat, &evaluated, target);
    let block_damage = (damage - hp_damage).max(0);
    let block = evaluated
        .base_block_mut
        .max(def.base_block + card.upgrades as i32 * def.upgrade_block);
    let kills_all = estimated_action_kills_all(combat, &evaluated, target);
    if damage > 0 {
        score += hp_damage * if danger { 10 } else { 13 };
        score += block_damage * if danger { 2 } else { 4 };
        if kills_all {
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
    score += intangible_survival_card_score(combat, card.id, unblocked, hp);

    let feel_no_pain = combat.get_power(
        combat.entities.player.id,
        crate::content::powers::PowerId::FeelNoPain,
    );
    if def.exhaust && feel_no_pain > 0 {
        let exhaust_block = feel_no_pain;
        let useful_block = exhaust_block.min(unblocked.max(0));
        score += useful_block * if danger { 18 } else { 6 };
        score += (exhaust_block - useful_block).max(0) * 2;
    }

    if card_gains_energy(card.id) {
        let playable_block_cards = playable_block_card_count(combat) as i32;
        let immediate_energy_gain = immediate_energy_gain_for_card(card.id, card.upgrades);
        let net_energy_gain = immediate_energy_gain - evaluated.get_cost().max(0) as i32;
        let hp_cost_is_tolerable = !matches!(card.id, CardId::Bloodletting | CardId::Offering)
            || hp > unblocked + 6;
        if net_energy_gain > 0
            && playable_block_cards > 0
            && hp_cost_is_tolerable
            && (danger || unblocked >= hp)
        {
            score += 260 + playable_block_cards.min(3) * 55;
            if unblocked >= hp {
                score += 1_250;
            }
        } else if net_energy_gain > 0
            && combat.turn.energy <= 1
            && playable_block_cards + playable_attack_count(combat) > 1
        {
            score += 90;
        }
    }
    score -= status_payoff_engine_exhaust_penalty(combat, card_index);

    let long_fight = combat.meta.is_boss_fight
        || combat.meta.is_elite_fight
        || total_alive_monster_hp(combat) >= 60;
    if card_applies_weak(card.id) && incoming > 0 {
        score += 70 + incoming.min(30) * if danger { 7 } else { 4 };
        if long_fight {
            score += 45;
        }
    }
    let target_block = target
        .and_then(|target_id| alive_monster_by_id(combat, target_id))
        .map(|monster| monster.block)
        .unwrap_or(0);
    let vulnerable_has_combat_value =
        target.is_none() || hp_damage > 0 || target_block <= evaluated.base_damage_mut.max(0) / 2;
    if card_applies_vulnerable(card.id)
        && vulnerable_has_combat_value
        && (long_fight || total_alive_monster_hp(combat) >= 35)
    {
        score += 65;
        if combat.meta.is_boss_fight {
            score += 45;
        }
    }

    if unblocked >= hp && !kills_all {
        let projected_unblocked = (unblocked - block).max(0);
        if block > 0 && projected_unblocked < hp {
            score += 650 + (unblocked - projected_unblocked) * 20;
        } else if block > 0 {
            score += 160 + (unblocked - projected_unblocked).max(0) * 10;
        } else {
            score -= 350;
        }
    }

    let specific_bonus = match card.id {
        CardId::Bash | CardId::Uppercut => {
            if vulnerable_has_combat_value {
                45
            } else {
                -10
            }
        }
        CardId::Shockwave => 45,
        CardId::Disarm => 70,
        CardId::Inflame | CardId::DemonForm | CardId::FeelNoPain | CardId::DarkEmbrace => 55,
        CardId::ShrugItOff | CardId::PommelStrike | CardId::BattleTrance => 35,
        CardId::InfernalBlade => {
            if combat.turn.energy > 0 {
                115
            } else {
                25
            }
        }
        CardId::Warcry => 65,
        CardId::JackOfAllTrades => 75,
        CardId::Armaments => {
            if combat.zones.hand.len() >= 3 {
                45
            } else {
                25
            }
        }
        CardId::Offering | CardId::Adrenaline => 80,
        CardId::Immolate | CardId::Feed | CardId::Reaper => 65,
        CardId::Flex | CardId::Bloodletting | CardId::SeeingRed | CardId::SpotWeakness => 35,
        CardId::Slimed if feel_no_pain > 0 => 45,
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
            score += if pressure == 0 { 20 } else { 8 };
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

pub fn score_play_card_action_v0_control(
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
    score += intangible_survival_card_score(combat, card.id, unblocked, hp);

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

fn intangible_survival_card_score(
    combat: &CombatState,
    card_id: CardId,
    unblocked: i32,
    hp: i32,
) -> i32 {
    if !matches!(card_id, CardId::Apparition) || unblocked <= 0 {
        return 0;
    }
    if combat.get_power(
        combat.entities.player.id,
        crate::content::powers::PowerId::IntangiblePlayer,
    ) > 0
    {
        return 0;
    }

    let prevented = (unblocked - 1).max(0);
    let mut score = 220 + prevented * 36;
    if unblocked >= hp {
        score += 1_400;
    } else if unblocked * 2 >= hp {
        score += 700;
    }
    if combat.meta.is_boss_fight || combat.meta.is_elite_fight {
        score += 260;
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

pub fn estimated_card_hp_damage(
    combat: &CombatState,
    card: &crate::runtime::combat::CombatCard,
    target: Option<usize>,
) -> i32 {
    let def = crate::content::cards::get_card_definition(card.id);
    if def.is_multi_damage || matches!(def.target, crate::content::cards::CardTarget::AllEnemy) {
        let alive = combat
            .entities
            .monsters
            .iter()
            .filter(|monster| !monster.is_dying && !monster.is_escaped && !monster.half_dead)
            .collect::<Vec<_>>();
        if !card.multi_damage.is_empty() {
            return alive
                .iter()
                .enumerate()
                .map(|(idx, monster)| {
                    let damage = card.multi_damage.get(idx).copied().unwrap_or(0).max(0);
                    (damage - monster.block.max(0))
                        .max(0)
                        .min(monster.current_hp.max(0))
                })
                .sum();
        }
        let damage = card.base_damage_mut.max(0);
        return alive
            .iter()
            .map(|monster| {
                (damage - monster.block.max(0))
                    .max(0)
                    .min(monster.current_hp.max(0))
            })
            .sum();
    }

    let damage = card.base_damage_mut.max(0);
    if let Some(target_id) = target {
        if let Some(monster) = alive_monster_by_id(combat, target_id) {
            return (damage - monster.block.max(0))
                .max(0)
                .min(monster.current_hp.max(0));
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

pub fn playable_block_card_count(combat: &CombatState) -> usize {
    combat
        .zones
        .hand
        .iter()
        .filter(|card| crate::content::cards::can_play_card(card, combat).is_ok())
        .filter(|card| {
            let def = crate::content::cards::get_card_definition(card.id);
            def.base_block + card.upgrades as i32 * def.upgrade_block > 0
                || card_is_block_core(card.id)
        })
        .count()
}

pub fn playable_attack_count(combat: &CombatState) -> i32 {
    combat
        .zones
        .hand
        .iter()
        .filter(|card| crate::content::cards::can_play_card(card, combat).is_ok())
        .filter(|card| {
            crate::content::cards::get_card_definition(card.id).card_type
                == crate::content::cards::CardType::Attack
        })
        .count() as i32
}

pub fn immediate_energy_gain_for_card(card_id: CardId, upgrades: u8) -> i32 {
    match card_id {
        CardId::SeeingRed => 2,
        CardId::Bloodletting => {
            if upgrades > 0 {
                3
            } else {
                3
            }
        }
        CardId::Offering => 2,
        CardId::Adrenaline => 1,
        _ => 0,
    }
}

pub fn status_payoff_engine_exhaust_penalty(combat: &CombatState, card_index: usize) -> i32 {
    let Some(card) = combat.zones.hand.get(card_index) else {
        return 0;
    };
    if card.id != CardId::SecondWind || visible_status_card_count(combat) == 0 {
        return 0;
    }
    let payoff_engines = combat
        .zones
        .hand
        .iter()
        .enumerate()
        .filter(|(index, other)| {
            *index != card_index && matches!(other.id, CardId::FireBreathing | CardId::Evolve)
        })
        .count() as i32;
    payoff_engines * 110
}

pub fn visible_status_card_count(combat: &CombatState) -> usize {
    combat
        .zones
        .hand
        .iter()
        .chain(combat.zones.draw_pile.iter())
        .chain(combat.zones.discard_pile.iter())
        .filter(|card| {
            crate::content::cards::get_card_definition(card.id).card_type
                == crate::content::cards::CardType::Status
        })
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
    (visible_incoming_damage(combat) + visible_end_turn_self_damage(combat)
        - combat.entities.player.block)
        .max(0)
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

pub fn visible_end_turn_self_damage(combat: &CombatState) -> i32 {
    combat
        .zones
        .hand
        .iter()
        .map(|card| match card.id {
            CardId::Burn => {
                if card.upgrades > 0 {
                    4
                } else {
                    2
                }
            }
            CardId::Decay => 2,
            CardId::Regret => combat.zones.hand.len() as i32,
            _ => 0,
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
    let ClientInput::SelectMapNode(_) = action else {
        return score_noncombat_fallback(action);
    };
    map_route_projection_for_action(run_state, action)
        .map(|projection| projection.total_score)
        .unwrap_or(0)
}

pub fn score_map_action_v0_control(run_state: &RunState, action: &ClientInput) -> i32 {
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
    match room_type {
        Some(RoomType::MonsterRoomElite)
            if run_state.current_hp * 100 / run_state.max_hp.max(1) >= 70 =>
        {
            70
        }
        Some(RoomType::MonsterRoomElite) => -20,
        Some(RoomType::RestRoom) if run_state.current_hp * 100 / run_state.max_hp.max(1) < 70 => 90,
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

#[derive(Clone, Copy, Debug, Default)]
struct SmokeRouteStats {
    first_shop_depth: Option<usize>,
    first_rest_depth: Option<usize>,
    first_elite_depth: Option<usize>,
    early_monster_count: usize,
    early_event_count: usize,
}

pub fn map_route_projection_for_action(
    run_state: &RunState,
    action: &ClientInput,
) -> Option<MapRouteCandidateProjectionV0> {
    let ClientInput::SelectMapNode(x) = action else {
        return None;
    };
    let target_y = if run_state.map.current_y == -1 {
        0
    } else if run_state.map.current_y == 14 {
        15
    } else {
        run_state.map.current_y + 1
    };
    if target_y == 15 {
        return Some(MapRouteCandidateProjectionV0 {
            score_kind: "full_map_route_v1".to_string(),
            target_x: *x as i32,
            target_y,
            total_score: 200,
            base_score: 200,
            adjustment_score: 0,
            first_shop_depth: None,
            first_rest_depth: None,
            first_elite_depth: None,
            early_monster_count: 0,
            early_event_count: 0,
            early_safe_room_count: 0,
            best_path: Vec::new(),
        });
    }
    score_full_map_route_projection(run_state, *x as i32, target_y)
}

fn score_full_map_route_projection(
    run_state: &RunState,
    x: i32,
    y: i32,
) -> Option<MapRouteCandidateProjectionV0> {
    let mut best = None;
    visit_smoke_route(
        run_state,
        x,
        y,
        0,
        0,
        SmokeRouteStats::default(),
        Vec::new(),
        &mut best,
    );
    best.map(|mut projection| {
        projection.target_x = x;
        projection.target_y = y;
        projection
    })
}

fn visit_smoke_route(
    run_state: &RunState,
    x: i32,
    y: i32,
    depth: usize,
    base_score_so_far: i32,
    mut stats: SmokeRouteStats,
    mut path: Vec<MapRouteRoomProjectionV0>,
    best: &mut Option<MapRouteCandidateProjectionV0>,
) {
    let room_type = run_state
        .map
        .graph
        .get(y as usize)
        .and_then(|row| row.get(x as usize))
        .and_then(|node| node.class);
    let Some(room_type) = room_type else {
        return;
    };
    record_smoke_route_room(&mut stats, room_type, depth);
    let room_score = score_map_room(run_state, room_type);
    let base_score_so_far = base_score_so_far + room_score;
    path.push(MapRouteRoomProjectionV0 {
        x,
        y,
        depth,
        room_type: format!("{room_type:?}"),
        room_score,
    });

    let Some(node) = run_state
        .map
        .graph
        .get(y as usize)
        .and_then(|row| row.get(x as usize))
    else {
        record_smoke_route_projection(run_state, x, y, stats, base_score_so_far, path, best);
        return;
    };

    if node.edges.is_empty() {
        record_smoke_route_projection(run_state, x, y, stats, base_score_so_far, path, best);
        return;
    }

    for edge in &node.edges {
        visit_smoke_route(
            run_state,
            edge.dst_x,
            edge.dst_y,
            depth + 1,
            base_score_so_far,
            stats,
            path.clone(),
            best,
        );
    }
}

fn record_smoke_route_projection(
    run_state: &RunState,
    target_x: i32,
    target_y: i32,
    stats: SmokeRouteStats,
    base_score: i32,
    path: Vec<MapRouteRoomProjectionV0>,
    best: &mut Option<MapRouteCandidateProjectionV0>,
) {
    let adjustment_score = smoke_route_adjustment(run_state, stats);
    let total_score = base_score + adjustment_score;
    if best
        .as_ref()
        .is_some_and(|current| current.total_score >= total_score)
    {
        return;
    }
    let early_safe_room_count = smoke_route_early_safe_room_count(stats);
    *best = Some(MapRouteCandidateProjectionV0 {
        score_kind: "full_map_route_v1".to_string(),
        target_x,
        target_y,
        total_score,
        base_score,
        adjustment_score,
        first_shop_depth: stats.first_shop_depth,
        first_rest_depth: stats.first_rest_depth,
        first_elite_depth: stats.first_elite_depth,
        early_monster_count: stats.early_monster_count,
        early_event_count: stats.early_event_count,
        early_safe_room_count,
        best_path: path,
    });
}

fn record_smoke_route_room(stats: &mut SmokeRouteStats, room_type: RoomType, depth: usize) {
    match room_type {
        RoomType::ShopRoom => {
            stats.first_shop_depth.get_or_insert(depth);
        }
        RoomType::RestRoom => {
            stats.first_rest_depth.get_or_insert(depth);
        }
        RoomType::MonsterRoomElite => {
            stats.first_elite_depth.get_or_insert(depth);
            if depth <= 3 {
                stats.early_monster_count += 2;
            }
        }
        RoomType::MonsterRoom => {
            if depth <= 3 {
                stats.early_monster_count += 1;
            }
        }
        RoomType::EventRoom => {
            if depth <= 3 {
                stats.early_event_count += 1;
            }
        }
        _ => {}
    }
}

fn smoke_route_adjustment(run_state: &RunState, stats: SmokeRouteStats) -> i32 {
    let need = crate::bot::shared::analyze_run_needs(run_state);
    let mut adjustment = 0;
    if let Some(shop_depth) = stats.first_shop_depth {
        if shop_depth <= 5 && run_state.gold >= 150 {
            adjustment += 120;
        } else if shop_depth <= 7 && run_state.gold >= 120 {
            adjustment += 50;
        }
    }

    if let Some(elite_depth) = stats.first_elite_depth {
        let shop_before_elite = stats
            .first_shop_depth
            .is_some_and(|shop_depth| shop_depth < elite_depth);
        let rest_before_elite = stats
            .first_rest_depth
            .is_some_and(|rest_depth| rest_depth < elite_depth);
        let readiness_gap = need.damage_gap + need.block_gap + need.control_gap;
        if elite_depth <= 5 && readiness_gap > 0 && !shop_before_elite && !rest_before_elite {
            adjustment -= 320 + readiness_gap * 3 + need.survival_pressure / 2;
        }
    }

    if run_state.act_num >= 2 {
        let early_safe_rooms = stats.early_event_count
            + stats
                .first_rest_depth
                .filter(|depth| *depth <= 4)
                .map(|_| 1)
                .unwrap_or(0)
            + stats
                .first_shop_depth
                .filter(|depth| *depth <= 4)
                .map(|_| 1)
                .unwrap_or(0);
        adjustment -= stats.early_monster_count as i32 * 80;
        adjustment += early_safe_rooms as i32 * 45;
        if run_state.gold >= 250 && stats.first_shop_depth.is_none() {
            adjustment -= 90;
        }
    }

    adjustment
}

fn smoke_route_early_safe_room_count(stats: SmokeRouteStats) -> usize {
    stats.early_event_count
        + stats
            .first_rest_depth
            .filter(|depth| *depth <= 4)
            .map(|_| 1)
            .unwrap_or(0)
        + stats
            .first_shop_depth
            .filter(|depth| *depth <= 4)
            .map(|_| 1)
            .unwrap_or(0)
}

fn score_map_room(run_state: &RunState, room_type: RoomType) -> i32 {
    let hp_ratio = run_state.current_hp * 100 / run_state.max_hp.max(1);
    match room_type {
        RoomType::MonsterRoomElite if hp_ratio <= 20 => -260,
        RoomType::MonsterRoomElite if hp_ratio >= 70 => 70,
        RoomType::MonsterRoomElite => -20,
        RoomType::RestRoom if hp_ratio < 70 => 90,
        RoomType::RestRoom => 45,
        RoomType::TreasureRoom => 80,
        RoomType::ShopRoom if run_state.gold >= 150 => 75,
        RoomType::ShopRoom => 25,
        RoomType::EventRoom => 55,
        RoomType::MonsterRoom if hp_ratio <= 15 => -120,
        RoomType::MonsterRoom if hp_ratio <= 25 => 10,
        RoomType::MonsterRoom => 50,
        RoomType::MonsterRoomBoss => 200,
        RoomType::TrueVictoryRoom => 300,
    }
}

pub fn score_event_action(run_state: &RunState, action: &ClientInput) -> i32 {
    match action {
        ClientInput::EventChoice(index) => {
            crate::engine::event_handler::get_event_options(run_state)
                .get(*index)
                .map(|option| score_event_option(run_state, option) - *index as i32)
                .unwrap_or(-1_000)
        }
        ClientInput::Proceed => 5,
        _ => score_noncombat_fallback(action),
    }
}

pub fn score_event_action_v0_control(action: &ClientInput) -> i32 {
    match action {
        ClientInput::EventChoice(index) => 30 - *index as i32,
        ClientInput::Proceed => 5,
        _ => score_noncombat_fallback(action),
    }
}

fn score_event_option(run_state: &RunState, option: &crate::state::events::EventOption) -> i32 {
    use crate::state::events::{
        EventActionKind, EventEffect, EventOptionTransition, EventRelicKind,
    };

    if option.ui.disabled {
        return -1_000;
    }

    let mut score = match option.semantics.action {
        EventActionKind::Leave | EventActionKind::Continue => 8,
        EventActionKind::Decline => 12,
        EventActionKind::Fight => {
            if run_state.act_num >= 3 {
                -220
            } else if run_state.current_hp * 100 / run_state.max_hp.max(1) >= 70 {
                35
            } else {
                -120
            }
        }
        EventActionKind::Accept
        | EventActionKind::Gain
        | EventActionKind::Trade
        | EventActionKind::DeckOperation
        | EventActionKind::Special => 25,
        EventActionKind::Unknown => 30,
    };

    match option.semantics.transition {
        EventOptionTransition::StartCombat => {
            score -= if run_state.act_num >= 3 { 220 } else { 60 };
        }
        EventOptionTransition::OpenSelection(_) => score += 45,
        EventOptionTransition::OpenReward => score += 45,
        EventOptionTransition::Complete => score += 5,
        EventOptionTransition::AdvanceScreen | EventOptionTransition::None => {}
    }

    for effect in &option.semantics.effects {
        match effect {
            EventEffect::GainGold(amount) => {
                score += (*amount / 8).min(80);
            }
            EventEffect::LoseGold(amount) => {
                score -= (*amount / 4).min(120);
            }
            EventEffect::LoseHp(amount) => {
                let hp_after = run_state.current_hp - *amount;
                score -= amount * if hp_after <= 0 { 100 } else { 4 };
            }
            EventEffect::Heal(amount) => {
                score += amount * 5;
            }
            EventEffect::GainMaxHp(amount) => score += amount * 8,
            EventEffect::LoseMaxHp(amount) => score -= amount * 10,
            EventEffect::ObtainRelic { count, kind } => {
                let count = (*count).min(3) as i32;
                score += count * 110;
                if matches!(
                    kind,
                    EventRelicKind::Specific(crate::content::relics::RelicId::MarkOfTheBloom)
                ) {
                    score -= 180;
                }
            }
            EventEffect::ObtainPotion { count } => score += (*count as i32).min(3) * 35,
            EventEffect::ObtainCard { count, kind }
            | EventEffect::ObtainColorlessCard { count, kind } => {
                score += event_obtained_card_value(*count, *kind, run_state);
            }
            EventEffect::ObtainCurse { count, .. } => score -= (*count as i32) * 120,
            EventEffect::RemoveCard { count, .. } => score += (*count as i32).min(3) * 85,
            EventEffect::UpgradeCard { count } => {
                let effective = if *count == usize::MAX {
                    run_state
                        .master_deck
                        .iter()
                        .filter(|card| card.upgrades == 0)
                        .count()
                } else {
                    *count
                };
                score += (effective as i32).min(20) * 18;
            }
            EventEffect::TransformCard { count } => score += (*count as i32).min(3) * 35,
            EventEffect::DuplicateCard { count } => score += (*count as i32).min(3) * 35,
            EventEffect::LoseRelic { .. } | EventEffect::LoseStarterRelic { .. } => score -= 120,
            EventEffect::StartCombat => {
                score -= if run_state.act_num >= 3 { 220 } else { 60 };
            }
        }
    }

    score
}

fn event_obtained_card_value(
    count: usize,
    kind: crate::state::events::EventCardKind,
    run_state: &RunState,
) -> i32 {
    use crate::state::events::EventCardKind;

    let count = count as i32;
    match kind {
        EventCardKind::Specific(card_id) => count * event_specific_card_value(card_id, run_state),
        EventCardKind::RandomColorless | EventCardKind::RandomClassCard => count.min(3) * 20,
        EventCardKind::Unknown => count.min(3) * 20,
    }
}

fn event_specific_card_value(card_id: CardId, run_state: &RunState) -> i32 {
    match card_id {
        // Apparition is an event-only survival card whose value is not represented by
        // base block/damage fields. Without this, Ghosts is valued as five generic
        // filler cards while paying the full max-HP cost.
        CardId::Apparition => {
            let low_hp_bonus = if run_state.current_hp * 100 / run_state.max_hp.max(1) <= 45 {
                25
            } else {
                0
            };
            95 + low_hp_bonus
        }
        _ => rule_card_offer_score(card_id, run_state).max(20),
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
                    score += match request.reason {
                        crate::state::selection::SelectionReason::Purge => {
                            70 + rule_remove_score(card.id, run_state)
                        }
                        crate::state::selection::SelectionReason::Transform
                        | crate::state::selection::SelectionReason::TransformUpgraded => {
                            45 + rule_remove_score(card.id, run_state)
                        }
                        crate::state::selection::SelectionReason::Upgrade => {
                            let already_upgraded_penalty = if card.upgrades > 0
                                && card.id != CardId::SearingBlow
                            {
                                90
                            } else {
                                0
                            };
                            55 + rule_upgrade_score(card.id) - already_upgraded_penalty
                        }
                        crate::state::selection::SelectionReason::Duplicate => {
                            55 + rule_card_offer_score(card.id, run_state)
                        }
                        _ => rule_remove_score(card.id, run_state).max(0) / 2,
                    };
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
