use sts_simulator::ai::combat_search_v2::{
    explain_combat_search_v2_initial_decision, CombatSearchV2Config, CombatSearchV2RolloutPolicy,
    CombatSearchV2SetupBiasPolicy,
};
use sts_simulator::content::cards::CardId;
use sts_simulator::content::monsters::EnemyId;
use sts_simulator::runtime::combat::CombatCard;
use sts_simulator::state::core::EngineState;
use sts_simulator::test_support::{blank_test_combat, planned_monster};

use super::selection::select_duel_candidate_indices;

#[test]
fn role_duel_selection_deduplicates_candidates_and_tracks_reasons() {
    let mut combat = blank_test_combat();
    let mut monster = planned_monster(EnemyId::JawWorm, 1);
    monster.current_hp = 50;
    monster.max_hp = 50;
    combat.entities.monsters = vec![monster];
    combat.zones.hand = vec![
        CombatCard::new(CardId::Strike, 1),
        CombatCard::new(CardId::DemonForm, 2),
    ];
    let microscope = explain_combat_search_v2_initial_decision(
        &EngineState::CombatPlayerTurn,
        &combat,
        CombatSearchV2Config {
            max_nodes: 20,
            rollout_policy: CombatSearchV2RolloutPolicy::Disabled,
            setup_bias_policy: CombatSearchV2SetupBiasPolicy::KeyCardOnline,
            ..CombatSearchV2Config::default()
        },
    );

    let selections = select_duel_candidate_indices(&microscope);
    let action_keys = selections
        .iter()
        .map(|selection| {
            microscope.candidates[selection.candidate_index]
                .action_key
                .as_str()
        })
        .collect::<Vec<_>>();

    assert_eq!(
        action_keys.len(),
        action_keys
            .iter()
            .collect::<std::collections::BTreeSet<_>>()
            .len()
    );
    assert!(selections.iter().any(|selection| {
        selection.reasons.contains(&"first_key_setup_card")
            && microscope.candidates[selection.candidate_index]
                .action_key
                .contains("Demon Form")
    }));
}
