// action_handlers/powers.rs — Power management domain
//
// Handles: ApplyPower, RemovePower, RemoveAllDebuffs, ApplyStasis,
//          UpdatePowerExtraData, AwakenedRebirthClear, GainEnergy

use crate::action::Action;
use crate::combat::CombatState;
use crate::content::powers::PowerId;

pub fn handle_apply_power(source: usize, target: usize, power_id: PowerId, mut amount: i32, state: &mut CombatState) {
    // C1: Snake Skull → +1 Poison
    if amount > 0 && power_id == PowerId::Poison && state.player.has_relic(crate::content::relics::RelicId::SneckoSkull) {
        if source == 0 && target != 0 {
            amount += 1;
        }
    }

    // U1: Dead/Escaped target guard
    if target == 0 {
        // Player target — always valid
    } else if let Some(m) = state.monsters.iter().find(|m| m.id == target) {
        if m.is_dying || m.current_hp <= 0 {
            return;
        }
    }

    // U3: onApplyPower hooks (Sadistic Nature)
    if source != 0 || target != 0 {
        if let Some(source_powers) = state.power_db.get(&source).cloned() {
            for power in &source_powers {
                let hook_actions = crate::content::powers::resolve_power_on_apply_power(
                    power.power_type, power.amount, power_id, amount, target, source, state
                );
                let hook_actions_4: smallvec::SmallVec<[crate::action::ActionInfo; 4]> = hook_actions.into_iter().collect();
                crate::engine::core::queue_actions(&mut state.action_queue, hook_actions_4);
            }
        }
    }

    // U4: Champion Belt
    let champion_belt_actions = crate::content::relics::hooks::on_apply_power(state, power_id, target);
    crate::engine::core::queue_actions(&mut state.action_queue, champion_belt_actions);

    // U5: Monster re-check after hooks
    if target != 0 {
        if let Some(m) = state.monsters.iter().find(|m| m.id == target) {
            if m.is_dying || m.current_hp <= 0 {
                return;
            }
        }
    }

    // U6+U7: Ginger/Turnip
    if target == 0 {
        amount = crate::content::relics::hooks::on_receive_power_modify(state, power_id, amount);
        if amount == 0 && crate::content::powers::is_debuff(power_id, amount) {
            return;
        }
    }

    // U8: Artifact blocks debuffs
    if crate::content::powers::is_debuff(power_id, amount) {
        let has_artifact = state.power_db.get(&target).map_or(false, |powers| {
            powers.iter().any(|p| p.power_type == PowerId::Artifact)
        });
        if has_artifact {
            if let Some(powers) = state.power_db.get_mut(&target) {
                if let Some(art) = powers.iter_mut().find(|p| p.power_type == PowerId::Artifact) {
                    art.amount -= 1;
                    if art.amount <= 0 {
                        powers.retain(|p| p.power_type != PowerId::Artifact);
                    }
                }
            }
            return;
        }
    }

    // Core power application
    let powers = state.power_db.entry(target).or_insert_with(Vec::new);
    if let Some(existing) = powers.iter_mut().find(|p| p.power_type == power_id) {
        existing.amount += amount;
        if power_id == PowerId::Combust {
            existing.extra_data += 1;
        }

        // Only remove if amount <= 0 AND the power cannot go negative
        let can_go_neg = matches!(power_id, PowerId::Strength | PowerId::Dexterity | PowerId::Focus);
        if existing.amount == 0 && can_go_neg {
            // Java keeps Strength/Dexterity/Focus around even if they hit exactly 0!
        } else if existing.amount <= 0 && !can_go_neg && power_id != PowerId::TimeWarp {
            powers.retain(|p| p.power_type != power_id);
        }
    } else {
        // If they don't have it, we only add it if amount > 0, OR if it's a negative amount of a power that CAN go negative
        let can_go_neg = matches!(power_id, PowerId::Strength | PowerId::Dexterity | PowerId::Focus);
        if amount > 0 || (amount < 0 && can_go_neg) {
            let extra_data = match power_id {
                PowerId::Combust => 1,
                _ => 0,
            };
            powers.push(crate::combat::Power { power_type: power_id, amount, extra_data, just_applied: true });
        }
    }

    // C2: Corruption on-apply hook
    if power_id == PowerId::Corruption {
        crate::content::cards::ironclad::corruption::corruption_on_apply(state);
    }
}

