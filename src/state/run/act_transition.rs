use super::*;

impl RunState {
    pub fn should_start_act3_double_boss(&self) -> bool {
        self.act_num == 3 && self.ascension_level >= 20 && self.boss_list.len() == 2
    }

    pub fn enter_final_act(&mut self) {
        self.act_num = 4;
        self.pending_boss_reward = false;
        self.pending_boss_act_transition = false;
        self.apply_dungeon_transition_setup_effects();
        self.map = crate::state::map::state::MapState::new(
            crate::state::map::generator::generate_ending_map(),
        );
        self.map.has_emerald_key = self.keys[2];
        self.init_encounter_lists();
        self.init_boss_list();
        self.event_generator.initialize_event_pools(self.act_num);
        self.event_generator.reset_probabilities();
        self.card_upgraded_chance = if self.ascension_level >= 12 {
            0.25
        } else {
            0.5
        };
    }

    /// Advance to the next act after boss defeat.
    /// Regenerates map, encounter lists, event pools, and boss list for the new act.
    /// Java flow: VictoryRoom → Proceed → next Dungeon constructor.
    pub fn advance_act(&mut self) {
        self.act_num += 1;
        self.pending_boss_reward = false;
        self.pending_boss_act_transition = false;

        self.apply_dungeon_transition_setup_effects();

        // Generate new map for the next act
        // Generate map for the new act; returns consumed mapRng for emerald placement.
        let (mut new_map, mut map_rng) = crate::state::map::generator::generate_map_for_act(
            self.seed,
            self.act_num,
            self.ascension_level == 0,
        );

        // Mark emerald elite on new map if Act 4 is enabled and key not yet obtained.
        // Java: setEmeraldElite() reuses the consumed mapRng, not a fresh one.
        if self.is_final_act_available && !self.keys[2] {
            crate::state::map::generator::set_emerald_elite(&mut new_map, &mut map_rng);
        }

        self.map = crate::state::map::state::MapState::new(new_map);
        self.map.has_emerald_key = self.keys[2];

        // Regenerate encounter lists for the new act
        self.init_encounter_lists();
        self.init_boss_list();

        // Reinitialize event pools for the new act
        self.event_generator.initialize_event_pools(self.act_num);

        // Reset event room-type probabilities (Java: AbstractDungeon.resetProbabilities)
        self.event_generator.reset_probabilities();

        // Update card_upgraded_chance per act (Java: initializeLevelSpecificChances)
        // Asc 12: halves the upgrade chance per act
        self.card_upgraded_chance = match self.act_num {
            1 => 0.0, // Exordium: always 0.0
            2 => {
                if self.ascension_level >= 12 {
                    0.125
                } else {
                    0.25
                }
            } // TheCity
            3 => {
                if self.ascension_level >= 12 {
                    0.25
                } else {
                    0.5
                }
            } // TheBeyond
            _ => {
                if self.ascension_level >= 12 {
                    0.25
                } else {
                    0.5
                }
            } // TheEnding
        };
    }

    pub fn complete_pending_boss_act_transition(&mut self) -> bool {
        if self.pending_boss_act_transition {
            self.advance_act();
            true
        } else {
            false
        }
    }

    pub(super) fn apply_dungeon_transition_setup_effects(&mut self) {
        self.align_card_rng_counter_for_dungeon_transition();
        self.potion_drop_chance_mod = 0;
        self.heal_for_dungeon_transition();
    }

    pub(super) fn heal_for_dungeon_transition(&mut self) {
        let missing = self.max_hp - self.current_hp;
        let heal_amount = if self.ascension_level >= 5 {
            (missing as f32 * 0.75).round() as i32
        } else {
            self.max_hp
        };
        self.current_hp = (self.current_hp + heal_amount).min(self.max_hp);
    }

    pub(super) fn align_card_rng_counter_for_dungeon_transition(&mut self) {
        let target = match self.rng_pool.card_rng.counter {
            1..=249 => Some(250),
            251..=499 => Some(500),
            501..=749 => Some(750),
            _ => None,
        };
        if let Some(target) = target {
            self.rng_pool.card_rng.advance_counter_to(target);
        }
    }
}
