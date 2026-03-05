use super::*;
use super::commands::apply_command;
use crate::schema::CardCommand;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::CardCommand;
    use crate::core::stances::Stance;
    
    #[test]
    fn test_deal_damage_command() {
        let mut state = GameState::new_test(42);
        let cmd = CardCommand::DealDamage { base: 8, upgrade: 10, times: None, times_upgrade: None, scaling: None };
        
        let result = apply_command(&mut state, &cmd, false, None, None);
        
        assert!(matches!(result, CommandResult::DamageDealt { amount: 8, killed: false, .. }));
        assert_eq!(state.enemies[0].hp, 42); // 50 - 8
    }
    
    #[test]
    fn test_gain_block_command() {
        let mut state = GameState::new_test(42);
        let cmd = CardCommand::GainBlock { base: 5, upgrade: 8 };
        
        apply_command(&mut state, &cmd, false, None, None);
        assert_eq!(state.player.block, 5);
        
        apply_command(&mut state, &cmd, true, None, None);
        assert_eq!(state.player.block, 13); // 5 + 8
    }
    
    #[test]
    fn test_vigor_buff() {
        let mut state = GameState::new_test(42);
        
        // Apply Vigor
        let buff_cmd = CardCommand::ApplyBuff {
            buff: "Vigor".to_string(),
            amount: Some(serde_json::json!(5)),
            upgrade_amount: None,
            target: None,
        };
        apply_command(&mut state, &buff_cmd, false, None, None);
        assert_eq!(state.player.vigor(), 5);
        
        // Deal damage - should consume Vigor
        let attack_cmd = CardCommand::DealDamage { base: 6, upgrade: 8, times: None, times_upgrade: None, scaling: None };
        let result = apply_command(&mut state, &attack_cmd, false, None, None);
        
        // 6 base + 5 vigor = 11 damage
        assert!(matches!(result, CommandResult::DamageDealt { amount: 11, .. }));
        assert_eq!(state.player.vigor(), 0); // Vigor consumed
        assert_eq!(state.enemies[0].hp, 39); // 50 - 11
    }
    
    #[test]
    fn test_fatal_condition() {
        let mut state = GameState::new_test(42);
        state.enemies[0].hp = 5; // Low HP enemy
        
        // Deal lethal damage
        let attack_cmd = CardCommand::DealDamage { base: 10, upgrade: 10, times: None, times_upgrade: None, scaling: None };
        apply_command(&mut state, &attack_cmd, false, None, None);
        
        assert!(state.was_last_attack_fatal());
        
        // Test conditional with Fatal
        let cond_cmd = CardCommand::Conditional {
            condition: Some(serde_json::json!({"type": "Fatal"})),
            then_do: Some(vec![serde_json::json!({"type": "GainMaxHP", "params": {"amount": 3}})]),
            else_do: None,
        };
        let result = apply_command(&mut state, &cond_cmd, false, None, None);
        
        assert!(matches!(result, CommandResult::ConditionalExecuted { condition_met: true }));
    }
    
    #[test]
    fn test_intangible() {
        let mut state = GameState::new_test(42);
        
        // Apply Intangible to player
        state.player.apply_temp_buff("Intangible", 1);
        
        // Take massive damage - should only be 1
        let actual = state.player.take_damage(999);
        assert_eq!(actual, 1);
        assert_eq!(state.player.current_hp, 79); // 80 - 1
    }

    // ================================================================
    // Damage Pipeline Tests — verified against Java source
    // ================================================================

    #[test]
    fn test_damage_pipeline_basic() {
        // No modifiers: base = 6 → 6
        let attacker = crate::powers::PowerSet::new();
        let defender = crate::powers::PowerSet::new();
        let dmg = calculate_card_damage(6, &attacker, &defender, Stance::Neutral, Default::default());
        assert_eq!(dmg, 6);
    }

    #[test]
    fn test_damage_pipeline_strength() {
        // Base 6 + Strength 3 = 9
        let mut attacker = crate::powers::PowerSet::new();
        attacker.apply("Strength", 3, None);
        let defender = crate::powers::PowerSet::new();
        let dmg = calculate_card_damage(6, &attacker, &defender, Stance::Neutral, Default::default());
        assert_eq!(dmg, 9);
    }

    #[test]
    fn test_damage_pipeline_weak() {
        // Base 8, Weak: 8 * 0.75 = 6.0 → 6
        let mut attacker = crate::powers::PowerSet::new();
        attacker.apply("Weak", 1, None);
        let defender = crate::powers::PowerSet::new();
        let dmg = calculate_card_damage(8, &attacker, &defender, Stance::Neutral, Default::default());
        assert_eq!(dmg, 6);
    }

    #[test]
    fn test_damage_pipeline_vulnerable() {
        // Base 6, Vulnerable: 6 * 1.5 = 9.0 → 9
        let attacker = crate::powers::PowerSet::new();
        let mut defender = crate::powers::PowerSet::new();
        defender.apply("Vulnerable", 1, None);
        let dmg = calculate_card_damage(6, &attacker, &defender, Stance::Neutral, Default::default());
        assert_eq!(dmg, 9);
    }

    #[test]
    fn test_damage_pipeline_strength_weak_vulnerable() {
        // Base 6 + Str 3 = 9, Weak: 9*0.75=6.75, Vuln: 6.75*1.5=10.125 → floor=10
        let mut attacker = crate::powers::PowerSet::new();
        attacker.apply("Strength", 3, None);
        attacker.apply("Weak", 1, None);
        let mut defender = crate::powers::PowerSet::new();
        defender.apply("Vulnerable", 1, None);
        let dmg = calculate_card_damage(6, &attacker, &defender, Stance::Neutral, Default::default());
        assert_eq!(dmg, 10);
    }

    #[test]
    fn test_damage_pipeline_negative_strength() {
        // Base 6 + Str(-8) = -2 → clamped to 0
        // Use force_set because Strength can be negative in-game
        let mut attacker = crate::powers::PowerSet::new();
        attacker.force_set("Strength", -8);
        let defender = crate::powers::PowerSet::new();
        let dmg = calculate_card_damage(6, &attacker, &defender, Stance::Neutral, Default::default());
        assert_eq!(dmg, 0);
    }

    #[test]
    fn test_damage_pipeline_rounding() {
        // Base 5 + Str 2 = 7, Weak: 7*0.75=5.25 → floor=5
        let mut attacker = crate::powers::PowerSet::new();
        attacker.apply("Strength", 2, None);
        attacker.apply("Weak", 1, None);
        let defender = crate::powers::PowerSet::new();
        let dmg = calculate_card_damage(5, &attacker, &defender, Stance::Neutral, Default::default());
        assert_eq!(dmg, 5);
    }
}
