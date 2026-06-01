use super::*;
use crate::runtime::action::ActionInfo;
use std::collections::VecDeque;

#[test]
fn card_zones_draw_pile_top_is_index_zero() {
    let mut zones = CardZones {
        draw_pile: vec![CombatCard::new(CardId::Strike, 1)],
        hand: vec![],
        discard_pile: vec![],
        exhaust_pile: vec![],
        limbo: vec![],
        queued_cards: VecDeque::new(),
        card_uuid_counter: 1,
    };

    zones.add_to_draw_pile_top(CombatCard::new(CardId::Defend, 2));
    zones.add_to_draw_pile_bottom(CombatCard::new(CardId::Bash, 3));

    assert_eq!(
        zones.draw_top_card().map(|card| card.id),
        Some(CardId::Defend)
    );
    assert_eq!(
        zones.draw_top_card().map(|card| card.id),
        Some(CardId::Strike)
    );
    assert_eq!(
        zones.draw_top_card().map(|card| card.id),
        Some(CardId::Bash)
    );
}

#[test]
fn card_zones_random_spot_maps_java_bottom_index_to_rust_top_index() {
    let mut zones = CardZones {
        draw_pile: vec![
            CombatCard::new(CardId::Strike, 1),
            CombatCard::new(CardId::Defend, 2),
            CombatCard::new(CardId::Bash, 3),
        ],
        hand: vec![],
        discard_pile: vec![],
        exhaust_pile: vec![],
        limbo: vec![],
        queued_cards: VecDeque::new(),
        card_uuid_counter: 3,
    };

    zones.add_to_draw_pile_random_spot_from_java_index(CombatCard::new(CardId::Wound, 4), 0);
    assert_eq!(zones.draw_pile[3].id, CardId::Wound);

    zones.add_to_draw_pile_random_spot_from_java_index(CombatCard::new(CardId::Burn, 5), 3);
    assert_eq!(zones.draw_pile[1].id, CardId::Burn);
    assert_eq!(zones.draw_pile[0].id, CardId::Strike);
}

#[test]
fn card_zones_discard_pile_preserves_java_card_group_order() {
    let mut zones = CardZones {
        draw_pile: vec![],
        hand: vec![],
        discard_pile: vec![],
        exhaust_pile: vec![],
        limbo: vec![],
        queued_cards: VecDeque::new(),
        card_uuid_counter: 0,
    };

    zones.add_to_discard_pile_top(CombatCard::new(CardId::Strike, 1));
    zones.add_to_discard_pile_top(CombatCard::new(CardId::Defend, 2));

    assert_eq!(zones.discard_pile[0].id, CardId::Strike);
    assert_eq!(zones.discard_pile[1].id, CardId::Defend);
}

#[test]
fn card_zones_exhaust_pile_preserves_java_card_group_order() {
    let mut zones = CardZones {
        draw_pile: vec![],
        hand: vec![],
        discard_pile: vec![],
        exhaust_pile: vec![],
        limbo: vec![],
        queued_cards: VecDeque::new(),
        card_uuid_counter: 0,
    };

    zones.add_to_exhaust_pile_top(CombatCard::new(CardId::Strike, 1));
    zones.add_to_exhaust_pile_top(CombatCard::new(CardId::Defend, 2));

    assert_eq!(zones.exhaust_pile[0].id, CardId::Strike);
    assert_eq!(zones.exhaust_pile[1].id, CardId::Defend);
}

