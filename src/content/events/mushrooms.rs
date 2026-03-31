use crate::content::cards::CardId;
use crate::content::relics::RelicId;
use crate::state::core::EngineState;
use crate::state::events::{EventChoiceMeta, EventState};
use crate::state::run::RunState;

pub fn get_choices(run_state: &RunState, event_state: &EventState) -> Vec<EventChoiceMeta> {
    match event_state.current_screen {
        0 => {
            let heal_amt = (run_state.max_hp as f32 * 0.25) as i32;
            vec![
                EventChoiceMeta::new("[Stomp] Fight the mushrooms!"),
                EventChoiceMeta::new(format!(
                    "[Eat] Heal {} HP. Become Cursed - Parasite.", heal_amt
                )),
            ]
        },
        1 => {
            // Post-heal or post-combat-setup leave screen
            vec![EventChoiceMeta::new("[Leave]")]
        },
        _ => vec![EventChoiceMeta::new("[Leave]")],
    }
}

pub fn handle_choice(engine_state: &mut EngineState, run_state: &mut RunState, choice_idx: usize) {
    let mut event_state = run_state.event_state.take().unwrap();

    match event_state.current_screen {
        0 => {
            match choice_idx {
                0 => {
                    // Fight the mushrooms
                    // Pre-load rewards: gold + OddMushroom relic
                    let gold = run_state.rng_pool.misc_rng.random_range(20, 30);
                    let mut rewards = crate::state::reward::RewardState::new();
                    rewards.items.push(crate::state::reward::RewardItem::Gold { amount: gold });

                    if run_state.relics.iter().any(|r| r.id == RelicId::OddMushroom) {
                        rewards.items.push(crate::state::reward::RewardItem::Relic { relic_id: RelicId::Circlet });
                    } else {
                        rewards.items.push(crate::state::reward::RewardItem::Relic { relic_id: RelicId::OddMushroom });
                    }

                    event_state.current_screen = 1;
                    event_state.completed = true;
                    run_state.event_state = Some(event_state);

                    *engine_state = EngineState::EventCombat(crate::state::core::EventCombatState {
                        rewards,
                        reward_allowed: true,
                        no_cards_in_rewards: false,
                        post_combat_return: crate::state::core::PostCombatReturn::MapNavigation,
                        encounter_key: "3 Fungi Beasts",
                    });
                    return;
                },
                _ => {
                    // Eat: Heal 25% maxHP + Parasite curse
                    let heal_amt = (run_state.max_hp as f32 * 0.25) as i32;
                    run_state.current_hp = (run_state.current_hp + heal_amt).min(run_state.max_hp);
                    run_state.add_card_to_deck(CardId::Parasite);
                    event_state.current_screen = 1;
                },
            }
        },
        _ => {
            event_state.completed = true;
        }
    }

    run_state.event_state = Some(event_state);
}
