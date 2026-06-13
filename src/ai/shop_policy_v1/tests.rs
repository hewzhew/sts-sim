use crate::ai::decision_tags_v1::{
    TAG_COMBAT_SHAPE_ADDS_SELF_COPY, TAG_COMBAT_SHAPE_ADDS_STATUS, TAG_COMBAT_SHAPE_RANDOM_EXHAUST,
    TAG_COMBAT_SHAPE_TOPDECK_SENSITIVE, TAG_DIGEST_CAPACITY_DRAW, TAG_DIGEST_CAPACITY_EXHAUST,
    TAG_DIGEST_CAPACITY_STATUS, TAG_DIGEST_CAPACITY_TOPDECK,
};
use crate::ai::shop_policy_v1::{
    build_shop_decision_context_v1, compile_shop_decision_v1, plan_shop_decision_v1,
    shop_card_conversion_priority_v1, ShopCompileModeV1, ShopDecisionSourceV1,
    ShopPlanKindV1, ShopPlanStepV1, ShopPolicyActionV1, ShopPolicyClassV1,
    ShopPolicyConfigV1, ShopPurchaseTargetV1,
};
use crate::ai::strategic::{CandidateAction, PressureKind, StrategicJob};
use crate::content::cards::CardId;
use crate::content::monsters::factory::EncounterId;
use crate::content::potions::PotionId;
use crate::content::relics::RelicId;
use crate::state::map::{MapEdge, MapRoomNode, MapState, RoomType};
use crate::state::run::RunState;
use crate::state::shop::{ShopCard, ShopPotion, ShopRelic, ShopState};

#[test]
fn shop_context_exposes_visible_curse_purge_candidate() {
    let mut run_state = RunState::new(1, 0, false, "Ironclad");
    run_state.gold = 100;
    run_state.add_card_to_deck_without_interception_from(
        CardId::Doubt,
        0,
        crate::state::selection::DomainEventSource::DeckMutation,
    );
    let shop = ShopState::new();

    let context = build_shop_decision_context_v1(&run_state, &shop);

    assert!(context
        .candidates
        .iter()
        .any(|candidate| candidate.class == ShopPolicyClassV1::CursePurge));
}

#[test]
fn shop_context_exposes_purchase_candidates_without_selecting_policy() {
    let mut run_state = RunState::new(1, 0, false, "Ironclad");
    run_state.gold = 100;
    run_state.add_card_to_deck(CardId::Inflame);
    run_state.add_card_to_deck(CardId::HeavyBlade);
    let mut shop = ShopState::new();
    shop.cards.push(ShopCard {
        card_id: CardId::PommelStrike,
        upgrades: 0,
        price: 50,
        can_buy: true,
        blocked_reason: None,
    });

    let context = build_shop_decision_context_v1(&run_state, &shop);

    assert!(context
        .candidates
        .iter()
        .any(|candidate| { candidate.label.contains("Pommel") }));
}

