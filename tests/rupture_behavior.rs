use sts_simulator::content::powers::store;
use sts_simulator::engine::action_handlers::execute_action;
use sts_simulator::fixtures::author_spec::AuthorCardSpec;
use sts_simulator::fixtures::combat_start_spec::{compile_combat_start_spec, CombatStartSpec};
use sts_simulator::runtime::action::Action;
use sts_simulator::runtime::combat::{CombatState, PowerId};

// Oracle: Java source
// Evidence:
// - cardcrawl/powers/RupturePower.java: wasHPLost() uses addToTop(ApplyPower Strength)
// - cardcrawl/cards/red/LimitBreak.java: use() uses addToBot(new LimitBreakAction())
//
// Therefore, in a queued sequence LoseHp(player,self) -> LimitBreak, the Strength from
// Rupture must resolve before LimitBreak executes.

fn jaw_worm_combat() -> CombatState {
    let spec = CombatStartSpec {
        name: "rupture-behavior".to_string(),
        player_class: "ironclad".to_string(),
        ascension_level: 0,
        encounter_id: "jaw_worm".to_string(),
        room_type: "monster".to_string(),
        seed: 1,
        player_current_hp: 80,
        player_max_hp: 80,
        relics: vec![],
        potions: vec![],
        master_deck: vec![
            AuthorCardSpec::Simple("Strike_R".to_string()),
            AuthorCardSpec::Simple("Strike_R".to_string()),
            AuthorCardSpec::Simple("Strike_R".to_string()),
            AuthorCardSpec::Simple("Strike_R".to_string()),
            AuthorCardSpec::Simple("Strike_R".to_string()),
            AuthorCardSpec::Simple("Defend_R".to_string()),
            AuthorCardSpec::Simple("Defend_R".to_string()),
            AuthorCardSpec::Simple("Defend_R".to_string()),
            AuthorCardSpec::Simple("Defend_R".to_string()),
            AuthorCardSpec::Simple("Bash".to_string()),
        ],
    };

    let (_engine_state, combat) = compile_combat_start_spec(&spec).expect("compile jaw worm spec");
    combat
}

fn drain_action_queue(state: &mut CombatState) {
    while let Some(action) = state.pop_next_action() {
        execute_action(action, state);
    }
}

#[test]
fn rupture_hp_loss_triggers_before_following_bottom_actions() {
    let mut combat = jaw_worm_combat();

    execute_action(
        Action::ApplyPower {
            source: 0,
            target: 0,
            power_id: PowerId::Rupture,
            amount: 1,
        },
        &mut combat,
    );

    combat.queue_action_back(Action::LoseHp {
        target: 0,
        amount: 1,
        triggers_rupture: true,
    });
    combat.queue_action_back(Action::LimitBreak);

    drain_action_queue(&mut combat);

    assert_eq!(combat.entities.player.current_hp, 79);
    assert_eq!(store::power_amount(&combat, 0, PowerId::Rupture), 1);
    assert_eq!(store::power_amount(&combat, 0, PowerId::Strength), 2);
}
