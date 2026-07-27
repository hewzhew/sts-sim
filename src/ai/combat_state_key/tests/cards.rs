use super::*;

#[test]
fn exact_key_keeps_transient_multi_target_damage() {
    let mut baseline = blank_test_combat();
    let mut card = CombatCard::new(CardId::Cleave, 7);
    card.multi_damage = smallvec::smallvec![8, 8];
    baseline.zones.hand = vec![card];

    let mut changed = baseline.clone();
    changed.zones.hand[0].multi_damage[1] = 9;

    assert_ne!(
        combat_exact_state_key(&EngineState::CombatPlayerTurn, &baseline),
        combat_exact_state_key(&EngineState::CombatPlayerTurn, &changed),
        "the inline key representation must preserve exact card semantics"
    );
}