#[test]
fn card_zones_uuid_helper_updates_java_battle_instances_only() {
    let mut zones = CardZones {
        draw_pile: vec![CombatCard::new(CardId::Rampage, 7)],
        hand: vec![CombatCard::new(CardId::Rampage, 7)],
        discard_pile: vec![CombatCard::new(CardId::Strike, 8)],
        exhaust_pile: vec![CombatCard::new(CardId::Rampage, 7)],
        limbo: vec![CombatCard::new(CardId::Rampage, 7)],
        queued_cards: VecDeque::from([QueuedCardPlay {
            card: CombatCard::new(CardId::Rampage, 7),
            target: None,
            energy_on_use: 1,
            ignore_energy_total: false,
            autoplay: false,
            random_target: false,
            is_end_turn_autoplay: false,
            purge_on_use: false,
            source: QueuedCardSource::Normal,
        }]),
        card_uuid_counter: 8,
    };

    zones.for_each_java_battle_instance_mut_by_uuid(7, |card| {
        card.misc_value += 2;
    });

    assert_eq!(zones.hand[0].misc_value, 2);
    assert_eq!(zones.draw_pile[0].misc_value, 2);
    assert_eq!(zones.exhaust_pile[0].misc_value, 2);
    assert_eq!(zones.limbo[0].misc_value, 2);
    assert_eq!(zones.discard_pile[0].misc_value, 0);
    assert_eq!(
        zones.queued_cards[0].card.misc_value, 0,
        "queued cards are not Java CardGroup battle instances"
    );

    zones.for_each_queued_instance_mut_by_uuid(7, |card| {
        card.misc_value += 3;
    });
    assert_eq!(zones.queued_cards[0].card.misc_value, 3);
}

#[test]
fn combat_card_update_cost_preserves_java_cost_for_turn_difference() {
    let mut card = CombatCard::new(CardId::BloodForBlood, 1);
    card.set_cost_for_turn_java(1);

    card.update_cost_java(-1);

    assert_eq!(card.combat_cost_without_turn_override_java(), 3);
    assert_eq!(card.cost_for_turn_java(), 0);
    assert_eq!(card.get_cost(), 0);
}

#[test]
fn combat_card_modify_cost_for_combat_matches_java_zero_turn_branch() {
    let mut card = CombatCard::new(CardId::BloodForBlood, 1);
    card.set_cost_for_turn_java(0);

    card.modify_cost_for_combat_java(-1);

    assert_eq!(card.combat_cost_without_turn_override_java(), 3);
    assert_eq!(card.cost_for_turn_java(), 0);
}

#[test]
fn combat_card_can_set_combat_cost_without_erasing_turn_override() {
    let mut card = CombatCard::new(CardId::BloodForBlood, 1);
    card.set_cost_for_turn_java(0);

    card.set_combat_cost_preserving_turn_java(1);

    assert_eq!(card.combat_cost_without_turn_override_java(), 1);
    assert_eq!(card.cost_for_turn_java(), 0);
}

#[test]
fn java_initialize_deck_order_places_innate_on_rust_top_after_reversing() {
    let mut state = crate::test_support::blank_test_combat();
    state.zones.draw_pile = vec![
        CombatCard::new(CardId::Strike, 1),
        CombatCard::new(CardId::Writhe, 2),
        CombatCard::new(CardId::Defend, 3),
        CombatCard::new(CardId::Pride, 4),
    ];

    state.apply_java_initialize_deck_order_after_shuffle();

    assert_eq!(
        state
            .zones
            .draw_pile
            .iter()
            .map(|card| card.id)
            .collect::<Vec<_>>(),
        vec![
            CardId::Pride,
            CardId::Writhe,
            CardId::Defend,
            CardId::Strike
        ]
    );
}

#[test]
fn engine_runtime_queue_actions_matches_java_add_to_top_order() {
    let mut engine = EngineRuntime {
        action_queue: VecDeque::new(),
    };

    engine.push_back(Action::DrawCards(99));
    engine.queue_actions(smallvec::smallvec![
        ActionInfo {
            action: Action::DrawCards(1),
            insertion_mode: AddTo::Bottom,
        },
        ActionInfo {
            action: Action::GainEnergy { amount: 2 },
            insertion_mode: AddTo::Top,
        },
        ActionInfo {
            action: Action::GainBlock {
                target: 0,
                amount: 3,
            },
            insertion_mode: AddTo::Top,
        },
    ]);

    assert_eq!(
        engine.pop_front(),
        Some(Action::GainBlock {
            target: 0,
            amount: 3,
        }),
        "later addToTop calls should execute before earlier top calls"
    );
    assert_eq!(
        engine.pop_front(),
        Some(Action::GainEnergy { amount: 2 }),
        "earlier addToTop calls should remain ahead of existing queued actions"
    );
    assert_eq!(
        engine.pop_front(),
        Some(Action::DrawCards(99)),
        "existing queued actions remain ahead of later addToBot actions"
    );
    assert_eq!(
        engine.pop_front(),
        Some(Action::DrawCards(1)),
        "addToBot actions should append after existing queued actions"
    );
}

