use sts_simulator::ai::strategy::decision_pipeline::{
    CandidateOrderKey, DecisionCandidateKind, DecisionPipelineContext,
};
use sts_simulator::ai::strategy::deck_plan::DeckPlanSnapshot;
use sts_simulator::ai::strategy::deck_strategic_deficit::StrategicDeficitLevel;
use sts_simulator::ai::strategy::reward_admission::{
    assess_reward_admission_from_master_deck, RewardAdmission,
};
use sts_simulator::ai::strategy::shop_boss_preview::shop_boss_preview_bundles;
use sts_simulator::ai::strategy::shop_purchase_bundle::ShopGoldOpportunity;
use sts_simulator::content::relics::RelicId;
use sts_simulator::eval::run_control::{DecisionCandidateKey, DecisionSurface, RunControlSession};
use sts_simulator::runtime::combat::CombatCard;

use super::candidate_ir_adapter::shop_tiny_kind;
use super::expansion_policy::shop_tiny_choice_expansion;
use super::owner_candidate_eval::candidate_annotation;
use super::owner_commands::executable_choices;
use super::owner_model::{ChoiceAnnotation, OwnerChoice};
use super::shop_investment::shop_investment_for_surface;
use super::shop_route_evidence::future_shop_distance;

pub(super) fn shop_tiny_owner_choices(
    session: &RunControlSession,
    surface: &DecisionSurface,
) -> Vec<OwnerChoice> {
    let base_context = shop_tiny_context(session);
    let deck = &session.run_state.master_deck;
    let shop_investment = shop_investment_for_surface(session, surface, deck, base_context);
    let context = shop_investment
        .map(|shop| base_context.with_shop_investment(shop))
        .unwrap_or(base_context);
    let mut choices = executable_choices(surface)
        .into_iter()
        .map(|mut choice| {
            choice.annotation = shop_tiny_candidate_for_choice(context, deck, &choice);
            choice
        })
        .enumerate()
        .collect::<Vec<_>>();
    let mut auto_purge_targets = Vec::new();
    for (_, choice) in choices.iter_mut() {
        choice.expansion = shop_tiny_choice_expansion(&choice.annotation, &mut auto_purge_targets);
    }
    let preferred_bundle_next = preferred_shop_boss_bundle_next_kind(session, &choices);
    choices.sort_by_key(|(index, choice)| {
        (
            bundle_execution_rank(&preferred_bundle_next, &choice.key),
            shop_tiny_choice_rank(choice),
            *index,
        )
    });
    choices.into_iter().map(|(_, choice)| choice).collect()
}

fn shop_tiny_context(session: &RunControlSession) -> DecisionPipelineContext {
    let deck_plan = DeckPlanSnapshot::from_run_state(&session.run_state);
    let context = DecisionPipelineContext::shop(deck_plan, session.run_state.gold);
    let active_maw_bank = active_maw_bank_opportunity(session);
    let hard_checkpoint_imminent = hard_checkpoint_imminent(session);
    let future_shop_distance = future_shop_distance(session);
    if active_maw_bank || hard_checkpoint_imminent || future_shop_distance.is_some() {
        context.with_shop_gold_opportunity(ShopGoldOpportunity {
            current_gold: session.run_state.gold,
            current_hp: session.run_state.current_hp,
            max_hp: session.run_state.max_hp,
            active_maw_bank,
            future_rooms_before_next_shop: future_shop_distance.unwrap_or(5),
            hard_checkpoint_imminent,
            survival_purchase_needed: deck_plan.survival_pressure(),
            boss_answer_needed: matches!(
                deck_plan.strategic_deficit.boss_scaling_plan,
                StrategicDeficitLevel::Missing | StrategicDeficitLevel::Thin
            ),
        })
    } else {
        context
    }
}

fn active_maw_bank_opportunity(session: &RunControlSession) -> bool {
    active_maw_bank(session)
        || session
            .shop_visit_context()
            .is_some_and(|context| context.maw_bank_live_at_entry)
}

fn active_maw_bank(session: &RunControlSession) -> bool {
    session
        .run_state
        .relics
        .iter()
        .any(|relic| relic.id == RelicId::MawBank && !relic.used_up)
}

fn hard_checkpoint_imminent(session: &RunControlSession) -> bool {
    session.run_state.act_num == 1 && session.run_state.floor_num >= 13
}

fn shop_tiny_candidate_for_choice(
    context: DecisionPipelineContext,
    deck: &[CombatCard],
    choice: &OwnerChoice,
) -> ChoiceAnnotation {
    let kind = shop_tiny_kind(&choice.key);
    candidate_annotation(context, kind, shop_card_admission(deck, kind))
}

