use crate::content::relics::RelicId;
use crate::state::core::EngineState;
use crate::state::run::RunState;

/// Central router for macro/out-of-combat relic hooks, modeling Java's AbstractRelic lifecycle.
/// Primary hook: on_equip (called when a relic is added to the player's run state).
pub fn on_equip(run_state: &mut RunState, relic_id: RelicId, return_state: EngineState) -> Option<EngineState> {
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

        // === Potion Slots ===
        RelicId::PotionBelt => potion_belt::on_equip(run_state),

        // === Card Upgrades (shuffle-based, no UI needed) ===
        RelicId::Whetstone => whetstone::on_equip(run_state),
        RelicId::WarPaint => war_paint::on_equip(run_state),

        // === TinyHouse ===
        RelicId::TinyHouse => tiny_house::on_equip(run_state),

        // === State-interrupting Relics (return an EngineState override) ===
        // DollysMirror: duplicate a card from deck
        RelicId::DollysMirror => dollys_mirror::on_equip(run_state, return_state),
        // Astrolabe: select 3 cards to Transform + Upgrade
        RelicId::Astrolabe => astrolabe::on_equip(run_state, return_state),
        // EmptyCage: Purge 2 cards
        RelicId::EmptyCage => empty_cage::on_equip(run_state, return_state),
        // Pandora's Box: Transform all Strikes and Defends
        RelicId::PandorasBox => {
            let results = pandoras_box::on_equip(run_state);
            if !results.is_empty() {
                println!("  [Pandora's Box] Transformed {} cards:", results.len());
                for (old, new) in &results {
                    println!("    {} → {}", old, new);
                }
            }
            None
        },
        // CallingBell: Curse of the Bell + 3 Relics
        RelicId::CallingBell => calling_bell::on_equip(run_state, return_state),
        // Orrery: 5 Card Rewards
        RelicId::Orrery => orrery::on_equip(run_state, return_state),

        _ => None,
    }
}
