use crate::content::relics::RelicId;
use crate::state::core::EngineState;
use crate::state::run::RunState;

fn bottle_on_equip(
    run_state: &RunState,
    reason: crate::state::core::RunPendingChoiceReason,
    return_state: EngineState,
) -> Option<EngineState> {
    let has_candidate = run_state.master_deck.iter().any(|card| {
        crate::state::core::run_pending_choice_allows_card_for_run(&reason, card, run_state)
    });
    if !has_candidate {
        return None;
    }

    Some(EngineState::RunPendingChoice(
        crate::state::core::RunPendingChoiceState {
            min_choices: 1,
            max_choices: 1,
            reason,
            source: crate::state::selection::DomainEventSource::Relic(match reason {
                crate::state::core::RunPendingChoiceReason::BottleFlame => {
                    crate::content::relics::RelicId::BottledFlame
                }
                crate::state::core::RunPendingChoiceReason::BottleLightning => {
                    crate::content::relics::RelicId::BottledLightning
                }
                crate::state::core::RunPendingChoiceReason::BottleTornado => {
                    crate::content::relics::RelicId::BottledTornado
                }
                _ => return None,
            }),
            return_state: Box::new(return_state),
        },
    ))
}

/// Central router for macro/out-of-combat relic hooks, modeling Java's AbstractRelic lifecycle.
/// Primary hook: on_equip (called when a relic is added to the player's run state).
pub fn on_equip(
    run_state: &mut RunState,
    relic_id: RelicId,
    return_state: EngineState,
) -> Option<EngineState> {
    // We will route specific on_equip logic to the individual relic modules.
    use crate::content::relics::*;
    match relic_id {
        // === HP Gain ===
        RelicId::Strawberry => strawberry::on_equip(run_state),
        RelicId::Pear => pear::on_equip(run_state),
        RelicId::Mango => mango::on_equip(run_state),
        RelicId::Waffle => waffle::on_equip(run_state),

        // === Gold ===
        RelicId::OldCoin => old_coin::on_equip(run_state),

        // === Reward-screen relics ===
        RelicId::Cauldron => cauldron::on_equip(run_state, return_state),

        // === Potion Slots ===
        RelicId::PotionBelt => potion_belt::on_equip(run_state),

        // === Card Upgrades (shuffle-based, no UI needed) ===
        RelicId::Whetstone => whetstone::on_equip(run_state),
        RelicId::WarPaint => war_paint::on_equip(run_state),

        // === TinyHouse ===
        RelicId::TinyHouse => tiny_house::on_equip(run_state),

        // === State-interrupting Relics (return an EngineState override) ===
        RelicId::BottledFlame => bottle_on_equip(
            run_state,
            crate::state::core::RunPendingChoiceReason::BottleFlame,
            return_state,
        ),
        RelicId::BottledLightning => bottle_on_equip(
            run_state,
            crate::state::core::RunPendingChoiceReason::BottleLightning,
            return_state,
        ),
        RelicId::BottledTornado => bottle_on_equip(
            run_state,
            crate::state::core::RunPendingChoiceReason::BottleTornado,
            return_state,
        ),
        // DollysMirror: duplicate a card from deck
        RelicId::DollysMirror => dollys_mirror::on_equip(run_state, return_state),
        // Astrolabe: select 3 cards to Transform + Upgrade
        RelicId::Astrolabe => astrolabe::on_equip(run_state, return_state),
        // EmptyCage: Purge 2 cards
        RelicId::EmptyCage => empty_cage::on_equip(run_state, return_state),
        // Pandora's Box: Transform all Strikes and Defends
        RelicId::PandorasBox => {
            pandoras_box::on_equip(run_state);
            None
        }
        // CallingBell: Curse of the Bell + 3 Relics
        RelicId::CallingBell => calling_bell::on_equip(run_state, return_state),
        // Orrery: 5 Card Rewards
        RelicId::Orrery => orrery::on_equip(run_state, return_state),
        RelicId::Necronomicon => {
            necronomicon::on_equip(run_state);
            None
        }
        RelicId::DuVuDoll => du_vu_doll::on_equip(run_state),

        _ => None,
    }
}

pub fn on_unequip(
    run_state: &mut RunState,
    relic_id: RelicId,
    source: crate::state::selection::DomainEventSource,
) {
    use crate::content::relics::*;
    match relic_id {
        RelicId::Necronomicon => necronomicon::on_unequip(run_state, source),
        _ => {}
    }
}
