pub mod cleric;
pub mod golden_idol;
pub mod golden_shrine;
pub mod living_wall;
pub mod vampires;

pub(crate) fn obtain_event_card(
    run_state: &mut crate::state::run::RunState,
    event_id: crate::state::events::EventId,
    card_id: crate::content::cards::CardId,
) -> bool {
    run_state.add_card_to_deck_with_upgrades_from(
        card_id,
        0,
        crate::state::selection::DomainEventSource::Event(event_id),
    )
}

/// Applies Java `AbstractDungeon.player.damage(DamageInfo(null, amount, HP_LOSS))`
/// from event code.
///
/// HP_LOSS bypasses block and owner-based attack callbacks, but
/// `AbstractPlayer.damage` still runs player relic `onLoseHpLast`, which means
/// Tungsten Rod reduces the loss by 1.
pub(crate) fn apply_player_hp_loss_damage(
    run_state: &mut crate::state::run::RunState,
    amount: i32,
    source: crate::state::selection::DomainEventSource,
) -> i32 {
    let mut damage = amount.max(0);
    if damage > 0
        && run_state
            .relics
            .iter()
            .any(|r| r.id == crate::content::relics::RelicId::TungstenRod)
    {
        damage -= 1;
    }
    run_state.change_hp_with_source(-damage, source)
}

// Phase 1: Exordium Events
pub mod big_fish;
pub mod golden_wing;
pub mod goop_puddle;
pub mod scrap_ooze;
pub mod shining_light;
pub mod sssserpent;

// Phase 2: Shrine Events
pub mod duplicator;
pub mod lab;
pub mod purification_shrine;
pub mod transmogrifier;
pub mod upgrade_shrine;

// Phase 4: Combat Events
pub mod colosseum;
pub mod masked_bandits;
pub mod mushrooms;
pub mod mysterious_sphere;

// Phase 5: Act 2 Events
pub mod addict;
pub mod forgotten_altar;
pub mod ghosts;
pub mod knowing_skull;
pub mod mausoleum;
pub mod nest;

// Phase 6: Act 3 Events
pub mod falling;
pub mod moai_head;
pub mod tomb_red_mask;
pub mod winding_halls;

// Phase 7 Tier 1: Simple Events
pub mod cursed_tome;
pub mod face_trader;
pub mod fountain;
pub mod nloth;
pub mod woman_in_blue;

// Phase 7 Tier 2: Grid-Select Events
pub mod back_to_basics;
pub mod beggar;
pub mod bonfire_spirits;
pub mod designer;
pub mod the_library;

// Phase 7 Tier 3: Final Batch
pub mod dead_adventurer;
pub mod match_and_keep;
pub mod mind_bloom;
pub mod note_for_yourself;
pub mod sensory_stone;
pub mod we_meet_again;

// Phase 8: Previously Missing Events
pub mod accursed_blacksmith;
pub mod bonfire_elementals;
pub mod drug_dealer;
pub mod gremlin_wheel;
pub mod the_joust;

// Neow event
pub mod neow;