fn shop_card_admission(
    deck: &[CombatCard],
    kind: DecisionCandidateKind,
) -> Option<RewardAdmission> {
    if let DecisionCandidateKind::ShopBuyCard { card, upgrades, .. } = kind {
        Some(assess_reward_admission_from_master_deck(
            deck, card, upgrades,
        ))
    } else {
        None
    }
}

fn shop_tiny_choice_rank(choice: &OwnerChoice) -> (u8, CandidateOrderKey) {
    match &choice.annotation {
        ChoiceAnnotation::Candidate(decision) => decision.evaluation.auto_order_key(false),
        _ => (u8::MAX, CandidateOrderKey::fallback()),
    }
}

fn preferred_shop_boss_bundle_next_kind(
    session: &RunControlSession,
    choices: &[(usize, OwnerChoice)],
) -> Option<DecisionCandidateKind> {
    if !hard_checkpoint_imminent(session) {
        return None;
    }
    let executable_kinds = choices
        .iter()
        .filter_map(|(_, choice)| {
            if choice.auto_expand_allowed() {
                Some(shop_tiny_kind(&choice.key))
            } else {
                None
            }
        })
        .collect::<Vec<_>>();
    let bundle = shop_boss_preview_bundles(executable_kinds, session.run_state.gold, 2)
        .into_iter()
        .find(|bundle| !bundle.items.is_empty())?;
    bundle.items.into_iter().find(|kind| {
        choices
            .iter()
            .any(|(_, choice)| choice.auto_expand_allowed() && shop_tiny_kind(&choice.key) == *kind)
    })
}

