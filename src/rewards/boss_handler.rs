use crate::rewards::state::BossRelicChoiceState;
use crate::state::core::{ClientInput, EngineState};
use crate::state::run::RunState;
use crate::state::selection::DomainEventSource;

pub fn handle(
    run_state: &mut RunState,
    state: &mut BossRelicChoiceState,
    input: Option<ClientInput>,
) -> Option<EngineState> {
    if let Some(in_val) = input {
        match in_val {
            ClientInput::SubmitRelicChoice(idx) => {
                if idx < state.relics.len() {
                    let chosen_relic = state.relics[idx];

                    // Java obtains the selected boss relic in the current boss chest
                    // room. The dungeon transition happens only after the boss chest
                    // is left, so state-interrupting on-equip effects must resolve
                    // before advance_act().
                    if let Some(next_state) = run_state.obtain_boss_relic_choice_with_source(
                        chosen_relic,
                        EngineState::MapNavigation,
                        DomainEventSource::BossRelicChoice,
                    ) {
                        run_state.pending_boss_act_transition = true;
                        return Some(next_state);
                    }

                    run_state.advance_act();
                    return Some(EngineState::MapNavigation);
                }
            }
            ClientInput::Proceed | ClientInput::Cancel => {
                // Skipping Boss Relic
                run_state.advance_act();
                return Some(EngineState::MapNavigation);
            }
            _ => {}
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::handle;
    use crate::content::relics::RelicId;
    use crate::rewards::state::BossRelicChoiceState;
    use crate::state::core::{ClientInput, EngineState};
    use crate::state::run::RunState;
    use crate::state::selection::{SelectionResolution, SelectionScope};

    #[test]
    fn boss_relic_choice_obtains_normal_relic_before_advancing_act() {
        let mut run_state = RunState::new(7, 0, false, "Ironclad");
        let mut boss_state = BossRelicChoiceState::new(vec![RelicId::CoffeeDripper]);

        let next = handle(
            &mut run_state,
            &mut boss_state,
            Some(ClientInput::SubmitRelicChoice(0)),
        )
        .expect("boss relic choice should transition");

        assert!(matches!(next, EngineState::MapNavigation));
        assert_eq!(run_state.act_num, 2);
        assert!(run_state
            .relics
            .iter()
            .any(|relic| relic.id == RelicId::CoffeeDripper));
        assert!(!run_state.pending_boss_act_transition);
    }

    #[test]
    fn boss_relic_choice_defers_act_transition_until_on_equip_selection_resolves() {
        let mut run_state = RunState::new(7, 0, false, "Ironclad");
        let mut boss_state = BossRelicChoiceState::new(vec![RelicId::Astrolabe]);

        let next = handle(
            &mut run_state,
            &mut boss_state,
            Some(ClientInput::SubmitRelicChoice(0)),
        )
        .expect("Astrolabe should open a run-level selection");

        let EngineState::RunPendingChoice(choice) = next else {
            panic!("Astrolabe should interrupt into RunPendingChoice");
        };
        assert_eq!(
            run_state.act_num, 1,
            "Java obtains boss relics before leaving the boss chest room"
        );
        assert!(run_state
            .relics
            .iter()
            .any(|relic| relic.id == RelicId::Astrolabe));
        assert!(run_state.pending_boss_act_transition);

        let request = choice.selection_request(&run_state);
        let mut engine_state = EngineState::RunPendingChoice(choice);
        let mut combat_state = None;
        assert!(crate::engine::run_loop::tick_run(
            &mut engine_state,
            &mut run_state,
            &mut combat_state,
            Some(ClientInput::SubmitSelection(SelectionResolution {
                scope: SelectionScope::Deck,
                selected: request.targets.into_iter().take(3).collect(),
            })),
        ));

        assert!(matches!(engine_state, EngineState::MapNavigation));
        assert_eq!(
            run_state.act_num, 2,
            "act transition should happen after the boss relic's pending selection resolves"
        );
        assert!(!run_state.pending_boss_act_transition);
    }

    #[test]
    fn boss_starter_upgrade_relic_replaces_starter_slot_before_advancing_act() {
        for (player_class, starter, upgrade) in [
            ("Ironclad", RelicId::BurningBlood, RelicId::BlackBlood),
            ("Silent", RelicId::SnakeRing, RelicId::RingOfTheSerpent),
            ("Defect", RelicId::CrackedCore, RelicId::FrozenCore),
            ("Watcher", RelicId::PureWater, RelicId::HolyWater),
        ] {
            let mut run_state = RunState::new(7, 0, false, player_class);
            let mut boss_state = BossRelicChoiceState::new(vec![upgrade]);

            assert_eq!(run_state.relics[0].id, starter);

            let next = handle(
                &mut run_state,
                &mut boss_state,
                Some(ClientInput::SubmitRelicChoice(0)),
            )
            .expect("boss relic choice should transition");

            assert!(matches!(next, EngineState::MapNavigation));
            assert_eq!(run_state.act_num, 2);
            assert_eq!(
                run_state.relics[0].id, upgrade,
                "Java instantObtain(player, 0, true) replaces slot 0 for {upgrade:?}"
            );
            assert!(
                !run_state.relics.iter().any(|relic| relic.id == starter),
                "starter relic should not remain beside its boss upgrade"
            );
        }
    }
}
