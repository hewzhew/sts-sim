use crate::content::relics::{RelicId, RelicState};
use crate::state::core::{CampfireChoice, ClientInput, EngineState};
use crate::state::run::RunState;
use crate::state::selection::DomainEventSource;

/// Campfire (Rest Site) handler.
///
/// Java behaviour reference:
///   1. On room entry → `onEnterRestRoom()` for all relics (AncientTeaSet, etc.) — done in run_loop.rs
///   2. Button init: Rest (unless CoffeeDripper), Smith (unless FusionHammer or no upgradable cards),
///      then relic options (Lift/Dig/Toke), then Recall (Act 3+ & Ruby Key missing).
///   3. Execution → per-option logic + relic callbacks (`onRest`, `onSmith`).
pub fn handle(
    engine_state: &mut EngineState,
    run_state: &mut RunState,
    input: Option<ClientInput>,
) -> bool {
    if let Some(ClientInput::CampfireOption(choice)) = input {
        match choice {
            CampfireChoice::Rest => {
                // Java: Asc 14 = 25%, else 30%. (int)(maxHP * multiplier) — truncation.
                let heal_pct = if run_state.ascension_level >= 14 {
                    0.25f32
                } else {
                    0.3f32
                };
                let mut heal = (run_state.max_hp as f32 * heal_pct) as i32;

                // Regal Pillow: flat +15 to rest heal
                if run_state
                    .relics
                    .iter()
                    .any(|r| r.id == RelicId::RegalPillow)
                {
                    heal += 15;
                }

                // MarkOfTheBloom: blocks ALL healing (Java: onPlayerHeal → return 0)
                if run_state
                    .relics
                    .iter()
                    .any(|r| r.id == RelicId::MarkOfTheBloom)
                {
                    heal = 0;
                }

                run_state.current_hp = (run_state.current_hp + heal).min(run_state.max_hp);

                // --- onRest() relic callbacks ---
                // DreamCatcher: after resting, generate a card reward screen
                if run_state
                    .relics
                    .iter()
                    .any(|r| r.id == RelicId::DreamCatcher)
                {
                    let cards =
                        crate::rewards::generator::generate_card_reward(run_state, 3, false);
                    let mut reward_state = crate::rewards::state::RewardState::new();
                    reward_state
                        .items
                        .push(crate::rewards::state::RewardItem::Card { cards });
                    *engine_state = EngineState::RewardScreen(reward_state);
                    return true;
                }

                *engine_state = EngineState::MapNavigation;
            }

            CampfireChoice::Smith(idx) => {
                // Java: SmithOption → card upgrade on master_deck
                if idx < run_state.master_deck.len() {
                    let uuid = run_state.master_deck[idx].uuid;
                    run_state.upgrade_card_with_source(uuid, DomainEventSource::CampfireSmith);
                }
                *engine_state = EngineState::MapNavigation;
            }

            CampfireChoice::Dig => {
                // Shovel: Java → Dig grants relic via reward screen (AbstractRoom.addRelicToRewards)
                let id = run_state.random_relic();
                let mut reward_state = crate::rewards::state::RewardState::new();
                reward_state
                    .items
                    .push(crate::rewards::state::RewardItem::Relic { relic_id: id });
                *engine_state = EngineState::RewardScreen(reward_state);
            }

            CampfireChoice::Lift => {
                // Girya: increment counter (capped at 3). Strength applied at battleStart.
                for relic in run_state.relics.iter_mut() {
                    if relic.id == RelicId::Girya {
                        relic.counter = (relic.counter + 1).min(3);
                    }
                }
                *engine_state = EngineState::MapNavigation;
            }

            CampfireChoice::Toke(idx) => {
                // Peace Pipe: remove a card from master_deck.
                // Java: Toke filters out bottled cards — cards attached to
                // BottledFlame / BottledLightning / BottledTornado are not removable.
                // Currently CombatCard lacks bottled flags; once added, filter here.
                if idx < run_state.master_deck.len() {
                    // Guard: skip bottled cards (when tracking is implemented)
                    let card = &run_state.master_deck[idx];
                    if !is_card_bottled(card, &run_state.relics) {
                        let uuid = card.uuid;
                        run_state.remove_card_from_deck_with_source(
                            uuid,
                            DomainEventSource::CampfireToke,
                        );
                    }
                }
                *engine_state = EngineState::MapNavigation;
            }

            CampfireChoice::Recall => {
                // Ruby Key: obtained by choosing Recall instead of Resting.
                run_state.keys[0] = true; // keys[0] = Ruby Key
                *engine_state = EngineState::MapNavigation;
            }
        }
    }
    true
}

