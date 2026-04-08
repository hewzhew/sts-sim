use crate::content::relics::{RelicId, RelicState};
use crate::state::core::EngineState;
use crate::state::events::{EventChoiceMeta, EventState};
use crate::state::run::RunState;

pub fn get_choices(run_state: &RunState, event_state: &EventState) -> Vec<EventChoiceMeta> {
    match event_state.current_screen {
        0 => vec![EventChoiceMeta::new("[Proceed]")],
        1 => {
            let gold_reward = if run_state.ascension_level >= 15 {
                50
            } else {
                75
            };
            let damage = (run_state.max_hp / 10).max(1);
            vec![
                EventChoiceMeta::new(format!(
                    "[Touch] Lose {} HP. Gain {} Gold.",
                    damage, gold_reward
                )),
                EventChoiceMeta::new("[Trade] Obtain a face Relic."),
                EventChoiceMeta::new("[Leave]"),
            ]
        }
        _ => vec![EventChoiceMeta::new("[Leave]")],
    }
}

pub fn handle_choice(_engine_state: &mut EngineState, run_state: &mut RunState, choice_idx: usize) {
    let mut event_state = run_state.event_state.take().unwrap();

    match event_state.current_screen {
        0 => {
            event_state.current_screen = 1;
        }
        1 => {
            match choice_idx {
                0 => {
                    // Touch: damage + gold
                    // Java: DamageInfo(null, damage) — DEFAULT damage type (not HP_LOSS)
                    // DEFAULT damage can be reduced by Tungsten Rod (-1)
                    let gold_reward = if run_state.ascension_level >= 15 {
                        50
                    } else {
                        75
                    };
                    let mut damage = (run_state.max_hp / 10).max(1);
                    // Apply Tungsten Rod if present (reduces non-HP_LOSS damage by 1)
                    if run_state
                        .relics
                        .iter()
                        .any(|r| r.id == RelicId::TungstenRod)
                    {
                        damage = (damage - 1).max(0);
                    }
                    run_state.current_hp = (run_state.current_hp - damage).max(0);
                    run_state.gold += gold_reward;
                    event_state.current_screen = 2;
                }
                1 => {
                    // Trade: get a face relic
                    // Java: Collections.shuffle(ids, new Random(miscRng.randomLong()))
                    let face_relics = [
                        RelicId::CultistMask,
                        RelicId::FaceOfCleric,
                        RelicId::GremlinMask,
                        RelicId::NlothsMask,
                        RelicId::SsserpentHead,
                    ];
                    let mut available: Vec<RelicId> = face_relics
                        .iter()
                        .copied()
                        .filter(|r| !run_state.relics.iter().any(|owned| owned.id == *r))
                        .collect();

                    let relic_id = if available.is_empty() {
                        // Consume randomLong for seed parity even with no available relics
                        let _seed = run_state.rng_pool.misc_rng.random_long();
                        RelicId::Circlet
                    } else {
                        crate::rng::shuffle_with_random_long(
                            &mut available,
                            &mut run_state.rng_pool.misc_rng,
                        );
                        available[0]
                    };
                    run_state.relics.push(RelicState::new(relic_id));
                    event_state.current_screen = 2;
                }
                _ => {
                    event_state.completed = true;
                }
            }
        }
        _ => {
            event_state.completed = true;
        }
    }

    run_state.event_state = Some(event_state);
}