#[test]
fn shop_context_exposes_neutral_combat_shape_and_digest_evidence() {
    let mut run_state = RunState::new(1, 0, false, "Ironclad");
    run_state.gold = 500;
    run_state.add_card_to_deck(CardId::Evolve);
    run_state.add_card_to_deck(CardId::Corruption);
    run_state.add_card_to_deck(CardId::BattleTrance);
    run_state.add_card_to_deck(CardId::Headbutt);
    let mut shop = ShopState::new();
    shop.cards.push(ShopCard {
        card_id: CardId::RecklessCharge,
        upgrades: 0,
        price: 50,
        can_buy: true,
        blocked_reason: None,
    });
    shop.cards.push(ShopCard {
        card_id: CardId::Anger,
        upgrades: 0,
        price: 50,
        can_buy: true,
        blocked_reason: None,
    });
    shop.cards.push(ShopCard {
        card_id: CardId::Havoc,
        upgrades: 0,
        price: 50,
        can_buy: true,
        blocked_reason: None,
    });

    let context = build_shop_decision_context_v1(&run_state, &shop);
    let reckless = shop_card_candidate(&context, CardId::RecklessCharge);
    let anger = shop_card_candidate(&context, CardId::Anger);
    let havoc = shop_card_candidate(&context, CardId::Havoc);

    assert_has_evidence(reckless, TAG_COMBAT_SHAPE_ADDS_STATUS);
    assert_has_evidence(reckless, TAG_DIGEST_CAPACITY_STATUS);
    assert_has_evidence(reckless, TAG_DIGEST_CAPACITY_EXHAUST);
    assert_has_evidence(reckless, TAG_DIGEST_CAPACITY_DRAW);
    assert_has_evidence(reckless, TAG_DIGEST_CAPACITY_TOPDECK);
    assert_has_evidence(anger, TAG_COMBAT_SHAPE_ADDS_SELF_COPY);
    assert_has_evidence(havoc, TAG_COMBAT_SHAPE_RANDOM_EXHAUST);
    assert_has_evidence(havoc, TAG_COMBAT_SHAPE_TOPDECK_SENSITIVE);
    assert!(
        reckless
            .risks
            .iter()
            .all(|risk| !risk.contains("combat_shape")),
        "combat shape is neutral evidence, not a risk/approval shortcut"
    );
}

#[test]
fn shop_strategic_trace_maps_buy_cards_by_semantic_jobs() {
    let mut run_state = RunState::new(1, 0, false, "Ironclad");
    run_state.gold = 500;
    let mut shop = ShopState::new();
    shop.cards.push(ShopCard {
        card_id: CardId::ShrugItOff,
        upgrades: 0,
        price: 50,
        can_buy: true,
        blocked_reason: None,
    });
    shop.cards.push(ShopCard {
        card_id: CardId::BurningPact,
        upgrades: 0,
        price: 50,
        can_buy: true,
        blocked_reason: None,
    });

    let context = build_shop_decision_context_v1(&run_state, &shop);
    let trace = crate::ai::strategic::strategic_trace_for_shop(&context);
    let shrug = buy_card_delta(&trace, CardId::ShrugItOff);
    let burning_pact = buy_card_delta(&trace, CardId::BurningPact);

    assert_delta_has_job(shrug, StrategicJob::Block);
    assert_delta_has_job(shrug, StrategicJob::DrawEnergy);
    assert_delta_lacks_job(shrug, StrategicJob::Frontload);
    assert_delta_has_job(burning_pact, StrategicJob::DrawEnergy);
    assert_delta_has_job(burning_pact, StrategicJob::ExhaustAccess);
}

#[test]
fn shop_policy_converts_high_gold_into_affordable_relic_even_below_old_high_impact_threshold() {
    let mut run_state = RunState::new(1, 0, false, "Ironclad");
    run_state.act_num = 3;
    run_state.floor_num = 46;
    run_state.gold = 430;
    let mut shop = ShopState::new();
    shop.relics.push(ShopRelic {
        relic_id: RelicId::Anchor,
        price: 146,
        can_buy: true,
        blocked_reason: None,
    });

    let context = build_shop_decision_context_v1(&run_state, &shop);
    let decision = plan_shop_decision_v1(&context, &ShopPolicyConfigV1::default());

    assert!(context.conversion_pressure);
    assert!(matches!(
        decision.action,
        ShopPolicyActionV1::Purchase {
            target: ShopPurchaseTargetV1::Relic {
                relic: RelicId::Anchor,
                ..
            },
            ..
        }
    ));
}

