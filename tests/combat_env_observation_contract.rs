use sts_simulator::bot::harness::{
    CombatAction, CombatEnv, CombatEnvDrawOrderVariant, CombatEnvSpec, CombatEpisodeOutcome,
};
use sts_simulator::content::cards::CardId;
use sts_simulator::fixtures::combat_start_spec::{
    compile_combat_start_spec_with_seed, CombatStartSpec,
};
use sts_simulator::runtime::combat::{CombatCard, CombatState};
use sts_simulator::state::core::EngineState;
use sts_simulator::test_support::blank_test_combat;

fn card(id: CardId, uuid: u32) -> CombatCard {
    CombatCard::new(id, uuid)
}

fn draw_uuids(combat: &CombatState) -> Vec<u32> {
    combat
        .zones
        .draw_pile
        .iter()
        .map(|card| card.uuid)
        .collect()
}

fn spec_with_hidden_draw_pile() -> CombatEnvSpec {
    let mut combat = blank_test_combat();
    combat.zones.hand = vec![card(CardId::Defend, 100)];
    combat.zones.draw_pile = vec![
        card(CardId::Bash, 9001),
        card(CardId::Anger, 9002),
        card(CardId::PommelStrike, 9003),
        card(CardId::ShrugItOff, 9004),
    ];
    CombatEnvSpec::from_combat(
        "observation_contract",
        7,
        EngineState::CombatPlayerTurn,
        combat,
    )
}

#[test]
fn combat_env_observation_exposes_only_hidden_pile_counts() {
    let env = CombatEnv::new(spec_with_hidden_draw_pile());
    let observation = env.observation();

    assert_eq!(observation.draw_count, 4);
    assert_eq!(observation.hand.len(), 1);

    let payload = serde_json::to_value(&observation).expect("serialize observation");
    assert!(payload.get("draw_pile").is_none());
    assert!(payload.get("discard_pile").is_none());
    assert!(payload.get("exhaust_pile").is_none());

    let serialized = serde_json::to_string(&payload).expect("serialize observation json");
    assert!(!serialized.contains("Bash"));
    assert!(!serialized.contains("Anger"));
    assert!(!serialized.contains("PommelStrike"));
    assert!(!serialized.contains("ShrugItOff"));
    assert!(!serialized.contains("9001"));
    assert!(!serialized.contains("9002"));
    assert!(!serialized.contains("9003"));
    assert!(!serialized.contains("9004"));
}

#[test]
fn reshuffle_draw_variant_changes_hidden_order_not_public_counts() {
    let spec = spec_with_hidden_draw_pile();
    let before_order = draw_uuids(&spec.initial_combat);
    let mut reshuffled = None;

    for seed in 1..=32 {
        let mut candidate = spec.clone();
        candidate.apply_draw_order_variant(CombatEnvDrawOrderVariant::ReshuffleDraw, seed);
        if draw_uuids(&candidate.initial_combat) != before_order {
            reshuffled = Some(candidate);
            break;
        }
    }

    let reshuffled = reshuffled.expect("test seeds should produce at least one changed order");
    let after_order = draw_uuids(&reshuffled.initial_combat);
    let mut before_sorted = before_order.clone();
    let mut after_sorted = after_order.clone();
    before_sorted.sort_unstable();
    after_sorted.sort_unstable();

    assert_ne!(after_order, before_order);
    assert_eq!(after_sorted, before_sorted);

    let env = CombatEnv::new(reshuffled);
    let observation = env.observation();
    assert_eq!(observation.draw_count, 4);
    assert!(observation
        .run_rule_context_summary
        .as_deref()
        .unwrap_or_default()
        .contains("draw_order_variant=reshuffle_draw"));
}

#[test]
fn combat_env_marks_reward_screen_after_kill_as_victory() {
    let start_spec: CombatStartSpec = serde_json::from_value(serde_json::json!({
        "name": "one_hit_jaw_worm",
        "player_class": "Ironclad",
        "ascension_level": 0,
        "encounter_id": "JawWorm",
        "room_type": "MonsterRoom",
        "seed": 11,
        "player_current_hp": 80,
        "player_max_hp": 80,
        "relics": [],
        "potions": [],
        "master_deck": ["Strike_R"]
    }))
    .expect("valid start spec");
    let (engine_state, mut combat) =
        compile_combat_start_spec_with_seed(&start_spec, 11).expect("compile start spec");
    for monster in &mut combat.entities.monsters {
        monster.current_hp = 1;
        monster.max_hp = 1;
        monster.block = 0;
    }
    combat.zones.hand = vec![card(CardId::Strike, 100)];
    combat.zones.draw_pile.clear();
    combat.zones.discard_pile.clear();
    combat.zones.exhaust_pile.clear();

    let mut env = CombatEnv::new(CombatEnvSpec::from_combat(
        "reward_screen_victory",
        11,
        engine_state,
        combat,
    ));
    let step = env
        .step(CombatAction::PlayCard {
            card_index: 0,
            target: Some(1),
        })
        .expect("strike should be legal and kill the only monster");

    assert!(step.done);
    assert_eq!(step.outcome, Some(CombatEpisodeOutcome::Victory));
    assert!(matches!(
        env.current_engine_state(),
        EngineState::RewardScreen(_)
    ));
    assert!(step.observation.monsters.is_empty());
}
