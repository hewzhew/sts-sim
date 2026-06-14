use crate::ai::decision_tags_v1::{
    TAG_COMBAT_SHAPE_ADDS_SELF_COPY, TAG_COMBAT_SHAPE_ADDS_STATUS, TAG_COMBAT_SHAPE_RANDOM_EXHAUST,
    TAG_COMBAT_SHAPE_TOPDECK_SENSITIVE, TAG_DIGEST_CAPACITY_DRAW, TAG_DIGEST_CAPACITY_EXHAUST,
    TAG_DIGEST_CAPACITY_STATUS, TAG_DIGEST_CAPACITY_TOPDECK,
};
use crate::ai::shop_policy_v1::{
    build_shop_decision_context_v1, compile_shop_decision_v1, plan_shop_decision_v1,
    shop_card_conversion_priority_v1, ShopCompileModeV1, ShopDecisionSourceV1,
    ShopPlanComponentKindV1, ShopPlanKindV1, ShopPlanStepV1, ShopPlanVerdictV1, ShopPolicyActionV1,
    ShopPolicyClassV1, ShopPolicyConfigV1, ShopPurchaseTargetV1,
};
use crate::ai::strategic::{CandidateAction, PressureKind, StrategicBossTax, StrategicJob};
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
fn shop_purge_candidates_are_sourced_from_deck_mutation_compiler() {
    let mut run_state = RunState::new(1, 0, false, "Ironclad");
    run_state.gold = 100;
    run_state.add_card_to_deck_without_interception_from(
        CardId::Doubt,
        0,
        crate::state::selection::DomainEventSource::DeckMutation,
    );
    let shop = ShopState::new();

    let context = build_shop_decision_context_v1(&run_state, &shop);
    let purge = context
        .candidates
        .iter()
        .find(|candidate| candidate.class == ShopPolicyClassV1::CursePurge)
        .expect("expected visible curse purge candidate");

    assert!(
        purge
            .evidence
            .iter()
            .any(|item| item.contains("DeckMutationCompilerV1")),
        "shop purge targets must come from the deck mutation compiler boundary"
    );
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
fn shop_strategic_trace_carries_champ_execute_pressure() {
    let mut run_state = RunState::new(1, 0, false, "Ironclad");
    run_state.act_num = 2;
    run_state.floor_num = 18;
    run_state.boss_key = Some(EncounterId::TheChamp);
    run_state.gold = 500;
    let mut shop = ShopState::new();
    shop.cards.push(ShopCard {
        card_id: CardId::FlameBarrier,
        upgrades: 0,
        price: 73,
        can_buy: true,
        blocked_reason: None,
    });

    let context = build_shop_decision_context_v1(&run_state, &shop);
    let trace = crate::ai::strategic::strategic_trace_for_shop(&context);

    assert!(
        trace
            .ledger
            .items
            .iter()
            .any(|item| { item.kind == PressureKind::BossTax(StrategicBossTax::ChampExecutePlan) }),
        "shop strategic trace should carry The Champ execute pressure, got {:?}",
        trace.ledger.items
    );
}

#[test]
fn shop_strategic_delta_maps_champ_execute_answer_through_component_report() {
    let mut run_state = RunState::new(1, 0, false, "Ironclad");
    run_state.act_num = 2;
    run_state.floor_num = 18;
    run_state.boss_key = Some(EncounterId::TheChamp);
    run_state.gold = 500;
    let mut shop = ShopState::new();
    shop.cards.push(ShopCard {
        card_id: CardId::FlameBarrier,
        upgrades: 0,
        price: 73,
        can_buy: true,
        blocked_reason: None,
    });

    let context = build_shop_decision_context_v1(&run_state, &shop);
    let trace = crate::ai::strategic::strategic_trace_for_shop(&context);
    let flame_barrier = buy_card_delta(&trace, CardId::FlameBarrier);

    assert_delta_has_boss_tax(flame_barrier, StrategicBossTax::ChampExecutePlan);
    assert!(
        flame_barrier
            .evidence
            .iter()
            .any(|evidence| evidence == "card_component_marginal_value contributor"),
        "shop card deltas should reuse the component report boundary, got {:?}",
        flame_barrier.evidence
    );
}

#[test]
fn compiled_shop_card_purchase_can_be_approved_by_strategic_verdict() {
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

    let context = build_shop_decision_context_v1(&run_state, &shop);
    assert!(
        !context.conversion_pressure,
        "test requires no legacy conversion-pressure approval path"
    );
    let carnage = shop_card_candidate(&context, CardId::Carnage);
    assert_eq!(carnage.purchase_priority, Some(250));
    assert!(
        carnage.purchase_priority.unwrap()
            < ShopPolicyConfigV1::default().high_impact_card_purchase_priority_threshold,
        "test requires legacy priority below high-impact gate"
    );

    let strategic_trace = crate::ai::strategic::strategic_trace_for_shop(&context);
    let carnage_delta = buy_card_delta(&strategic_trace, CardId::Carnage);
    let strategic_decision = strategic_trace
        .compiled_for_action(&carnage_delta.action)
        .expect("strategic compiler should evaluate Carnage");
    assert!(
        strategic_decision.verdict.allows_behavior_acquisition(),
        "test requires strategic compiler to allow the purchase, got {:?}",
        strategic_decision
    );

    let compiled = compile_shop_decision_v1(
        &context,
        &ShopPolicyConfigV1::default(),
        ShopCompileModeV1::ExecuteOne,
    );
    let carnage_plan = compiled
        .candidate_plans
        .iter()
        .find(|candidate| {
            candidate.plan.steps.iter().any(|step| {
                matches!(
                    step,
                    ShopPlanStepV1::BuyCard {
                        card: CardId::Carnage,
                        ..
                    }
                )
            })
        })
        .expect("Carnage shop plan should exist");

    assert_eq!(carnage_plan.evaluation.verdict, ShopPlanVerdictV1::Allow);
    assert!(
        carnage_plan
            .evaluation
            .reasons
            .iter()
            .any(|reason| reason.contains("strategic approval")),
        "strategic approval should be the purchase reason, got {:?}",
        carnage_plan.evaluation.reasons
    );
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
    assert_eq!(
        compiled.selected_plan.legacy_priority,
        relic_candidate_priority
    );
    assert_eq!(
        compiled.selected_plan.steps,
        vec![ShopPlanStepV1::BuyRelic {
            index: 0,
            relic: RelicId::Anchor,
            cost: 146,
        }]
    );
    assert!(compiled
        .candidate_plans
        .iter()
        .any(|candidate| candidate.plan.plan_id == compiled.selected_plan.plan_id));
}

#[test]
fn shop_compiler_blocks_enemy_strength_relic_when_boss_pressure_flags_multi_hit_risk() {
    let mut run_state = RunState::new(1, 0, false, "Ironclad");
    run_state.act_num = 3;
    run_state.floor_num = 46;
    run_state.boss_key = Some(EncounterId::AwakenedOne);
    run_state.gold = 500;
    let mut shop = ShopState::new();
    shop.relics.push(ShopRelic {
        relic_id: RelicId::Brimstone,
        price: 156,
        can_buy: true,
        blocked_reason: None,
    });

    let context = build_shop_decision_context_v1(&run_state, &shop);
    let brimstone_candidate = context
        .candidates
        .iter()
        .find(|candidate| {
            candidate.purchase_target
                == Some(ShopPurchaseTargetV1::Relic {
                    index: 0,
                    relic: RelicId::Brimstone,
                })
        })
        .expect("Brimstone purchase candidate should exist");
    assert!(
        brimstone_candidate
            .risks
            .iter()
            .any(|risk| risk.contains("enemy_strength_multi_hit_risk")),
        "enemy-strength relics should surface boss pressure risk, got {:?}",
        brimstone_candidate.risks
    );

    let compiled = compile_shop_decision_v1(
        &context,
        &ShopPolicyConfigV1::default(),
        ShopCompileModeV1::ExecuteOne,
    );
    let brimstone_plan = compiled
        .candidate_plans
        .iter()
        .find(|candidate| {
            candidate.plan.steps.iter().any(|step| {
                matches!(
                    step,
                    ShopPlanStepV1::BuyRelic {
                        relic: RelicId::Brimstone,
                        ..
                    }
                )
            })
        })
        .expect("Brimstone plan candidate should exist");

    assert_eq!(brimstone_plan.evaluation.verdict, ShopPlanVerdictV1::Block);
    assert!(
        brimstone_plan
            .evaluation
            .reasons
            .iter()
            .any(|reason| reason.contains("enemy_strength_multi_hit_risk")),
        "blocked relic plan should explain the boss pressure risk, got {:?}",
        brimstone_plan.evaluation.reasons
    );
    assert!(
        !matches!(
            compiled.selected_plan.steps.first(),
            Some(ShopPlanStepV1::BuyRelic {
                relic: RelicId::Brimstone,
                ..
            })
        ),
        "selected shop plan must not bypass the boss pressure block"
    );

    let branch_compiled = compile_shop_decision_v1(
        &context,
        &ShopPolicyConfigV1::default(),
        ShopCompileModeV1::BranchTopK { max_plans: 4 },
    );
    assert!(
        branch_compiled.alternatives.iter().all(|plan| {
            !plan.steps.iter().any(|step| {
                matches!(
                    step,
                    ShopPlanStepV1::BuyRelic {
                        relic: RelicId::Brimstone,
                        ..
                    }
                )
            })
        }),
        "branch alternatives must not retain blocked boss-pressure purchases"
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
    let candidate_plan_ids = compiled
        .candidate_plans
        .iter()
        .map(|candidate| candidate.plan.plan_id.as_str())
        .collect::<std::collections::BTreeSet<_>>();
    assert!(candidate_plan_ids.contains(compiled.selected_plan.plan_id.as_str()));
    assert!(compiled
        .alternatives
        .iter()
        .all(|plan| { !plan.steps.is_empty() || matches!(plan.kind, ShopPlanKindV1::Stop) }));
    assert!(compiled
        .alternatives
        .iter()
        .all(|plan| candidate_plan_ids.contains(plan.plan_id.as_str())));
    assert!(compiled
        .alternatives
        .iter()
        .any(|plan| plan.candidate_ids.iter().any(|id| id.starts_with("shop:"))));
}

#[test]
fn compiled_shop_stop_selection_is_also_a_plan_candidate() {
    let mut run_state = RunState::new(1, 0, false, "Ironclad");
    run_state.gold = 10;
    let shop = ShopState::new();

    let context = build_shop_decision_context_v1(&run_state, &shop);
    let compiled = compile_shop_decision_v1(
        &context,
        &ShopPolicyConfigV1::default(),
        ShopCompileModeV1::ExecuteOne,
    );

    assert_eq!(compiled.selected_plan.kind, ShopPlanKindV1::Stop);
    assert!(compiled
        .candidate_plans
        .iter()
        .any(|candidate| candidate.plan.plan_id == compiled.selected_plan.plan_id));
}

#[test]
fn compiled_shop_decision_evaluates_every_candidate_plan() {
    let mut run_state = RunState::new(1, 0, false, "Ironclad");
    run_state.act_num = 3;
    run_state.floor_num = 46;
    run_state.gold = 430;
    run_state.add_card_to_deck_without_interception_from(
        CardId::Doubt,
        0,
        crate::state::selection::DomainEventSource::DeckMutation,
    );
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
        ShopCompileModeV1::BranchTopK { max_plans: 4 },
    );

    assert!(!compiled.candidate_plans.is_empty());
    assert!(compiled
        .candidate_plans
        .iter()
        .all(|candidate| !candidate.evaluation.reasons.is_empty()));
    let selected = compiled
        .candidate_plans
        .iter()
        .find(|candidate| candidate.plan.plan_id == compiled.selected_plan.plan_id)
        .expect("selected plan must come from evaluated candidate plans");
    assert_eq!(selected.evaluation.verdict, ShopPlanVerdictV1::Allow);
    assert!(selected.evaluation.confidence > 0.0);
}

#[test]
fn compiled_shop_plan_evaluations_expose_neutral_components() {
    let mut run_state = RunState::new(1, 0, false, "Ironclad");
    run_state.act_num = 2;
    run_state.floor_num = 18;
    run_state.gold = 500;
    run_state.add_card_to_deck_without_interception_from(
        CardId::Doubt,
        0,
        crate::state::selection::DomainEventSource::DeckMutation,
    );
    let mut shop = ShopState::new();
    shop.cards.push(ShopCard {
        card_id: CardId::ShrugItOff,
        upgrades: 0,
        price: 73,
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
        ShopCompileModeV1::BranchTopK { max_plans: 4 },
    );
    let purge = compiled
        .candidate_plans
        .iter()
        .find(|candidate| {
            candidate.plan.steps.iter().any(|step| {
                matches!(
                    step,
                    ShopPlanStepV1::RemoveCard {
                        card: CardId::Doubt,
                        ..
                    }
                )
            })
        })
        .expect("curse purge plan should exist");
    let card = compiled
        .candidate_plans
        .iter()
        .find(|candidate| {
            candidate.plan.steps.iter().any(|step| {
                matches!(
                    step,
                    ShopPlanStepV1::BuyCard {
                        card: CardId::ShrugItOff,
                        ..
                    }
                )
            })
        })
        .expect("card purchase plan should exist");
    let relic = compiled
        .candidate_plans
        .iter()
        .find(|candidate| {
            candidate.plan.steps.iter().any(|step| {
                matches!(
                    step,
                    ShopPlanStepV1::BuyRelic {
                        relic: RelicId::Anchor,
                        ..
                    }
                )
            })
        })
        .expect("relic purchase plan should exist");
    let potion = compiled
        .candidate_plans
        .iter()
        .find(|candidate| {
            candidate.plan.steps.iter().any(|step| {
                matches!(
                    step,
                    ShopPlanStepV1::BuyPotion {
                        potion: PotionId::FirePotion,
                        ..
                    }
                )
            })
        })
        .expect("potion purchase plan should exist");

    assert_plan_has_component(purge, ShopPlanComponentKindV1::DeckCleanup);
    assert_plan_has_component(card, ShopPlanComponentKindV1::DeckBloatCost);
    assert_plan_has_component(relic, ShopPlanComponentKindV1::RelicValue);
    assert_plan_has_component(potion, ShopPlanComponentKindV1::PotionFill);
    assert_plan_has_component(card, ShopPlanComponentKindV1::GoldSpend);
    assert_plan_has_component(relic, ShopPlanComponentKindV1::LegacyEstimate);
}

#[test]
fn compiled_shop_plan_evaluation_components_do_not_change_selected_plan() {
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

    assert_eq!(
        compiled.selected_plan.steps,
        vec![ShopPlanStepV1::BuyRelic {
            index: 0,
            relic: RelicId::Anchor,
            cost: 146,
        }]
    );
    let selected = compiled
        .candidate_plans
        .iter()
        .find(|candidate| candidate.plan.plan_id == compiled.selected_plan.plan_id)
        .expect("selected plan should be a candidate");
    assert_plan_has_component(selected, ShopPlanComponentKindV1::RelicValue);
    assert_plan_has_component(selected, ShopPlanComponentKindV1::LegacyEstimate);
}

#[test]
fn compiled_shop_plan_evaluations_expose_component_score() {
    let mut run_state = RunState::new(1, 0, false, "Ironclad");
    run_state.gold = 500;
    let mut shop = ShopState::new();
    shop.cards.push(ShopCard {
        card_id: CardId::ShrugItOff,
        upgrades: 0,
        price: 73,
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
        ShopCompileModeV1::BranchTopK { max_plans: 4 },
    );

    for candidate in &compiled.candidate_plans {
        assert!(
            !candidate.evaluation.component_score.explanation.is_empty(),
            "{} should expose a component score explanation",
            candidate.plan.label
        );
    }
    let relic = compiled
        .candidate_plans
        .iter()
        .find(|candidate| {
            candidate.plan.steps.iter().any(|step| {
                matches!(
                    step,
                    ShopPlanStepV1::BuyRelic {
                        relic: RelicId::Anchor,
                        ..
                    }
                )
            })
        })
        .expect("relic purchase plan should exist");
    assert!(
        relic.evaluation.component_score.positive > 0.0,
        "relic plan should have positive component score"
    );
}

#[test]
fn compiled_shop_branch_alternatives_are_sorted_by_component_score() {
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
        ShopCompileModeV1::BranchTopK { max_plans: 4 },
    );
    let scores = compiled
        .alternatives
        .iter()
        .map(|plan| {
            compiled
                .candidate_plans
                .iter()
                .find(|candidate| candidate.plan.plan_id == plan.plan_id)
                .expect("alternative should have matching candidate")
                .evaluation
                .component_score
                .net
        })
        .collect::<Vec<_>>();

    assert!(
        scores.windows(2).all(|pair| pair[0] >= pair[1]),
        "branch alternatives should be sorted by component score, got {scores:?}"
    );
}

#[test]
fn compiled_shop_branch_alternatives_are_evaluated_plan_candidates() {
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
    for alternative in &compiled.alternatives {
        let candidate = compiled
            .candidate_plans
            .iter()
            .find(|candidate| candidate.plan.plan_id == alternative.plan_id)
            .expect("alternative must be backed by an evaluated candidate plan");
        assert_eq!(candidate.evaluation.verdict, ShopPlanVerdictV1::Allow);
        assert!(
            candidate.evaluation.score > 0,
            "branch alternatives should carry evaluator score"
        );
    }
}

#[test]
fn compiled_shop_branch_candidate_plan_ids_are_unique() {
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
    let unique_ids = compiled
        .candidate_plans
        .iter()
        .map(|candidate| candidate.plan.plan_id.as_str())
        .collect::<std::collections::BTreeSet<_>>();

    assert_eq!(
        unique_ids.len(),
        compiled.candidate_plans.len(),
        "plan ids must be unique so inspect and branch alternatives attach the correct evaluation"
    );
}

#[test]
fn compiled_shop_branch_portfolio_evaluation_is_distinct_from_single_action_gate() {
    let mut run_state = RunState::new(1, 0, false, "Ironclad");
    run_state.act_num = 2;
    run_state.floor_num = 18;
    run_state.gold = 274;
    let mut shop = ShopState::new();
    shop.cards.push(ShopCard {
        card_id: CardId::Inflame,
        upgrades: 0,
        price: 73,
        can_buy: true,
        blocked_reason: None,
    });
    shop.relics.push(ShopRelic {
        relic_id: RelicId::OrangePellets,
        price: 151,
        can_buy: true,
        blocked_reason: None,
    });

    let context = build_shop_decision_context_v1(&run_state, &shop);
    let compiled = compile_shop_decision_v1(
        &context,
        &ShopPolicyConfigV1::default(),
        ShopCompileModeV1::BranchTopK { max_plans: 6 },
    );

    let single = compiled
        .candidate_plans
        .iter()
        .find(|candidate| {
            candidate.role == crate::ai::shop_policy_v1::ShopPlanCandidateRoleV1::SingleAction
                && candidate.plan.candidate_ids == vec!["shop:card-0".to_string()]
        })
        .expect("single-action card candidate should exist");
    let portfolio = compiled
        .candidate_plans
        .iter()
        .find(|candidate| {
            candidate.role
                == crate::ai::shop_policy_v1::ShopPlanCandidateRoleV1::PortfolioAlternative
                && candidate.plan.candidate_ids == vec!["shop:card-0".to_string()]
        })
        .expect("portfolio card candidate should exist");

    assert_eq!(single.evaluation.verdict, ShopPlanVerdictV1::Block);
    assert_eq!(portfolio.evaluation.verdict, ShopPlanVerdictV1::Allow);
    assert_ne!(single.plan.plan_id, portfolio.plan.plan_id);
    assert!(
        portfolio
            .evaluation
            .reasons
            .iter()
            .any(|reason| reason.contains("branch exploration")),
        "portfolio alternatives must explain that they are exploration candidates"
    );
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
fn shop_card_priority_does_not_apply_champ_boss_bonus_directly() {
    let mut run_state = RunState::new(1, 0, false, "Ironclad");
    run_state.act_num = 2;
    run_state.floor_num = 18;
    run_state.boss_key = Some(EncounterId::TheChamp);
    run_state.gold = 125;
    let mut no_boss = run_state.clone();
    no_boss.boss_key = None;

    assert_eq!(
        shop_card_conversion_priority_v1(CardId::Carnage, &run_state),
        shop_card_conversion_priority_v1(CardId::Carnage, &no_boss),
        "shop raw purchase priority must not encode The Champ transition-burst policy"
    );
}

#[test]
fn shop_card_priority_does_not_apply_champ_flex_bonus_directly() {
    let mut run_state = RunState::new(1, 0, false, "Ironclad");
    run_state.act_num = 2;
    run_state.floor_num = 18;
    run_state.boss_key = Some(EncounterId::TheChamp);
    run_state.gold = 125;
    run_state.add_card_to_deck(CardId::HeavyBlade);
    let mut no_boss = run_state.clone();
    no_boss.boss_key = None;

    assert_eq!(
        shop_card_conversion_priority_v1(CardId::Flex, &run_state),
        shop_card_conversion_priority_v1(CardId::Flex, &no_boss),
        "Flex conversion potential must be modeled by shared strength/component profiles, not by a shop-local Champ bonus"
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

fn assert_plan_has_component(
    candidate: &crate::ai::shop_policy_v1::ShopPlanCandidateV1,
    kind: ShopPlanComponentKindV1,
) {
    assert!(
        candidate
            .evaluation
            .components
            .iter()
            .any(|component| component.kind == kind),
        "{} should include component {kind:?}, got {:?}",
        candidate.plan.label,
        candidate.evaluation.components
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

fn assert_delta_has_boss_tax(delta: &crate::ai::strategic::CandidateDelta, tax: StrategicBossTax) {
    assert!(
        delta
            .positive
            .iter()
            .any(|entry| entry.kind == PressureKind::BossTax(tax)),
        "delta should include positive boss tax {tax:?}, got {:?}",
        delta.positive
    );
}