#[test]
fn compiled_shop_decision_wraps_selected_relic_purchase_as_plan() {
    let mut run_state = RunState::new(1, 0, false, "Ironclad");
    run_state.act_num = 3;
    run_state.floor_num = 46;
    run_state.gold = 430;
    let mut shop = ShopState::new();
    shop.relics.push(ShopRelic {
        relic_id: RelicId::Anchor,
        price: 146,
        can_buy: true,
        blocked_reason: None,
    });

    let context = build_shop_decision_context_v1(&run_state, &shop);
    let compiled = compile_shop_decision_v1(
        &context,
        &ShopPolicyConfigV1::default(),
        ShopCompileModeV1::ExecuteOne,
    );

    assert_eq!(compiled.source, ShopDecisionSourceV1::LegacyWrapped);
    assert_eq!(compiled.selected_plan.kind, ShopPlanKindV1::Execute);
    assert_eq!(compiled.selected_plan.total_gold_spent, 146);
    let relic_candidate_priority = context
        .candidates
        .iter()
        .find(|candidate| {
            candidate.purchase_target
                == Some(ShopPurchaseTargetV1::Relic {
                    index: 0,
                    relic: RelicId::Anchor,
                })
        })
        .and_then(|candidate| candidate.purchase_priority);
    assert_eq!(compiled.selected_plan.legacy_priority, relic_candidate_priority);
    assert_eq!(
        compiled.selected_plan.steps,
        vec![ShopPlanStepV1::BuyRelic {
            index: 0,
            relic: RelicId::Anchor,
            cost: 146,
        }]
    );
}

#[test]
fn compiled_shop_decision_wraps_curse_purge_as_plan() {
    let mut run_state = RunState::new(1, 0, false, "Ironclad");
    run_state.gold = 100;
    run_state.add_card_to_deck_without_interception_from(
        CardId::Doubt,
        0,
        crate::state::selection::DomainEventSource::DeckMutation,
    );
    let shop = ShopState::new();

    let context = build_shop_decision_context_v1(&run_state, &shop);
    let compiled = compile_shop_decision_v1(
        &context,
        &ShopPolicyConfigV1::default(),
        ShopCompileModeV1::ExecuteOne,
    );

    assert_eq!(compiled.selected_plan.kind, ShopPlanKindV1::Execute);
    assert_eq!(
        compiled.selected_plan.steps,
        vec![ShopPlanStepV1::RemoveCard {
            deck_index: 10,
            card: CardId::Doubt,
            cost: 75,
        }]
    );
}

#[test]
fn compiled_shop_branch_topk_returns_plan_alternatives() {
    let mut run_state = RunState::new(1, 0, false, "Ironclad");
    run_state.gold = 500;
    let mut shop = ShopState::new();
    shop.cards.push(ShopCard {
        card_id: CardId::Shockwave,
        upgrades: 0,
        price: 89,
        can_buy: true,
        blocked_reason: None,
    });
    shop.relics.push(ShopRelic {
        relic_id: RelicId::Anchor,
        price: 146,
        can_buy: true,
        blocked_reason: None,
    });
    shop.potions.push(ShopPotion {
        potion_id: PotionId::FirePotion,
        price: 50,
        can_buy: true,
        blocked_reason: None,
    });

    let context = build_shop_decision_context_v1(&run_state, &shop);
    let compiled = compile_shop_decision_v1(
        &context,
        &ShopPolicyConfigV1::default(),
        ShopCompileModeV1::BranchTopK { max_plans: 3 },
    );

    assert!(!compiled.alternatives.is_empty());
    assert!(compiled.alternatives.len() <= 3);
    assert!(compiled.alternatives.iter().all(|plan| {
        !plan.steps.is_empty() || matches!(plan.kind, ShopPlanKindV1::Stop)
    }));
    assert!(compiled
        .alternatives
        .iter()
        .any(|plan| plan.candidate_ids.iter().any(|id| id.starts_with("shop:"))));
}

#[test]
fn shop_card_priority_penalizes_transition_cards_when_deck_is_bloated() {
    let mut bloated = RunState::new(1, 0, false, "Ironclad");
    bloated.act_num = 3;
    bloated.floor_num = 46;
    add_deck_bloat(&mut bloated, 34);

    assert!(
        shop_card_conversion_priority_v1(CardId::PommelStrike, &bloated) <= 0,
        "large decks should not keep treating a normal transition card as shop conversion"
    );
}