pub fn handle_remove_power(target: usize, power_id: PowerId, state: &mut CombatState) {
    if let Some(powers) = state.power_db.get_mut(&target) {
        powers.retain(|p| p.power_type != power_id);
    }
}

pub fn handle_remove_all_debuffs(target: usize, state: &mut CombatState) {
    if let Some(powers) = state.power_db.get_mut(&target) {
        powers.retain(|p| {
            !crate::content::powers::is_debuff(p.power_type, p.amount)
        });
    }
}

pub fn handle_apply_stasis(target_id: usize, state: &mut CombatState) {
    if state.draw_pile.is_empty() && state.discard_pile.is_empty() {
        return;
    }

    let source_pile_draw = !state.draw_pile.is_empty();
    let source_pile = if source_pile_draw { &state.draw_pile } else { &state.discard_pile };

    let rarities_to_check = [
        crate::content::cards::CardRarity::Rare,
        crate::content::cards::CardRarity::Uncommon,
        crate::content::cards::CardRarity::Common,
    ];

    let mut candidates = Vec::new();
    for expected_rarity in rarities_to_check {
        for (i, card) in source_pile.iter().enumerate() {
            let def = crate::content::cards::get_card_definition(card.id);
            if def.rarity == expected_rarity {
                candidates.push(i);
            }
        }
        if !candidates.is_empty() {
            break;
        }
    }

    if candidates.is_empty() {
        for i in 0..source_pile.len() {
            candidates.push(i);
        }
    }

    let pick_idx = if candidates.len() > 1 {
        let r = state.rng.card_random_rng.random(candidates.len() as i32 - 1) as usize;
        candidates[r]
    } else {
        candidates[0]
    };

    let card = if source_pile_draw {
        state.draw_pile.remove(pick_idx)
    } else {
        state.discard_pile.remove(pick_idx)
    };

    let uuid = card.uuid as i32;
    state.limbo.push(card);

    state.action_queue.push_front(Action::ApplyPower {
        source: target_id,
        target: target_id,
        power_id: PowerId::Stasis,
        amount: uuid,
    });
}

pub fn handle_update_power_extra_data(target: usize, power_id: PowerId, value: i32, state: &mut CombatState) {
    if let Some(powers) = state.power_db.get_mut(&target) {
        if let Some(power) = powers.iter_mut().find(|p| p.power_type == power_id) {
            power.extra_data = value;
        }
    }
}

pub fn handle_awakened_rebirth_clear(target: usize, state: &mut CombatState) {
    if let Some(powers) = state.power_db.get_mut(&target) {
        // Remove Curiosity and Unawakened, keep the rest
        powers.retain(|p| p.power_type != PowerId::Curiosity && p.power_type != PowerId::Unawakened);
    }
}

pub fn handle_gain_energy(amount: i32, state: &mut CombatState) {
    state.energy = (state.energy as i32 + amount).max(0) as u8;
}

pub fn handle_gain_max_hp(amount: i32, state: &mut CombatState) {
    state.player.max_hp += amount;
    state.player.current_hp = (state.player.current_hp + amount).min(state.player.max_hp);
}

pub fn handle_lose_max_hp(target: usize, amount: i32, state: &mut CombatState) {
    if target == 0 {
        state.player.max_hp = (state.player.max_hp - amount).max(1);
        state.player.current_hp = state.player.current_hp.min(state.player.max_hp);
    }
}
