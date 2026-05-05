/// Pandora's Box — Boss Relic
///
/// Java: PandorasBox.onEquip()
///   1. Iterate masterDeck, remove all cards with STARTER_STRIKE or STARTER_DEFEND tags
///   2. Count removed cards
///   3. For each removed card, call `returnTrulyRandomCard()` (uses `cardRandomRng`)
///   4. Call `onPreviewObtainCard` on each new card (egg relics auto-upgrade)
///   5. Show confirmation grid (visual only, no player choice)
///
/// Key difference from `transform_card`:
///   - `transform_card` uses `miscRng` and excludes the original card from the pool
///   - PandorasBox uses `cardRandomRng` and picks from the ENTIRE pool (no exclusion)
use crate::content::cards::{get_card_definition, CardId};
use crate::state::run::RunState;

/// Executes PandorasBox's onEquip logic.
/// Returns a Vec of (old_card_name, new_card_name) pairs for CLI display.
pub fn on_equip(run_state: &mut RunState) -> Vec<(String, String)> {
    // Phase 1: Remove all starter Strikes and Defends, count them
    let mut removed_names = Vec::new();
    let mut to_remove_uuids = Vec::new();
    for card in &run_state.master_deck {
        if crate::content::cards::is_starter_basic(card.id) {
            to_remove_uuids.push(card.uuid);
            let def = get_card_definition(card.id);
            removed_names.push(def.name.to_string());
        }
    }

    for uuid in to_remove_uuids {
        run_state.remove_card_from_deck(uuid);
    }

    let count = removed_names.len();
    if count == 0 {
        return Vec::new();
    }

    // Phase 2: Generate `count` truly random cards using cardRandomRng
    // Java: returnTrulyRandomCard() → srcCommonCardPool + srcUncommonCardPool + srcRareCardPool
    //       → cardRandomRng.random(list.size() - 1)
    let pool: Vec<CardId> = crate::engine::campfire_handler::card_pool_for_class(
        run_state.player_class,
        crate::content::cards::CardRarity::Common,
    )
    .iter()
    .chain(
        crate::engine::campfire_handler::card_pool_for_class(
            run_state.player_class,
            crate::content::cards::CardRarity::Uncommon,
        )
        .iter(),
    )
    .chain(
        crate::engine::campfire_handler::card_pool_for_class(
            run_state.player_class,
            crate::content::cards::CardRarity::Rare,
        )
        .iter(),
    )
    .copied()
    .collect();

    let mut results = Vec::new();

    for idx in 0..count {
        let old_name = removed_names[idx].clone();

        if pool.is_empty() {
            continue;
        }

        // Java uses cardRandomRng (NOT miscRng)
        let pick = run_state
            .rng_pool
            .card_rng
            .random_range(0, pool.len() as i32 - 1) as usize;
        let new_card_id = pool[pick];

        // Java calls onPreviewObtainCard for each relic (egg auto-upgrade etc.)
        // Our add_card_to_deck already handles egg logic, CeramicFish, Omamori, etc.
        run_state.add_card_to_deck(new_card_id);

        let new_def = get_card_definition(new_card_id);
        results.push((old_name, new_def.name.to_string()));
    }

    results
}