#[test]
fn shop_card_priority_preserves_boss_patch_cards_when_deck_is_bloated() {
    let mut run_state = RunState::new(1, 0, false, "Ironclad");
    run_state.act_num = 3;
    run_state.floor_num = 46;
    add_deck_bloat(&mut run_state, 34);

    assert!(
        shop_card_conversion_priority_v1(CardId::Shockwave, &run_state)
            >= ShopPolicyConfigV1::default().high_impact_card_purchase_priority_threshold,
        "deck bloat pressure should not block a clear boss/elite answer"
    );
}

#[test]
fn shop_card_priority_penalizes_more_fnp_without_exhaust_engine() {
    let mut run_state = RunState::new(1, 0, false, "Ironclad");
    run_state.act_num = 2;
    run_state.add_card_to_deck(CardId::FeelNoPain);

    assert!(
        shop_card_conversion_priority_v1(CardId::FeelNoPain, &run_state) <= 0,
        "FNP should stop being a high-impact shop buy when the deck cannot exhaust cards"
    );
}

#[test]
fn shop_policy_does_not_convert_gold_into_transition_card_when_deck_is_bloated() {
    let mut run_state = RunState::new(1, 0, false, "Ironclad");
    run_state.act_num = 3;
    run_state.floor_num = 46;
    run_state.gold = 430;
    add_deck_bloat(&mut run_state, 34);
    let mut shop = ShopState::new();
    shop.cards.push(ShopCard {
        card_id: CardId::PommelStrike,
        upgrades: 0,
        price: 50,
        can_buy: true,
        blocked_reason: None,
    });

    let context = build_shop_decision_context_v1(&run_state, &shop);
    let decision = plan_shop_decision_v1(&context, &ShopPolicyConfigV1::default());

    assert!(context.conversion_pressure);
    assert!(
        !matches!(
            decision.action,
            ShopPolicyActionV1::Purchase {
                target: ShopPurchaseTargetV1::Card {
                    card: CardId::PommelStrike,
                    ..
                },
                ..
            }
        ),
        "conversion pressure should not force ordinary card bloat when the deck is already oversized"
    );
}

#[test]
fn shop_policy_buys_elite_potion_when_first_elite_prep_window_is_open() {
    let mut run_state = RunState::new(1, 0, false, "Ironclad");
    run_state.act_num = 1;
    run_state.floor_num = 4;
    run_state.gold = 100;
    install_current_room_route(
        &mut run_state,
        RoomType::ShopRoom,
        &[RoomType::MonsterRoom, RoomType::MonsterRoomElite],
    );
    let mut shop = ShopState::new();
    shop.potions.push(ShopPotion {
        potion_id: PotionId::FirePotion,
        price: 50,
        can_buy: true,
        blocked_reason: None,
    });

    let context = build_shop_decision_context_v1(&run_state, &shop);
    let decision = plan_shop_decision_v1(&context, &ShopPolicyConfigV1::default());

    assert!(matches!(
        decision.action,
        ShopPolicyActionV1::Purchase {
            target: ShopPurchaseTargetV1::Potion {
                potion: PotionId::FirePotion,
                ..
            },
            ..
        }
    ));
}

#[test]
fn shop_policy_uses_champ_pressure_for_transition_burst_purchase() {
    let mut run_state = RunState::new(1, 0, false, "Ironclad");
    run_state.act_num = 2;
    run_state.floor_num = 18;
    run_state.boss_key = Some(EncounterId::TheChamp);
    run_state.gold = 125;
    let mut shop = ShopState::new();
    shop.cards.push(ShopCard {
        card_id: CardId::Carnage,
        upgrades: 0,
        price: 36,
        can_buy: true,
        blocked_reason: None,
    });
    shop.cards.push(ShopCard {
        card_id: CardId::DeepBreath,
        upgrades: 0,
        price: 96,
        can_buy: true,
        blocked_reason: None,
    });

    let context = build_shop_decision_context_v1(&run_state, &shop);
    let decision = plan_shop_decision_v1(&context, &ShopPolicyConfigV1::default());

    assert!(
        shop_card_conversion_priority_v1(CardId::Carnage, &run_state)
            >= ShopPolicyConfigV1::default().high_impact_card_purchase_priority_threshold
    );
    assert!(matches!(
        decision.action,
        ShopPolicyActionV1::Purchase {
            target: ShopPurchaseTargetV1::Card {
                card: CardId::Carnage,
                ..
            },
            ..
        }
    ));
}

