use crate::bot::card_facts::{facts as card_facts, CostBand};
use crate::bot::card_structure::structure as card_structure;
use crate::content::cards::{self, CardId, CardType};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct NoncombatCardSignals {
    pub damage_patch_strength: i32,
    pub block_patch_strength: i32,
    pub control_patch_strength: i32,
    pub frontload_patch_strength: i32,
    pub scaling_signal: i32,
    pub filler_attack_risk: i32,
}

pub(crate) fn signals(card_id: CardId) -> NoncombatCardSignals {
    let facts = card_facts(card_id);
    let structure = card_structure(card_id);
    let def = cards::get_card_definition(card_id);

    let mut damage_patch_strength = 0;
    if def.card_type == CardType::Attack {
        damage_patch_strength += (def.base_damage / 2).min(10);
        if facts.aoe || def.is_multi_damage {
            damage_patch_strength += 6;
        }
        if facts.multi_hit || structure.is_multi_attack_payoff() {
            damage_patch_strength += 4;
        }
        if facts.applies_vuln {
            damage_patch_strength += 5;
        }
        if structure.is_strength_payoff() {
            damage_patch_strength += 4;
        }
        damage_patch_strength += match facts.cost_band {
            CostBand::ZeroOne => 2,
            CostBand::Two => 1,
            _ => 0,
        };
    }

    let mut block_patch_strength = (def.base_block / 2).min(10);
    if facts.applies_weak {
        block_patch_strength += 6;
    }
    if facts.combat_heal {
        block_patch_strength += 6;
    }
    if structure.is_block_core() {
        block_patch_strength += 4;
    }
    if structure.is_block_payoff() {
        block_patch_strength += 2;
    }
    if facts.draws_cards && def.base_block > 0 {
        block_patch_strength += 2;
    }

    let mut control_patch_strength = 0;
    if facts.applies_weak {
        control_patch_strength += 8;
    }
    if facts.applies_vuln {
        control_patch_strength += 6;
    }
    if facts.applies_frail {
        control_patch_strength += 4;
    }
    if facts.target_sensitive {
        control_patch_strength += 2;
    }
    if facts.aoe {
        control_patch_strength += 2;
    }

    let mut frontload_patch_strength = damage_patch_strength + control_patch_strength / 2;
    frontload_patch_strength += match facts.cost_band {
        CostBand::ZeroOne | CostBand::Two => 4,
        CostBand::XCost => 2,
        _ => 0,
    };
    if structure.is_setup_piece() || structure.is_engine_piece() {
        frontload_patch_strength -= 4;
    }
    frontload_patch_strength = frontload_patch_strength.max(0);

    let mut scaling_signal = 0;
    if structure.is_strength_enabler() {
        scaling_signal += 6;
    }
    if structure.is_strength_payoff() {
        scaling_signal += 5;
    }
    if structure.is_scaling_piece() || structure.is_setup_piece() {
        scaling_signal += 4;
    }
    if structure.is_engine_piece() || structure.is_exhaust_engine() {
        scaling_signal += 6;
    }
    if structure.is_exhaust_outlet() {
        scaling_signal += 4;
    }
    if structure.is_block_payoff() {
        scaling_signal += 5;
    }
    if facts.draws_cards && structure.is_draw_core() {
        scaling_signal += 2;
    }

    let mut filler_attack_risk = 0;
    if def.card_type == CardType::Attack
        && !facts.aoe
        && !facts.applies_weak
        && !facts.applies_vuln
        && !facts.draws_cards
        && !facts.gains_energy
        && !structure.is_strength_payoff()
        && !structure.is_multi_attack_payoff()
        && !structure.is_block_payoff()
        && !structure.is_vuln_payoff()
    {
        filler_attack_risk += 3;
        if def.base_damage <= 10 {
            filler_attack_risk += 2;
        }
        if facts.produces_status {
            filler_attack_risk += 2;
        }
    }
    if structure.is_strength_payoff() && !structure.is_strength_enabler() && !facts.applies_vuln {
        filler_attack_risk += 1;
    }

    NoncombatCardSignals {
        damage_patch_strength,
        block_patch_strength,
        control_patch_strength,
        frontload_patch_strength,
        scaling_signal,
        filler_attack_risk,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn noncombat_signals_capture_patch_and_scaling_semantics() {
        let shrug = signals(CardId::ShrugItOff);
        let strike = signals(CardId::Strike);
        let shockwave = signals(CardId::Shockwave);
        let corruption = signals(CardId::Corruption);

        assert!(shrug.block_patch_strength > strike.block_patch_strength);
        assert!(shockwave.control_patch_strength > strike.control_patch_strength);
        assert!(corruption.scaling_signal > strike.scaling_signal);
        assert!(strike.filler_attack_risk > corruption.filler_attack_risk);
    }
}