#[test]
fn turn_runtime_begin_next_player_turn_resets_turn_counters_and_energy() {
    let mut turn = TurnRuntime {
        turn_count: 2,
        current_phase: CombatPhase::MonsterTurn,
        energy: 1,
        turn_start_draw_modifier: -1,
        counters: EphemeralCounters {
            cards_played_this_turn: 4,
            attacks_played_this_turn: 2,
            cards_discarded_this_turn: 3,
            card_ids_played_this_turn: vec![CardId::Strike, CardId::Defend],
            card_ids_played_this_combat: vec![CardId::Zap],
            orbs_channeled_this_turn: vec![OrbId::Lightning],
            orbs_channeled_this_combat: vec![OrbId::Lightning, OrbId::Frost],
            mantra_gained_this_combat: 4,
            times_damaged_this_combat: 3,
            victory_triggered: false,
            discovery_cost_for_turn: None,
            early_end_turn_pending: true,
            skip_monster_turn_pending: true,
            player_escaping: false,
            escape_pending_reward: false,
        },
    };

    turn.begin_next_player_turn(3);

    assert_eq!(turn.turn_count, 3);
    assert_eq!(turn.current_phase, CombatPhase::PlayerTurn);
    assert_eq!(turn.energy, 3);
    assert_eq!(turn.counters.cards_played_this_turn, 0);
    assert_eq!(turn.counters.attacks_played_this_turn, 0);
    assert_eq!(turn.counters.cards_discarded_this_turn, 0);
    assert!(turn.counters.card_ids_played_this_turn.is_empty());
    assert_eq!(
        turn.counters.card_ids_played_this_combat,
        vec![CardId::Zap],
        "combat-wide played-card history should remain untouched"
    );
    assert!(turn.counters.orbs_channeled_this_turn.is_empty());
    assert_eq!(
        turn.counters.orbs_channeled_this_combat,
        vec![OrbId::Lightning, OrbId::Frost],
        "combat-wide orb channel history should remain untouched"
    );
    assert_eq!(
        turn.counters.mantra_gained_this_combat, 4,
        "combat-wide mantra history should remain untouched"
    );
    assert_eq!(
        turn.counters.times_damaged_this_combat, 3,
        "combat-wide counters should remain untouched"
    );
    assert!(
        turn.counters.early_end_turn_pending,
        "unrelated flags should not be reset by turn-start setup"
    );
    assert!(
        !turn.counters.skip_monster_turn_pending,
        "Java clears room.skipMonsterTurn when the next player turn starts"
    );
    assert_eq!(turn.turn_start_draw_modifier, -1);
}

#[test]
fn turn_runtime_counter_helpers_update_expected_flags() {
    let mut turn = TurnRuntime::fresh_player_turn(3);

    turn.mark_early_end_turn_pending();
    turn.mark_player_escaping();
    turn.mark_escape_pending_reward();
    turn.mark_victory_triggered();
    turn.increment_cards_played();
    turn.increment_attacks_played();
    turn.increment_times_damaged_this_combat();
    turn.set_discovery_cost_for_turn(Some(1));

    assert!(turn.counters.early_end_turn_pending);
    assert!(turn.counters.player_escaping);
    assert!(turn.counters.escape_pending_reward);
    assert!(turn.counters.victory_triggered);
    assert_eq!(turn.counters.cards_played_this_turn, 1);
    assert_eq!(turn.counters.attacks_played_this_turn, 1);
    assert_eq!(turn.counters.times_damaged_this_combat, 1);
    assert_eq!(turn.take_discovery_cost_for_turn(), Some(1));
    assert_eq!(turn.take_discovery_cost_for_turn(), None);
}