/// Check if a card is bottled (attached to BottledFlame/Lightning/Tornado).
/// Java stores the card UUID in the relic's `misc` field; our RelicState uses `amount` for this.
/// For now: RelicState.amount stores the bottled card's UUID.
/// A value of 0 means no card is bottled (since UUIDs start at 1).
fn is_card_bottled(
    card: &crate::runtime::combat::CombatCard,
    relics: &[RelicState],
) -> bool {
    if card.uuid == 0 {
        return false;
    } // UUID 0 = not a real bottled target
    for relic in relics {
        match relic.id {
            RelicId::BottledFlame | RelicId::BottledLightning | RelicId::BottledTornado => {
                if relic.amount == card.uuid as i32 {
                    return true;
                }
            }
            _ => {}
        }
    }
    false
}

/// Dispatch card pool by player class and rarity.
pub fn card_pool_for_class(
    player_class: &str,
    rarity: crate::content::cards::CardRarity,
) -> &'static [crate::content::cards::CardId] {
    match player_class {
        "Ironclad" => crate::content::cards::ironclad_pool_for_rarity(rarity),
        "Silent" => crate::content::cards::silent_pool_for_rarity(rarity),
        "Defect" => crate::content::cards::defect_pool_for_rarity(rarity),
        "Watcher" => crate::content::cards::watcher_pool_for_rarity(rarity),
        _ => crate::content::cards::ironclad_pool_for_rarity(rarity), // fallback
    }
}

pub fn nonempty_card_pool_for_class(
    player_class: &str,
    rarity: crate::content::cards::CardRarity,
) -> &'static [crate::content::cards::CardId] {
    use crate::content::cards::CardRarity;

    let fallbacks = match rarity {
        CardRarity::Rare => [CardRarity::Rare, CardRarity::Uncommon, CardRarity::Common],
        CardRarity::Uncommon => [CardRarity::Uncommon, CardRarity::Common, CardRarity::Rare],
        CardRarity::Common => [CardRarity::Common, CardRarity::Uncommon, CardRarity::Rare],
        _ => [CardRarity::Common, CardRarity::Uncommon, CardRarity::Rare],
    };

    for candidate_rarity in fallbacks {
        let pool = card_pool_for_class(player_class, candidate_rarity);
        if !pool.is_empty() {
            return pool;
        }
    }

    &[]
}

/// Returns the list of available campfire options for the current run state.
/// Java: CampfireUI.initializeButtons() order:
///   1. Rest (unless CoffeeDripper)
///   2. Smith (unless FusionHammer or no upgradable cards)
///   3. Relic options: Lift (Girya counter < 3), Dig (Shovel owned), Toke (PeacePipe owned)
///   4. Recall (Act 3+ AND Ruby Key missing)
pub fn get_available_options(run_state: &RunState) -> Vec<CampfireChoice> {
    let mut options = Vec::new();

    // 1. Rest — vetoed by CoffeeDripper
    let has_coffee_dripper = run_state
        .relics
        .iter()
        .any(|r| r.id == RelicId::CoffeeDripper);
    if !has_coffee_dripper {
        options.push(CampfireChoice::Rest);
    }

    // 2. Smith — vetoed by FusionHammer or no upgradable cards
    let has_fusion_hammer = run_state
        .relics
        .iter()
        .any(|r| r.id == RelicId::FusionHammer);
    if !has_fusion_hammer {
        // SearingBlow can always upgrade; other cards can upgrade once (upgrades == 0)
        let has_upgradable = run_state
            .master_deck
            .iter()
            .any(|c| c.id == crate::content::cards::CardId::SearingBlow || c.upgrades == 0);
        if has_upgradable {
            options.push(CampfireChoice::Smith(0)); // Index 0 is placeholder; AI picks actual card
        }
    }

    // 3. Relic campfire options (in relic acquisition order)
    for relic in &run_state.relics {
        match relic.id {
            RelicId::Girya if relic.counter < 3 => {
                options.push(CampfireChoice::Lift);
            }
            RelicId::Shovel => {
                options.push(CampfireChoice::Dig);
            }
            RelicId::PeacePipe => {
                // Only offer Toke if there are non-bottled removable cards
                let has_removable = run_state
                    .master_deck
                    .iter()
                    .any(|c| !is_card_bottled(c, &run_state.relics));
                if has_removable {
                    options.push(CampfireChoice::Toke(0)); // Index 0 placeholder
                }
            }
            _ => {}
        }
    }

    // 4. Recall — Java: CampfireUI:91 shows if isFinalActAvailable && !hasRubyKey
    if run_state.is_final_act_available && !run_state.keys[0] {
        options.push(CampfireChoice::Recall);
    }

    options
}