fn bundle_execution_rank(
    preferred_bundle_next: &Option<DecisionCandidateKind>,
    key: &Option<DecisionCandidateKey>,
) -> u8 {
    match preferred_bundle_next {
        Some(kind) if shop_tiny_kind(key) == *kind => 0,
        Some(_) => 1,
        None => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sts_simulator::content::cards::CardId;
    use sts_simulator::content::potions::PotionId;
    use sts_simulator::content::relics::RelicState;
    use sts_simulator::eval::run_control::{
        build_decision_surface, DecisionCandidateKey, RunControlCommand, RunControlConfig,
        ShopVisitContextV1,
    };
    use sts_simulator::runtime::combat::CombatCard;
    use sts_simulator::state::core::{ClientInput, EngineState};
    use sts_simulator::state::map::node::{MapEdge, MapRoomNode, RoomType};
    use sts_simulator::state::map::state::MapState;
    use sts_simulator::state::shop::{ShopCard, ShopPotion, ShopRelic, ShopState};

    fn map_node(x: i32, y: i32, class: RoomType) -> MapRoomNode {
        let mut node = MapRoomNode::new(x, y);
        node.class = Some(class);
        node
    }

    fn maw_bank_session() -> RunControlSession {
        let mut session = RunControlSession::new(RunControlConfig::default());
        session.run_state.gold = 224;
        session.run_state.relics = vec![RelicState::new(RelicId::MawBank)];
        session.run_state.master_deck = vec![
            CombatCard::new(CardId::Strike, 1),
            CombatCard::new(CardId::Strike, 2),
            CombatCard::new(CardId::Defend, 3),
            CombatCard::new(CardId::Defend, 4),
            CombatCard::new(CardId::Bash, 5),
            CombatCard::new(CardId::Immolate, 6),
            CombatCard::new(CardId::IronWave, 7),
            CombatCard::new(CardId::Cleave, 8),
            CombatCard::new(CardId::ShrugItOff, 9),
            CombatCard::new(CardId::PommelStrike, 10),
            CombatCard::new(CardId::Bloodletting, 11),
        ];
        session
    }

    fn choice(key: DecisionCandidateKey) -> OwnerChoice {
        OwnerChoice {
            key: Some(key),
            action: RunControlCommand::Noop,
            label: String::new(),
            annotation: ChoiceAnnotation::None,
            expansion: super::super::owner_model::OwnerChoiceExpansion::AutoAllowed,
        }
    }

    #[test]
    fn shop_tiny_context_preserves_actual_future_shop_distance() {
        let mut current = map_node(0, 0, RoomType::ShopRoom);
        current.edges.insert(MapEdge::new(0, 0, 0, 1));
        let mut first = map_node(0, 1, RoomType::MonsterRoom);
        first.edges.insert(MapEdge::new(0, 1, 0, 2));
        let mut second = map_node(0, 2, RoomType::EventRoom);
        second.edges.insert(MapEdge::new(0, 2, 0, 3));
        let mut third = map_node(0, 3, RoomType::RestRoom);
        third.edges.insert(MapEdge::new(0, 3, 0, 4));
        let future_shop = map_node(0, 4, RoomType::ShopRoom);

        let mut session = RunControlSession::new(RunControlConfig::default());
        session.run_state.map = MapState::new(vec![
            vec![current],
            vec![first],
            vec![second],
            vec![third],
            vec![future_shop],
        ]);
        session.run_state.map.current_x = 0;
        session.run_state.map.current_y = 0;

        let context = shop_tiny_context(&session);

        assert_eq!(
            context
                .shop_gold_opportunity
                .expect("reachable future shop should create route evidence")
                .future_rooms_before_next_shop,
            4,
            "a distant reachable shop must not be compressed into the two-room liquidity window"
        );
    }

    #[test]
    fn shop_tiny_owner_context_prefers_leave_over_generic_maw_bank_breaking_relic() {
        let session = maw_bank_session();
        let context = shop_tiny_context(&session);
        let mut clockwork = choice(DecisionCandidateKey::ShopBuyRelic {
            shop_slot: 0,
            relic: RelicId::ClockworkSouvenir,
            price: 149,
        });
        let mut leave = choice(DecisionCandidateKey::ShopLeave);

        clockwork.annotation =
            shop_tiny_candidate_for_choice(context, &session.run_state.master_deck, &clockwork);
        leave.annotation =
            shop_tiny_candidate_for_choice(context, &session.run_state.master_deck, &leave);
        let mut auto_purge_targets = Vec::new();
        clockwork.expansion =
            shop_tiny_choice_expansion(&clockwork.annotation, &mut auto_purge_targets);
        leave.expansion = shop_tiny_choice_expansion(&leave.annotation, &mut auto_purge_targets);

        assert_eq!(
            clockwork.inspect_only_reason(),
            Some("BreaksMawBankWithoutHardNeed")
        );
        assert!(
            shop_tiny_choice_rank(&leave) < shop_tiny_choice_rank(&clockwork),
            "ShopTiny should prefer LeaveWithGold over generic Maw Bank-breaking relic"
        );
    }

    #[test]
    fn shop_tiny_allows_hard_checkpoint_starter_strike_cleanup_through_maw_bank() {
        let mut session = maw_bank_session();
        session.run_state.floor_num = 13;
        session.run_state.gold = 249;
        let context = shop_tiny_context(&session);
        let mut remove_strike = choice(DecisionCandidateKey::ShopPurgeCard {
            deck_index: 0,
            card: CardId::Strike,
            upgrades: 0,
        });
        let mut leave = choice(DecisionCandidateKey::ShopLeave);

        remove_strike.annotation =
            shop_tiny_candidate_for_choice(context, &session.run_state.master_deck, &remove_strike);
        leave.annotation =
            shop_tiny_candidate_for_choice(context, &session.run_state.master_deck, &leave);
        let mut auto_purge_targets = Vec::new();
        remove_strike.expansion =
            shop_tiny_choice_expansion(&remove_strike.annotation, &mut auto_purge_targets);
        leave.expansion = shop_tiny_choice_expansion(&leave.annotation, &mut auto_purge_targets);

        assert_eq!(remove_strike.inspect_only_reason(), None);
        assert!(
            shop_tiny_choice_rank(&remove_strike) < shop_tiny_choice_rank(&leave),
            "hard checkpoint cleanup should be eligible ahead of preserving Maw Bank"
        );
    }

    #[test]
    fn shop_tiny_keeps_entry_maw_bank_cost_after_same_shop_spend() {
        let mut session = maw_bank_session();
        session.run_state.gold = 224;
        let mut shop = ShopState::new();
        shop.purge_cost = 75;
        shop.relics.push(ShopRelic {
            relic_id: RelicId::ClockworkSouvenir,
            price: 149,
            can_buy: true,
            blocked_reason: None,
        });
        session.engine_state = EngineState::Shop(shop);

        session
            .apply_command(RunControlCommand::Input(ClientInput::PurgeCard(0)))
            .expect("shop purge should apply");
        assert_eq!(session.run_state.gold, 149);
        assert!(
            session
                .run_state
                .relics
                .iter()
                .any(|relic| relic.id == RelicId::MawBank && relic.used_up),
            "spending in shop should consume Maw Bank"
        );

        let context = shop_tiny_context(&session);
        let mut clockwork = choice(DecisionCandidateKey::ShopBuyRelic {
            shop_slot: 0,
            relic: RelicId::ClockworkSouvenir,
            price: 149,
        });
        clockwork.annotation =
            shop_tiny_candidate_for_choice(context, &session.run_state.master_deck, &clockwork);
        let mut auto_purge_targets = Vec::new();
        clockwork.expansion =
            shop_tiny_choice_expansion(&clockwork.annotation, &mut auto_purge_targets);

        assert_eq!(
            clockwork.inspect_only_reason(),
            Some("BreaksMawBankWithoutHardNeed"),
            "same-shop follow-up purchases must still carry entry Maw Bank opportunity cost"
        );
    }

    #[test]
    fn shop_tiny_prefers_low_hp_waffle_before_cleanup() {
        let mut session = maw_bank_session();
        session.run_state.current_hp = 41;
        session.run_state.max_hp = 85;
        session.run_state.gold = 335;
        let mut shop = ShopState::new();
        shop.purge_cost = 75;
        shop.relics.push(ShopRelic {
            relic_id: RelicId::Waffle,
            price: 155,
            can_buy: true,
            blocked_reason: None,
        });
        shop.potions.push(ShopPotion {
            potion_id: PotionId::FearPotion,
            price: 50,
            can_buy: true,
            blocked_reason: None,
        });
        session.engine_state = EngineState::Shop(shop);

        let surface = build_decision_surface(&session);
        let choices = shop_tiny_owner_choices(&session, &surface);

        assert!(
            matches!(
                choices.first().and_then(|choice| choice.key.as_ref()),
                Some(DecisionCandidateKey::ShopBuyRelic {
                    relic: RelicId::Waffle,
                    price: 155,
                    ..
                })
            ),
            "low HP Waffle should be treated as survival repair before cleanup/potions; got {:?}",
            choices.first().map(|choice| choice.label.as_str())
        );
    }

    #[test]
    fn shop_tiny_ignores_stale_maw_bank_visit_context_on_later_shop_floor() {
        let mut session = maw_bank_session();
        session.run_state.floor_num = 13;
        session.run_state.gold = 194;
        session.run_state.relics.clear();
        session.shop_visit_context = Some(ShopVisitContextV1 {
            entry_act: 1,
            entry_floor: 11,
            entry_gold: 226,
            maw_bank_live_at_entry: true,
            spent_gold_in_visit: true,
        });

        let context = shop_tiny_context(&session);
        let mut clockwork = choice(DecisionCandidateKey::ShopBuyRelic {
            shop_slot: 0,
            relic: RelicId::ClockworkSouvenir,
            price: 149,
        });
        clockwork.annotation =
            shop_tiny_candidate_for_choice(context, &session.run_state.master_deck, &clockwork);
        let mut auto_purge_targets = Vec::new();
        clockwork.expansion =
            shop_tiny_choice_expansion(&clockwork.annotation, &mut auto_purge_targets);

        assert_ne!(
            clockwork.inspect_only_reason(),
            Some("BreaksMawBankWithoutHardNeed"),
            "Maw Bank entry cost must not leak into a later shop floor"
        );
    }

    #[test]
    fn hard_checkpoint_bundle_next_item_preempts_random_potion_spend() {
        let mut session = maw_bank_session();
        session.run_state.act_num = 1;
        session.run_state.floor_num = 13;
        session.run_state.gold = 162;
        session.run_state.relics.clear();
        session.run_state.potions = vec![None, None, None];
        let mut shop = ShopState::new();
        shop.cards.push(ShopCard {
            card_id: CardId::FiendFire,
            upgrades: 0,
            price: 152,
            can_buy: true,
            blocked_reason: None,
        });
        shop.cards.push(ShopCard {
            card_id: CardId::Bludgeon,
            upgrades: 0,
            price: 162,
            can_buy: true,
            blocked_reason: None,
        });
        shop.potions.push(ShopPotion {
            potion_id: PotionId::GamblersBrew,
            price: 73,
            can_buy: true,
            blocked_reason: None,
        });
        session.engine_state = EngineState::Shop(shop);

        let surface = build_decision_surface(&session);
        let choices = shop_tiny_owner_choices(&session, &surface);

        assert!(
            matches!(
                choices.first().and_then(|choice| choice.key.as_ref()),
                Some(DecisionCandidateKey::ShopBuyCard {
                    card: CardId::FiendFire,
                    price: 152,
                    ..
                })
            ),
            "hard checkpoint bundle execution should buy Fiend Fire before spending on random/deferred potions; got {:?}",
            choices.first().map(|choice| choice.label.as_str())
        );
    }
}
