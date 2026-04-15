use super::PotionId;
use crate::action::{Action, ActionInfo, AddTo, DamageInfo, DamageType, NO_SOURCE};
use crate::content::powers::PowerId;
use crate::core::EntityId;
use smallvec::SmallVec;

/// Player entity ID constant
const PLAYER: EntityId = 0;

/// Helper: push a single action to the bottom
fn bottom(actions: &mut SmallVec<[ActionInfo; 4]>, action: Action) {
    actions.push(ActionInfo {
        action,
        insertion_mode: AddTo::Bottom,
    });
}

/// Generates the actions applied when a potion is used in combat.
/// `target_idx`: Some(enemy_entity_id) for targeted/thrown potions, None for self.
/// `potency`: the effective potency (base * SacredBark multiplier).
#[allow(unused)]
pub fn get_potion_actions(
    enemy_count: usize,
    potion: PotionId,
    target_idx: Option<usize>,
    potency: i32,
) -> SmallVec<[ActionInfo; 4]> {
    let mut actions = SmallVec::new();
    let target = target_idx.unwrap_or(0) as EntityId;

    match potion {
        // ────────────── Common (20) ──────────────
        PotionId::FirePotion => {
            // Deal 20 damage to target enemy (thrown). Java: DamageType.THORNS
            bottom(
                &mut actions,
                Action::Damage(DamageInfo {
                    source: PLAYER,
                    target,
                    base: potency,
                    output: potency,
                    damage_type: DamageType::Thorns,
                    is_modified: false,
                }),
            );
        }
        PotionId::ExplosivePotion => {
            // Deal 10 damage to ALL enemies (thrown)
            bottom(
                &mut actions,
                Action::DamageAllEnemies {
                    // Java uses DamageAllEnemiesAction(null, ..., NORMAL), so this
                    // damage should not look like a player-owned normal attack for
                    // hooks such as Curl Up.
                    source: NO_SOURCE,
                    damages: crate::action::repeated_damage_matrix(enemy_count, potency),
                    damage_type: DamageType::Normal,
                    is_modified: false,
                },
            );
        }
        PotionId::PoisonPotion => {
            // Apply 6 Poison to target enemy (thrown)
            bottom(
                &mut actions,
                Action::ApplyPower {
                    source: PLAYER,
                    target,
                    power_id: PowerId::Poison,
                    amount: potency,
                },
            );
        }
        PotionId::WeakenPotion => {
            // Apply 3 Weak to target enemy (thrown)
            bottom(
                &mut actions,
                Action::ApplyPower {
                    source: PLAYER,
                    target,
                    power_id: PowerId::Weak,
                    amount: potency,
                },
            );
        }
        PotionId::FearPotion => {
            // Apply 3 Vulnerable to target enemy (thrown)
            bottom(
                &mut actions,
                Action::ApplyPower {
                    source: PLAYER,
                    target,
                    power_id: PowerId::Vulnerable,
                    amount: potency,
                },
            );
        }
        PotionId::BlockPotion => {
            // Gain 12 Block
            bottom(
                &mut actions,
                Action::GainBlock {
                    target: PLAYER,
                    amount: potency,
                },
            );
        }
        PotionId::BloodPotion => {
            // Heal for potency% of Max HP. Negative sentinel triggers maxHP% calculation in handler.
            bottom(
                &mut actions,
                Action::Heal {
                    target: PLAYER,
                    amount: -(potency),
                },
            );
        }
        PotionId::EnergyPotion => {
            // Gain 2 Energy
            bottom(&mut actions, Action::GainEnergy { amount: potency });
        }
        PotionId::StrengthPotion => {
            // Gain 2 Strength (permanent, unlike Java's Flex which wears off)
            bottom(
                &mut actions,
                Action::ApplyPower {
                    source: PLAYER,
                    target: PLAYER,
                    power_id: PowerId::Strength,
                    amount: potency,
                },
            );
        }
        PotionId::DexterityPotion => {
            // Gain 2 Dexterity (permanent)
            bottom(
                &mut actions,
                Action::ApplyPower {
                    source: PLAYER,
                    target: PLAYER,
                    power_id: PowerId::Dexterity,
                    amount: potency,
                },
            );
        }
        PotionId::SpeedPotion => {
            // Gain 5 temporary Dexterity (lost at end of turn via DexterityDown)
            bottom(
                &mut actions,
                Action::ApplyPower {
                    source: PLAYER,
                    target: PLAYER,
                    power_id: PowerId::Dexterity,
                    amount: potency,
                },
            );
            bottom(
                &mut actions,
                Action::ApplyPower {
                    source: PLAYER,
                    target: PLAYER,
                    power_id: PowerId::DexterityDown,
                    amount: potency,
                },
            );
        }
        PotionId::SteroidPotion => {
            // Gain 5 temporary Strength (lost at end of turn via LoseStrength)
            bottom(
                &mut actions,
                Action::ApplyPower {
                    source: PLAYER,
                    target: PLAYER,
                    power_id: PowerId::Strength,
                    amount: potency,
                },
            );
            bottom(
                &mut actions,
                Action::ApplyPower {
                    source: PLAYER,
                    target: PLAYER,
                    power_id: PowerId::LoseStrength,
                    amount: potency,
                },
            );
        }
        PotionId::SwiftPotion => {
            // Draw 3 cards
            bottom(&mut actions, Action::DrawCards(potency as u32));
        }
        PotionId::FocusPotion => {
            // Gain 2 Focus (Defect)
            bottom(
                &mut actions,
                Action::ApplyPower {
                    source: PLAYER,
                    target: PLAYER,
                    power_id: PowerId::Focus,
                    amount: potency,
                },
            );
        }
        PotionId::AttackPotion => {
            // Java: DiscoveryAction(CardType.ATTACK, potency) — opens choice screen with 3 Attack cards.
            bottom(
                &mut actions,
                Action::SuspendForDiscovery {
                    colorless: false,
                    card_type: Some(crate::content::cards::CardType::Attack),
                    cost_for_turn: Some(0),
                },
            );
        }
        PotionId::SkillPotion => {
            // Java: DiscoveryAction(CardType.SKILL, potency) — opens choice screen with 3 Skill cards.
            bottom(
                &mut actions,
                Action::SuspendForDiscovery {
                    colorless: false,
                    card_type: Some(crate::content::cards::CardType::Skill),
                    cost_for_turn: Some(0),
                },
            );
        }
        PotionId::PowerPotion => {
            // Java: DiscoveryAction(CardType.POWER, potency) — opens choice screen with 3 Power cards.
            // SuspendForDiscovery consumes the correct 3+ cardRandomRng calls.
            // diff_driver auto-resolves the discovery choice by matching Java snapshot.
            bottom(
                &mut actions,
                Action::SuspendForDiscovery {
                    colorless: false,
                    card_type: Some(crate::content::cards::CardType::Power),
                    cost_for_turn: Some(0),
                },
            );
        }
        PotionId::ColorlessPotion => {
            // Java: DiscoveryAction(true, potency) — discover from colorless pool.
            // Colorless discovery uses card_type: None and we handle the colorless flag
            // in the SuspendForDiscovery handler via a separate field.
            bottom(
                &mut actions,
                Action::SuspendForDiscovery {
                    colorless: true,
                    card_type: None,
                    cost_for_turn: Some(0),
                },
            );
        }
        PotionId::BottledMiracle => {
            // Add 2 Miracles to hand (Watcher)
            bottom(
                &mut actions,
                Action::MakeTempCardInHand {
                    card_id: crate::content::cards::CardId::Miracle,
                    amount: potency as u8,
                    upgraded: false,
                },
            );
        }
        PotionId::BlessingOfTheForge => {
            // Upgrade ALL cards in hand
            bottom(&mut actions, Action::UpgradeAllInHand);
        }

        // ────────────── Uncommon (12) ──────────────
        PotionId::AncientPotion => {
            // Gain 1 Artifact
            bottom(
                &mut actions,
                Action::ApplyPower {
                    source: PLAYER,
                    target: PLAYER,
                    power_id: PowerId::Artifact,
                    amount: potency,
                },
            );
        }
        PotionId::RegenPotion => {
            // Gain 5 Regen
            bottom(
                &mut actions,
                Action::ApplyPower {
                    source: PLAYER,
                    target: PLAYER,
                    power_id: PowerId::Regeneration,
                    amount: potency,
                },
            );
        }
        PotionId::EssenceOfSteel => {
            // Gain 4 Plated Armor
            bottom(
                &mut actions,
                Action::ApplyPower {
                    source: PLAYER,
                    target: PLAYER,
                    power_id: PowerId::PlatedArmor,
                    amount: potency,
                },
            );
        }
        PotionId::LiquidBronze => {
            // Gain 3 Thorns
            bottom(
                &mut actions,
                Action::ApplyPower {
                    source: PLAYER,
                    target: PLAYER,
                    power_id: PowerId::Thorns,
                    amount: potency,
                },
            );
        }
        PotionId::DistilledChaosPotion => {
            // Java DistilledChaosAction buffers the current top N cards, then
            // plays them in reverse buffered order. Earlier played draw cards
            // should not consume cards that were already chosen to be played.
            bottom(
                &mut actions,
                Action::PlayTopCardsBuffered {
                    count: potency.max(0) as u8,
                    target: None,
                    exhaust: false,
                },
            );
        }
        PotionId::DuplicationPotion => {
            // This turn, your next card is played twice
            // Java applies DuplicationPower (any card type), NOT DoubleTap (Attack-only)
            bottom(
                &mut actions,
                Action::ApplyPower {
                    source: PLAYER,
                    target: PLAYER,
                    power_id: PowerId::DuplicationPower,
                    amount: potency,
                },
            );
        }
        PotionId::CunningPotion => {
            // Java: creates UPGRADED Shiv, then MakeTempCardInHandAction(shiv, potency)
            bottom(
                &mut actions,
                Action::MakeTempCardInHand {
                    card_id: crate::content::cards::CardId::Shiv,
                    amount: potency as u8,
                    upgraded: true,
                },
            );
        }
        PotionId::PotionOfCapacity => {
            // Gain 2 Orb Slots (Defect)
            bottom(&mut actions, Action::IncreaseMaxOrb(potency as u8));
        }
        PotionId::LiquidMemories => {
            // Choose potency card(s) from discard pile to return to hand (cost 0 this turn).
            // Java: BetterDiscardPileToHandAction(potency, 0)
            bottom(
                &mut actions,
                Action::SuspendForGridSelect {
                    source_pile: crate::state::PileType::Discard,
                    min: 1,
                    max: potency as u8,
                    can_cancel: false,
                    filter: crate::state::GridSelectFilter::Any,
                    reason: crate::state::GridSelectReason::DiscardToHand,
                },
            );
        }
        PotionId::GamblersBrew => {
            // Discard any number of cards from hand, then draw that many.
            // Java: GamblingChipAction — same as Gambling Chip relic.
            bottom(
                &mut actions,
                Action::SuspendForHandSelect {
                    min: 0,
                    max: 99,
                    can_cancel: true,
                    filter: crate::state::HandSelectFilter::Any,
                    reason: crate::state::HandSelectReason::GamblingChip,
                },
            );
        }
        PotionId::Elixir => {
            // Exhaust any number of cards from hand.
            // Java: ExhaustAction(false, true, true) — no specific count, any amount, can cancel.
            bottom(
                &mut actions,
                Action::SuspendForHandSelect {
                    min: 0,
                    max: 99,
                    can_cancel: true,
                    filter: crate::state::HandSelectFilter::Any,
                    reason: crate::state::HandSelectReason::Exhaust,
                },
            );
        }
        PotionId::StancePotion => {
            // Java: ChooseOneAction(ChooseWrath, ChooseCalm) — player picks Wrath or Calm.
            bottom(&mut actions, Action::SuspendForStanceChoice);
        }

        // ────────────── Rare (10) ──────────────
        PotionId::FairyPotion => {
            // Passive death-prevention: heal potency% of Max HP.
            // Negative sentinel triggers maxHP% calculation in handler.
            bottom(
                &mut actions,
                Action::Heal {
                    target: PLAYER,
                    amount: -(potency),
                },
            );
        }
        PotionId::SmokeBomb => {
            // Escape from a non-boss combat (thrown).
            bottom(&mut actions, Action::FleeCombat);
        }
        PotionId::FruitJuice => {
            // Gain 5 Max HP (non-combat potion, but can be used in combat too).
            bottom(&mut actions, Action::GainMaxHp { amount: potency });
        }
        PotionId::EntropicBrew => {
            // Handled natively in Action::UsePotion (action_handlers.rs) to maintain accurate RNG parity.
        }
        PotionId::SneckoOil => {
            // Draw 5 cards, then randomize cost of all cards in hand to 0-3.
            // Java: DrawCardAction(potency) + RandomizeHandCostAction()
            bottom(&mut actions, Action::DrawCards(potency as u32));
            bottom(&mut actions, Action::RandomizeHandCosts);
        }
        PotionId::GhostInAJar => {
            // Gain 1 Intangible
            bottom(
                &mut actions,
                Action::ApplyPower {
                    source: PLAYER,
                    target: PLAYER,
                    power_id: PowerId::IntangiblePlayer,
                    amount: potency,
                },
            );
        }
        PotionId::HeartOfIron => {
            // Gain 6 Metallicize
            bottom(
                &mut actions,
                Action::ApplyPower {
                    source: PLAYER,
                    target: PLAYER,
                    power_id: PowerId::Metallicize,
                    amount: potency,
                },
            );
        }
        PotionId::CultistPotion => {
            // Gain 1 Ritual
            bottom(
                &mut actions,
                Action::ApplyPowerDetailed {
                    source: PLAYER,
                    target: PLAYER,
                    power_id: PowerId::Ritual,
                    amount: potency,
                    instance_id: None,
                    extra_data: Some(crate::content::powers::core::ritual::extra_data(
                        true, false,
                    )),
                },
            );
        }
        PotionId::Ambrosia => {
            // Enter Divinity stance (Watcher).
            // Java: AbstractDungeon.actionManager.addToBottom(new ChangeStanceAction("Divinity"))
            bottom(&mut actions, Action::EnterStance("Divinity".to_string()));
        }
        PotionId::EssenceOfDarkness => {
            for _ in 0..potency.max(0) {
                bottom(&mut actions, Action::ChannelOrb(crate::combat::OrbId::Dark));
            }
        }
    }

    actions
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn distilled_chaos_queues_play_top_card_actions() {
        let actions = get_potion_actions(1, PotionId::DistilledChaosPotion, None, 3);
        assert_eq!(actions.len(), 1);
        assert!(matches!(
            actions[0].action,
            Action::PlayTopCardsBuffered {
                count: 3,
                target: None,
                exhaust: false
            }
        ));
    }

    #[test]
    fn essence_of_darkness_queues_dark_orbs() {
        let actions = get_potion_actions(1, PotionId::EssenceOfDarkness, None, 2);
        assert_eq!(actions.len(), 2);
        assert!(actions
            .iter()
            .all(|a| matches!(a.action, Action::ChannelOrb(crate::combat::OrbId::Dark))));
    }
}