#[test]
fn turn_runtime_can_clear_transition_flags_without_touching_other_counters() {
    let mut turn = TurnRuntime::fresh_player_turn(3);
    turn.mark_early_end_turn_pending();
    turn.mark_escape_pending_reward();
    turn.increment_times_damaged_this_combat();

    turn.clear_early_end_turn_pending();
    turn.clear_escape_pending_reward();

    assert!(!turn.counters.early_end_turn_pending);
    assert!(!turn.counters.escape_pending_reward);
    assert_eq!(turn.counters.times_damaged_this_combat, 1);
}

#[test]
fn monster_turn_plan_and_preview_follow_planned_visible_spec() {
    let spec = MonsterMoveSpec::Attack(AttackSpec {
        base_damage: 6,
        hits: 2,
        damage_kind: DamageKind::Normal,
    });
    let monster = MonsterEntity {
        id: 1,
        monster_type: 0,
        current_hp: 30,
        max_hp: 30,
        block: 0,
        slot: 0,
        is_dying: false,
        is_escaped: false,
        half_dead: false,
        move_state: MonsterMoveState {
            planned_move_id: 7,
            history: VecDeque::from([1, 7]),
            planned_steps: Some(spec.to_steps()),
            planned_visible_spec: Some(spec.clone()),
        },
        logical_position: 0,
        hexaghost: HexaghostRuntimeState::default(),
        louse: LouseRuntimeState::default(),
        jaw_worm: JawWormRuntimeState::default(),
        thief: ThiefRuntimeState::default(),
        byrd: ByrdRuntimeState::default(),
        chosen: ChosenRuntimeState::default(),
        snecko: SneckoRuntimeState::default(),
        shelled_parasite: ShelledParasiteRuntimeState::default(),
        bronze_automaton: BronzeAutomatonRuntimeState::default(),
        bronze_orb: BronzeOrbRuntimeState::default(),
        book_of_stabbing: BookOfStabbingRuntimeState::default(),
        collector: CollectorRuntimeState::default(),
        champ: ChampRuntimeState::default(),
        awakened_one: AwakenedOneRuntimeState::default(),
        corrupt_heart: CorruptHeartRuntimeState::default(),
        writhing_mass: WrithingMassRuntimeState::default(),
        spiker: SpikerRuntimeState::default(),
        spire_shield: SpireShieldRuntimeState::default(),
        spire_spear: SpireSpearRuntimeState::default(),
        slaver_red: SlaverRedRuntimeState::default(),
        gremlin_leader: GremlinLeaderRuntimeState::default(),
        gremlin_nob: GremlinNobRuntimeState::default(),
        gremlin_wizard: GremlinWizardRuntimeState::default(),
        cultist: CultistRuntimeState::default(),
        sentry: SentryRuntimeState::default(),
        slime_boss: SlimeBossRuntimeState::default(),
        large_slime: LargeSlimeRuntimeState::default(),
        spheric_guardian: SphericGuardianRuntimeState::default(),
        reptomancer: ReptomancerRuntimeState::default(),
        darkling: DarklingRuntimeState::default(),
        nemesis: NemesisRuntimeState::default(),
        giant_head: GiantHeadRuntimeState::default(),
        time_eater: TimeEaterRuntimeState::default(),
        donu: DonuRuntimeState::default(),
        deca: DecaRuntimeState::default(),
        transient: TransientRuntimeState::default(),
        exploder: ExploderRuntimeState::default(),
        maw: MawRuntimeState::default(),
        snake_dagger: SnakeDaggerRuntimeState::default(),
        lagavulin: LagavulinRuntimeState::default(),
        guardian: GuardianRuntimeState::default(),
    };

    let plan = monster.turn_plan();
    let preview = crate::sim::combat_projection::project_monster_move_preview(&monster);

    assert_eq!(plan.move_id, 7);
    assert!(matches!(plan.summary_spec(), MonsterMoveSpec::Attack(_)));
    assert_eq!(preview.damage_per_hit, Some(6));
    assert_eq!(preview.hits, 2);
    assert_eq!(preview.total_damage, Some(12));
    assert!(crate::sim::combat_projection::monster_has_visible_attack(
        &monster
    ));
    assert_eq!(
        crate::sim::combat_projection::monster_preview_total_damage(&monster),
        12
    );
}