#[test]
fn shop_policy_treats_flex_as_champ_burst_piece_when_payoff_exists() {
    let mut run_state = RunState::new(1, 0, false, "Ironclad");
    run_state.act_num = 2;
    run_state.floor_num = 18;
    run_state.boss_key = Some(EncounterId::TheChamp);
    run_state.gold = 125;
    run_state.add_card_to_deck(CardId::HeavyBlade);

    assert!(
        shop_card_conversion_priority_v1(CardId::Flex, &run_state)
            > shop_card_conversion_priority_v1(CardId::DeepBreath, &run_state)
    );
}

fn add_deck_bloat(run_state: &mut RunState, count: usize) {
    for _ in 0..count {
        run_state.add_card_to_deck(CardId::Strike);
    }
}

fn install_current_room_route(
    run_state: &mut RunState,
    current_room: RoomType,
    future_rooms: &[RoomType],
) {
    let mut graph = Vec::new();
    let mut current = map_node(0, 0, current_room);
    if !future_rooms.is_empty() {
        current.edges.insert(MapEdge::new(0, 0, 0, 1));
    }
    graph.push(vec![current]);
    for (idx, room) in future_rooms.iter().enumerate() {
        let y = idx as i32 + 1;
        let mut node = map_node(0, y, *room);
        if idx + 1 < future_rooms.len() {
            node.edges.insert(MapEdge::new(0, y, 0, y + 1));
        }
        graph.push(vec![node]);
    }
    run_state.map = MapState::new(graph);
    run_state.map.current_x = 0;
    run_state.map.current_y = 0;
}

fn map_node(x: i32, y: i32, room_type: RoomType) -> MapRoomNode {
    let mut node = MapRoomNode::new(x, y);
    node.class = Some(room_type);
    node
}

fn shop_card_candidate(
    context: &crate::ai::shop_policy_v1::ShopDecisionContextV1,
    card: CardId,
) -> &crate::ai::shop_policy_v1::ShopCandidateEvidenceV1 {
    context
        .candidates
        .iter()
        .find(|candidate| candidate.card == Some(card))
        .expect("shop card candidate should exist")
}

fn assert_has_evidence(candidate: &crate::ai::shop_policy_v1::ShopCandidateEvidenceV1, tag: &str) {
    assert!(
        candidate.evidence.iter().any(|item| item == tag),
        "{} should include evidence tag {tag}",
        candidate.label
    );
}

fn buy_card_delta(
    trace: &crate::ai::strategic::StrategicDecisionTrace,
    card: CardId,
) -> &crate::ai::strategic::CandidateDelta {
    trace
        .candidate_deltas
        .iter()
        .find(|delta| {
            matches!(
                delta.action,
                CandidateAction::BuyCard {
                    card: candidate,
                    ..
                } if candidate == card
            )
        })
        .expect("buy-card delta should exist")
}

fn assert_delta_has_job(delta: &crate::ai::strategic::CandidateDelta, job: StrategicJob) {
    assert!(
        delta
            .positive
            .iter()
            .any(|entry| entry.kind == PressureKind::MissingJob(job)),
        "delta should include positive job {job:?}, got {:?}",
        delta.positive
    );
}

fn assert_delta_lacks_job(delta: &crate::ai::strategic::CandidateDelta, job: StrategicJob) {
    assert!(
        delta
            .positive
            .iter()
            .all(|entry| entry.kind != PressureKind::MissingJob(job)),
        "delta should not include positive job {job:?}, got {:?}",
        delta.positive
    );
}