#[test]
fn monster_turn_plan_uses_planned_steps_damage_without_visible_spec() {
    let monster = MonsterEntity {
        id: 1,
        monster_type: 0,
        current_hp: 12,
        max_hp: 12,
        block: 0,
        slot: 0,
        is_dying: false,
        is_escaped: false,
        half_dead: false,
        move_state: MonsterMoveState {
            planned_move_id: 3,
            history: VecDeque::from([4, 3]),
            planned_steps: Some(
                MonsterMoveSpec::Attack(AttackSpec {
                    base_damage: 7,
                    hits: 1,
                    damage_kind: DamageKind::Normal,
                })
                .to_steps(),
            ),
            planned_visible_spec: None,
        },
        logical_position: 0,
        hexaghost: HexaghostRuntimeState::default(),
        louse: LouseRuntimeState::default(),
        jaw_worm: JawWormRuntimeState::default(),
        thief: ThiefRuntimeState::default(),
        byrd: ByrdRuntimeState::default(),
        chosen: ChosenRuntimeState::default(),
        snecko: SneckoRuntimeState::default(),
        shelled_parasite: ShelledParasiteRuntimeState::default(),
        bronze_automaton: BronzeAutomatonRuntimeState::default(),
        bronze_orb: BronzeOrbRuntimeState::default(),
        book_of_stabbing: BookOfStabbingRuntimeState::default(),
        collector: CollectorRuntimeState::default(),
        champ: ChampRuntimeState::default(),
        awakened_one: AwakenedOneRuntimeState::default(),
        corrupt_heart: CorruptHeartRuntimeState::default(),
        writhing_mass: WrithingMassRuntimeState::default(),
        spiker: SpikerRuntimeState::default(),
        spire_shield: SpireShieldRuntimeState::default(),
        spire_spear: SpireSpearRuntimeState::default(),
        slaver_red: SlaverRedRuntimeState::default(),
        gremlin_leader: GremlinLeaderRuntimeState::default(),
        gremlin_nob: GremlinNobRuntimeState::default(),
        gremlin_wizard: GremlinWizardRuntimeState::default(),
        cultist: CultistRuntimeState::default(),
        sentry: SentryRuntimeState::default(),
        slime_boss: SlimeBossRuntimeState::default(),
        large_slime: LargeSlimeRuntimeState::default(),
        spheric_guardian: SphericGuardianRuntimeState::default(),
        reptomancer: ReptomancerRuntimeState::default(),
        darkling: DarklingRuntimeState::default(),
        nemesis: NemesisRuntimeState::default(),
        giant_head: GiantHeadRuntimeState::default(),
        time_eater: TimeEaterRuntimeState::default(),
        donu: DonuRuntimeState::default(),
        deca: DecaRuntimeState::default(),
        transient: TransientRuntimeState::default(),
        exploder: ExploderRuntimeState::default(),
        maw: MawRuntimeState::default(),
        snake_dagger: SnakeDaggerRuntimeState::default(),
        lagavulin: LagavulinRuntimeState::default(),
        guardian: GuardianRuntimeState::default(),
    };

    let plan = monster.turn_plan();

    assert_eq!(plan.move_id, 3);
    assert_eq!(plan.attack().map(|attack| attack.base_damage), Some(7));
    assert_eq!(
        crate::sim::combat_projection::project_monster_move_preview(&monster).damage_per_hit,
        Some(7)
    );
}

#[test]
fn monster_turn_plan_hides_visible_spec_for_half_dead_or_dying_monsters() {
    let monster = MonsterEntity {
        id: 1,
        monster_type: 0,
        current_hp: 0,
        max_hp: 56,
        block: 0,
        slot: 0,
        is_dying: false,
        is_escaped: false,
        half_dead: true,
        move_state: MonsterMoveState {
            planned_move_id: 4,
            history: VecDeque::from([3, 4]),
            planned_steps: Some(
                MonsterMoveSpec::Attack(AttackSpec {
                    base_damage: 13,
                    hits: 1,
                    damage_kind: DamageKind::Normal,
                })
                .to_steps(),
            ),
            planned_visible_spec: Some(MonsterMoveSpec::Attack(AttackSpec {
                base_damage: 13,
                hits: 1,
                damage_kind: DamageKind::Normal,
            })),
        },
        logical_position: 0,
        hexaghost: HexaghostRuntimeState::default(),
        louse: LouseRuntimeState::default(),
        jaw_worm: JawWormRuntimeState::default(),
        thief: ThiefRuntimeState::default(),
        byrd: ByrdRuntimeState::default(),
        chosen: ChosenRuntimeState::default(),
        snecko: SneckoRuntimeState::default(),
        shelled_parasite: ShelledParasiteRuntimeState::default(),
        bronze_automaton: BronzeAutomatonRuntimeState::default(),
        bronze_orb: BronzeOrbRuntimeState::default(),
        book_of_stabbing: BookOfStabbingRuntimeState::default(),
        collector: CollectorRuntimeState::default(),
        champ: ChampRuntimeState::default(),
        awakened_one: AwakenedOneRuntimeState::default(),
        corrupt_heart: CorruptHeartRuntimeState::default(),
        writhing_mass: WrithingMassRuntimeState::default(),
        spiker: SpikerRuntimeState::default(),
        spire_shield: SpireShieldRuntimeState::default(),
        spire_spear: SpireSpearRuntimeState::default(),
        slaver_red: SlaverRedRuntimeState::default(),
        gremlin_leader: GremlinLeaderRuntimeState::default(),
        gremlin_nob: GremlinNobRuntimeState::default(),
        gremlin_wizard: GremlinWizardRuntimeState::default(),
        cultist: CultistRuntimeState::default(),
        sentry: SentryRuntimeState::default(),
        slime_boss: SlimeBossRuntimeState::default(),
        large_slime: LargeSlimeRuntimeState::default(),
        spheric_guardian: SphericGuardianRuntimeState::default(),
        reptomancer: ReptomancerRuntimeState::default(),
        darkling: DarklingRuntimeState::default(),
        nemesis: NemesisRuntimeState::default(),
        giant_head: GiantHeadRuntimeState::default(),
        time_eater: TimeEaterRuntimeState::default(),
        donu: DonuRuntimeState::default(),
        deca: DecaRuntimeState::default(),
        transient: TransientRuntimeState::default(),
        exploder: ExploderRuntimeState::default(),
        maw: MawRuntimeState::default(),
        snake_dagger: SnakeDaggerRuntimeState::default(),
        lagavulin: LagavulinRuntimeState::default(),
        guardian: GuardianRuntimeState::default(),
    };

    let plan = monster.turn_plan();
    let preview = crate::sim::combat_projection::project_monster_move_preview(&monster);

    assert_eq!(plan.move_id, 4);
    assert!(matches!(plan.summary_spec(), MonsterMoveSpec::Unknown));
    assert_eq!(preview.damage_per_hit, None);
    assert!(!crate::sim::combat_projection::monster_has_visible_attack(
        &monster
    ));
    assert_eq!(
        crate::sim::combat_projection::monster_preview_total_damage(&monster),
        0
    );
}
