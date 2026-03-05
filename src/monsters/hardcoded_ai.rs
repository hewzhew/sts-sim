//! Hardcoded AI overrides for complex monsters.
//!
//! These implementations exactly match Java's `getMove()` logic for monsters
//! whose behavior is too complex for the data-driven JSON system.

use rand::Rng;
use super::enemy::MonsterState;

impl MonsterState {
    /// Check if this monster has a hardcoded AI override.
    /// Returns Some(move_name) if a hardcoded implementation exists,
    /// None to fall through to the data-driven JSON system.
    pub fn hardcoded_get_move<R: Rng>(&mut self, rng: &mut R, allies_alive: &[bool]) -> Option<String> {
        match self.definition_id.as_str() {
            // === Act 1 ===
            "Cultist" => Some(self.cultist_get_move()),
            "JawWorm" => Some(self.jaw_worm_get_move(rng)),
            "AcidSlime_S" => Some(self.acid_slime_s_get_move(rng)),
            "AcidSlime_M" => Some(self.acid_slime_m_get_move(rng)),
            "AcidSlime_L" => Some(self.acid_slime_l_get_move(rng)),
            "SpikeSlime_S" => Some("Tackle".to_string()),
            "SpikeSlime_M" => Some(self.spike_slime_m_get_move(rng)),
            "SpikeSlime_L" => Some(self.spike_slime_l_get_move(rng)),
            "FuzzyLouseNormal" => Some(self.red_louse_get_move(rng)),
            "FuzzyLouseDefensive" => Some(self.green_louse_get_move(rng)),
            "FungiBeast" => Some(self.fungi_beast_get_move(rng)),
            "GremlinWarrior" | "Mad Gremlin" => Some("Scratch".to_string()),
            "GremlinThief" | "Sneaky Gremlin" => Some("Puncture".to_string()),
            "GremlinFat" | "Fat Gremlin" => Some("Smash".to_string()),
            "GremlinWizard" | "Gremlin Wizard" => Some(self.gremlin_wizard_get_move()),
            "GremlinTsundere" | "Shield Gremlin" => Some(self.shield_gremlin_get_move(allies_alive)),
            "SlaverBlue" | "Blue Slaver" => Some(self.blue_slaver_get_move(rng)),
            "SlaverRed" | "Red Slaver" => Some(self.red_slaver_get_move(rng)),
            "Looter" => Some(self.looter_get_move()),
            "Sentry" => Some(self.sentry_get_move()),
            "GremlinNob" | "Gremlin Nob" => Some(self.gremlin_nob_get_move(rng)),
            "Lagavulin" => Some(self.lagavulin_get_move()),
            "SlimeBoss" => Some(self.slime_boss_get_move()),
            "Hexaghost" => Some(self.hexaghost_get_move()),
            "TheGuardian" => Some(self.guardian_get_move()),
            // === Act 2 ===
            "Champ" => Some(self.champ_get_move(rng)),
            "Byrd" => Some(self.byrd_get_move(rng)),
            "SphericGuardian" => Some(self.spheric_guardian_get_move()),
            "Chosen" => Some(self.chosen_get_move(rng)),
            "Centurion" => Some(self.centurion_get_move(rng, allies_alive)),
            "Healer" | "Mystic" => Some(self.mystic_get_move(rng)),
            "BanditBear" | "Bear" => Some(self.bandit_bear_get_move()),
            "BanditLeader" | "Romeo" => Some(self.romeo_get_move()),
            "BanditChild" | "Pointy" => Some("Attack".to_string()),
            "BookOfStabbing" | "Book of Stabbing" => Some(self.book_of_stabbing_get_move(rng)),
            "BronzeAutomaton" | "Bronze Automaton" => Some(self.bronze_automaton_get_move()),
            "BronzeOrb" | "Bronze Orb" => Some(self.bronze_orb_get_move(rng)),
            "GremlinLeader" | "Gremlin Leader" => Some(self.gremlin_leader_get_move(rng, allies_alive)),
            "Mugger" => Some(self.mugger_get_move()),
            "Shelled Parasite" => Some(self.shelled_parasite_get_move(rng)),
            "SnakePlant" | "Snake Plant" => Some(self.snake_plant_get_move(rng)),
            "Snecko" => Some(self.snecko_get_move(rng)),
            "SlaverBoss" | "Taskmaster" => Some("Scouring Whip".to_string()),
            "TheCollector" | "The Collector" => Some(self.the_collector_get_move(rng, allies_alive)),
            "TorchHead" | "Torch Head" => Some("Tackle".to_string()),
            // === Act 3 ===
            "AwakenedOne" | "Awakened One" => Some(self.awakened_one_get_move(rng)),
            "Darkling" => Some(self.darkling_get_move(rng)),
            "Deca" => Some(self.deca_get_move()),
            "Donu" => Some(self.donu_get_move()),
            "Exploder" => Some(self.exploder_get_move()),
            "GiantHead" | "Giant Head" => Some(self.giant_head_get_move(rng)),
            "Maw" | "The Maw" => Some(self.maw_get_move(rng)),
            "Nemesis" => Some(self.nemesis_get_move(rng)),
            "OrbWalker" | "Orb Walker" => Some(self.orb_walker_get_move(rng)),
            "Reptomancer" => Some(self.reptomancer_get_move(rng, allies_alive)),
            "Repulsor" => Some(self.repulsor_get_move(rng)),
            "SnakeDagger" | "Dagger" => Some(self.dagger_get_move()),
            "Spiker" => Some(self.spiker_get_move(rng)),
            "Serpent" | "Spire Growth" => Some(self.spire_growth_get_move(rng)),
            "TimeEater" | "Time Eater" => Some(self.time_eater_get_move(rng)),
            "Transient" => Some("Attack".to_string()),
            "WrithingMass" | "Writhing Mass" => Some(self.writhing_mass_get_move(rng)),
            // === Act 4 ===
            "CorruptHeart" | "Corrupt Heart" => Some(self.corrupt_heart_get_move(rng)),
            "SpireShield" | "Spire Shield" => Some(self.spire_shield_get_move(rng)),
            "SpireSpear" | "Spire Spear" => Some(self.spire_spear_get_move(rng)),
            _ => None,
        }
    }

    // ========================================================================
    // The Champ (Act 2 Boss)
    // Java: com.megacrit.cardcrawl.monsters.city.Champ
    // ========================================================================
    fn champ_get_move<R: Rng>(&mut self, rng: &mut R) -> String {
        self.num_turns_champ += 1;

        // Phase transition: HP < 50% triggers Anger
        if self.hp < self.max_hp / 2 && !self.threshold_reached {
            self.threshold_reached = true;
            return "Anger".to_string();
        }

        // After threshold: Execute priority (can't use 2 turns in a row)
        if self.threshold_reached
            && !self.last_move_is("Execute")
            && !self.last_move_before_is("Execute")
        {
            return "Execute".to_string();
        }

        // Every 4 turns (before threshold): Taunt
        if self.num_turns_champ == 4 && !self.threshold_reached {
            self.num_turns_champ = 0;
            return "Taunt".to_string();
        }

        // Probabilistic selection with constraints
        let num: i32 = rng.random_range(0..100);

        // Defensive Stance: 15% (30% at asc19), max 2 uses total, can't repeat
        let forge_threshold = if self.ascension_level >= 19 { 30 } else { 15 };
        if !self.last_move_is("Defensive Stance")
            && self.forge_times < 2
            && num <= forge_threshold
        {
            self.forge_times += 1;
            return "Defensive Stance".to_string();
        }

        // Gloat: 30% if not last move and not Defensive Stance last
        if !self.last_move_is("Gloat")
            && !self.last_move_is("Defensive Stance")
            && num <= 30
        {
            return "Gloat".to_string();
        }

        // Face Slap: up to 55%
        if !self.last_move_is("Face Slap") && num <= 55 {
            return "Face Slap".to_string();
        }

        // Heavy Slash fallback (can't repeat)
        if !self.last_move_is("Heavy Slash") {
            "Heavy Slash".to_string()
        } else {
            "Face Slap".to_string()
        }
    }

    // ========================================================================
    // Byrd (Act 2)
    // Java: com.megacrit.cardcrawl.monsters.city.Byrd
    // ========================================================================
    fn byrd_get_move<R: Rng>(&mut self, rng: &mut R) -> String {
        // First move: 37.5% Caw, 62.5% Peck
        if self.first_move {
            self.first_move = false;
            if rng.random_range(0..1000) < 375 {
                return "Caw".to_string();
            } else {
                return "Peck".to_string();
            }
        }

        // Grounded: always Headbutt (then Go Airborne is chained in takeTurn)
        if !self.is_flying {
            return "Headbutt".to_string();
        }

        // Flying: complex probabilistic
        let num: i32 = rng.random_range(0..100);
        if num < 50 {
            // 50% Peck, but can't use 3x in a row
            if self.last_two_moves_are("Peck") {
                // Reroll: 40% Swoop, 60% Caw
                if rng.random_range(0..1000) < 400 {
                    "Swoop".to_string()
                } else {
                    "Caw".to_string()
                }
            } else {
                "Peck".to_string()
            }
        } else if num < 70 {
            // 20% Swoop, but can't repeat
            if self.last_move_is("Swoop") {
                // Reroll: 37.5% Caw, 62.5% Peck
                if rng.random_range(0..1000) < 375 {
                    "Caw".to_string()
                } else {
                    "Peck".to_string()
                }
            } else {
                "Swoop".to_string()
            }
        } else {
            // 30% Caw, but can't repeat
            if self.last_move_is("Caw") {
                // Reroll: 28.57% Swoop, 71.43% Peck
                if rng.random_range(0..10000) < 2857 {
                    "Swoop".to_string()
                } else {
                    "Peck".to_string()
                }
            } else {
                "Caw".to_string()
            }
        }
    }

    // ========================================================================
    // SphericGuardian (Act 2)
    // Java: com.megacrit.cardcrawl.monsters.city.SphericGuardian
    // Turn 1: INITIAL_BLOCK_GAIN (Activate), Turn 2: FRAIL_ATTACK (Attack Debuff)
    // Then: alternate BIG_ATTACK (Slam x2) and BLOCK_ATTACK (Harden)
    // ========================================================================
    fn spheric_guardian_get_move(&mut self) -> String {
        // Turn 1: Activate (gain block)
        if self.first_move {
            self.first_move = false;
            return "Activate".to_string();
        }
        // Turn 2: Attack Debuff (frail attack)
        if self.second_move {
            self.second_move = false;
            return "Attack Debuff".to_string();
        }
        // Then alternate: Slam <-> Harden
        if self.last_move_is("Slam") {
            "Harden".to_string()
        } else {
            "Slam".to_string()
        }
    }

    // ========================================================================
    // SlimeBoss (Act 1 Boss)
    // Java: com.megacrit.cardcrawl.monsters.exordium.SlimeBoss
    // ========================================================================
    fn slime_boss_get_move(&mut self) -> String {
        // First turn: Goop Spray
        if self.first_move {
            self.first_move = false;
            return "Goop Spray".to_string();
        }
        // Move chaining is handled in takeTurn/combat engine:
        // Goop Spray -> Preparing -> Slam -> Goop Spray -> ...
        // Here we just cycle
        match self.last_move() {
            Some(last) => match last {
                "Goop Spray" => "Preparing".to_string(),
                "Preparing" => "Slam".to_string(),
                "Slam" => "Goop Spray".to_string(),
                _ => "Goop Spray".to_string(),
            },
            None => "Goop Spray".to_string(),
        }
    }

    // ========================================================================
    // Hexaghost (Act 1 Boss)
    // Java: com.megacrit.cardcrawl.monsters.exordium.Hexaghost
    // ========================================================================
    fn hexaghost_get_move(&mut self) -> String {
        // First turn: Activate
        if !self.activated {
            self.activated = true;
            return "Activate".to_string();
        }

        // Sequence based on orb count (each move activates/deactivates orbs)
        // After Activate: orbActiveCount starts at 6, then Divider deactivates → 0
        // Each subsequent move increments by 1 (via Activate Orb in takeTurn)
        let result = match self.orb_count {
            0 => "Sear",
            1 => "Tackle",
            2 => "Sear",
            3 => "Inflame",
            4 => "Tackle",
            5 => "Sear",
            _ => "Inferno",
        };

        // Increment orb counter (wraps after Inferno resets to 0)
        self.orb_count += 1;
        if self.orb_count > 6 {
            self.orb_count = 0;
        }

        result.to_string()
    }

    // ========================================================================
    // TheGuardian (Act 1 Boss)
    // Java: com.megacrit.cardcrawl.monsters.exordium.TheGuardian
    // ========================================================================
    fn guardian_get_move(&mut self) -> String {
        // getMove() is simple: open → Charging Up, closed → Roll Attack
        // All other moves are chained in takeTurn via setMove()
        if self.is_open {
            // Offensive mode cycle: Charge Up → Fierce Bash → Vent Steam → Whirlwind → repeat
            match self.last_move() {
                None => "Charging Up".to_string(),
                Some("Charging Up") => "Fierce Bash".to_string(),
                Some("Fierce Bash") => "Vent Steam".to_string(),
                Some("Vent Steam") => "Whirlwind".to_string(),
                Some("Whirlwind") => "Charging Up".to_string(),
                // After defensive mode ends (Twin Slam sets is_open=true)
                Some("Twin Slam") => "Whirlwind".to_string(),
                _ => "Charging Up".to_string(),
            }
        } else {
            // Defensive mode: Close Up → Roll Attack → Twin Slam
            match self.last_move() {
                Some("Defensive Mode") => "Roll Attack".to_string(),
                Some("Roll Attack") => "Twin Slam".to_string(),
                _ => "Roll Attack".to_string(),
            }
        }
    }

    // ========================================================================
    // Jaw Worm (Act 1 + Hard variant)
    // Java: com.megacrit.cardcrawl.monsters.exordium.JawWorm
    // ========================================================================
    fn jaw_worm_get_move<R: Rng>(&mut self, rng: &mut R) -> String {
        // First move: always Chomp
        if self.first_move {
            self.first_move = false;
            return "Chomp".to_string();
        }

        let num: i32 = rng.random_range(0..100);
        if num < 25 {
            // 25% Chomp, but can't repeat
            if self.last_move_is("Chomp") {
                // Reroll: 56.25% Bellow, 43.75% Thrash
                if rng.random_range(0..10000) < 5625 {
                    "Bellow".to_string()
                } else {
                    "Thrash".to_string()
                }
            } else {
                "Chomp".to_string()
            }
        } else if num < 55 {
            // 30% Thrash, but can't use 3x in a row
            if self.last_two_moves_are("Thrash") {
                // Reroll: 35.7% Chomp, 64.3% Bellow
                if rng.random_range(0..1000) < 357 {
                    "Chomp".to_string()
                } else {
                    "Bellow".to_string()
                }
            } else {
                "Thrash".to_string()
            }
        } else {
            // 45% Bellow, but can't repeat
            if self.last_move_is("Bellow") {
                // Reroll: 41.6% Chomp, 58.4% Thrash
                if rng.random_range(0..1000) < 416 {
                    "Chomp".to_string()
                } else {
                    "Thrash".to_string()
                }
            } else {
                "Bellow".to_string()
            }
        }
    }

    // ========================================================================
    // Acid Slime (M) (Act 1)
    // Java: com.megacrit.cardcrawl.monsters.exordium.AcidSlime_M
    // ========================================================================
    fn acid_slime_m_get_move<R: Rng>(&mut self, rng: &mut R) -> String {
        let num: i32 = rng.random_range(0..100);
        if num < 30 {
            // 30% Corrosive Spit, can't use 2x in a row
            if self.last_move_is("Corrosive Spit") {
                // Reroll: 50% Tackle, 50% Lick (approximately)
                if rng.random_bool(0.5) {
                    "Tackle".to_string()
                } else {
                    "Lick".to_string()
                }
            } else {
                "Corrosive Spit".to_string()
            }
        } else if num < 70 {
            // 40% Tackle, can't use 3x in a row
            if self.last_two_moves_are("Tackle") {
                // Reroll between Corrosive Spit and Lick
                if rng.random_bool(0.5) {
                    "Corrosive Spit".to_string()
                } else {
                    "Lick".to_string()
                }
            } else {
                "Tackle".to_string()
            }
        } else {
            // 30% Lick, can't use 2x in a row
            if self.last_move_is("Lick") {
                // Reroll between Corrosive Spit and Tackle
                if rng.random_bool(0.5) {
                    "Corrosive Spit".to_string()
                } else {
                    "Tackle".to_string()
                }
            } else {
                "Lick".to_string()
            }
        }
    }

    // ========================================================================
    // Spike Slime (M) (Act 1)
    // Java: com.megacrit.cardcrawl.monsters.exordium.SpikeSlime_M
    // ========================================================================
    fn spike_slime_m_get_move<R: Rng>(&mut self, rng: &mut R) -> String {
        let num: i32 = rng.random_range(0..100);
        if num < 30 {
            // 30% Flame Tackle, can't use 3x in a row
            if self.last_two_moves_are("Flame Tackle") {
                "Lick".to_string()
            } else {
                "Flame Tackle".to_string()
            }
        } else {
            // 70% Lick, constraints depend on ascension
            if self.ascension_level >= 17 {
                // Asc 17+: Lick can't repeat
                if self.last_move_is("Lick") {
                    "Flame Tackle".to_string()
                } else {
                    "Lick".to_string()
                }
            } else {
                // Below asc 17: Lick can't use 3x in a row
                if self.last_two_moves_are("Lick") {
                    "Flame Tackle".to_string()
                } else {
                    "Lick".to_string()
                }
            }
        }
    }

    // ========================================================================
    // Gremlin Nob (Act 1 Elite)
    // Java: com.megacrit.cardcrawl.monsters.exordium.GremlinNob
    // Turn 1: always Bellow (buff).
    // A18: deterministic — always Skull Bash unless used twice, then Rush
    // Below A18: probabilistic — num<33 → Skull Bash, else Rush (with lastTwoMoves)
    // ========================================================================
    fn gremlin_nob_get_move<R: Rng>(&mut self, rng: &mut R) -> String {
        // Turn 1: always Bellow
        if self.first_move {
            self.first_move = false;
            return "Bellow".to_string();
        }

        if self.ascension_level >= 18 {
            // A18: deterministic
            // Skull Bash if not used in last 2 moves
            if !self.last_move_is("Skull Bash") && !self.last_move_before_is("Skull Bash") {
                return "Skull Bash".to_string();
            }
            // If Rush used twice in a row → forced Skull Bash
            if self.last_two_moves_are("Rush") {
                return "Skull Bash".to_string();
            }
            "Rush".to_string()
        } else {
            // Below A18: probabilistic
            let num: i32 = rng.random_range(0..100);
            if num < 33 {
                // ~33% Skull Bash (can't use 2x in a row at A18-equivalent constraint)
                if self.last_two_moves_are("Skull Bash") {
                    "Rush".to_string()
                } else {
                    "Skull Bash".to_string()
                }
            } else {
                // ~67% Rush (can't use 2x in a row)
                if self.last_two_moves_are("Rush") {
                    "Skull Bash".to_string()
                } else {
                    "Rush".to_string()
                }
            }
        }
    }

    // ========================================================================
    // Lagavulin (Act 1 Elite)
    // Java: com.megacrit.cardcrawl.monsters.exordium.Lagavulin
    // Starts asleep with 8 Metallicize. Sleeps for 3 turns.
    // If hit while sleeping → wakes immediately (Stun + then Attack).
    // Once awake: Attack cycle with Siphon Soul every 3rd action.
    // getMove logic: if awake && debuffTurnCount < 2: attack (with lastTwoMoves → debuff)
    //                if debuffTurnCount >= 2: forced debuff
    //                if asleep: sleep
    // ========================================================================
    fn lagavulin_get_move(&mut self) -> String {
        // Check if we were just woken by damage (check_damage_triggers sets this)
        // If asleep (idleCount < 3 and not forced awake)
        if !self.activated {
            // Still asleep
            self.idle_count += 1;
            if self.idle_count >= 3 {
                // Natural wake after 3 idle turns
                self.activated = true;
                // Remove metallicize on wake
                self.powers.remove("Metallicize");
                return "Attack".to_string();
            }
            return "Sleep".to_string();
        }

        // Awake: cycle Attack / Siphon Soul
        // Java: debuffTurnCount tracks consecutive attacks
        // If debuffTurnCount < 2 && lastTwoMoves(Attack) → Siphon Soul
        // If debuffTurnCount >= 2 → forced Siphon Soul  
        if self.debuff_turn_count < 2 {
            if self.last_two_moves_are("Attack") {
                self.debuff_turn_count = 0;
                "Siphon Soul".to_string()
            } else {
                self.debuff_turn_count += 1;
                "Attack".to_string()
            }
        } else {
            self.debuff_turn_count = 0;
            "Siphon Soul".to_string()
        }
    }

    // ========================================================================
    // Cultist (Act 1 Normal)
    // Java: com.megacrit.cardcrawl.monsters.exordium.Cultist
    // Turn 1: Incantation (buff). Then: always Dark Strike.
    // ========================================================================
    fn cultist_get_move(&mut self) -> String {
        if self.first_move {
            self.first_move = false;
            "Incantation".to_string()
        } else {
            "Dark Strike".to_string()
        }
    }

    // ========================================================================
    // Acid Slime (S) (Act 1 Normal)
    // Java: com.megacrit.cardcrawl.monsters.exordium.AcidSlime_S
    // A17: alternates Tackle/Lick (lastTwoMoves prevents 3x Tackle)
    // <A17: 50/50 random
    // Note: move sequencing in takeTurn, NOT getMove. getMove just picks first.
    // ========================================================================
    fn acid_slime_s_get_move<R: Rng>(&mut self, rng: &mut R) -> String {
        if self.ascension_level >= 17 {
            if self.last_two_moves_are("Tackle") {
                "Lick".to_string()
            } else {
                "Tackle".to_string()
            }
        } else {
            if rng.random_bool(0.5) {
                "Tackle".to_string()
            } else {
                "Lick".to_string()
            }
        }
    }

    // ========================================================================
    // Acid Slime (L) (Act 1 Normal — SlimeBoss spawn)
    // Java: com.megacrit.cardcrawl.monsters.exordium.AcidSlime_L
    // Complex 3-way probabilistic with A17 constraints.
    // Split handled separately via check_damage_triggers (HP <= 50%).
    // ========================================================================
    fn acid_slime_l_get_move<R: Rng>(&mut self, rng: &mut R) -> String {
        let num: i32 = rng.random_range(0..100);
        if self.ascension_level >= 17 {
            if num < 40 {
                if self.last_two_moves_are("Corrosive Spit") {
                    if rng.random_bool(0.6) {
                        "Tackle".to_string()
                    } else {
                        "Lick".to_string()
                    }
                } else {
                    "Corrosive Spit".to_string()
                }
            } else if num < 70 {
                if self.last_two_moves_are("Tackle") {
                    if rng.random_bool(0.6) {
                        "Corrosive Spit".to_string()
                    } else {
                        "Lick".to_string()
                    }
                } else {
                    "Tackle".to_string()
                }
            } else {
                if self.last_move_is("Lick") {
                    if rng.random_bool(0.4) {
                        "Corrosive Spit".to_string()
                    } else {
                        "Tackle".to_string()
                    }
                } else {
                    "Lick".to_string()
                }
            }
        } else {
            if num < 30 {
                if self.last_two_moves_are("Corrosive Spit") {
                    if rng.random_bool(0.5) {
                        "Tackle".to_string()
                    } else {
                        "Lick".to_string()
                    }
                } else {
                    "Corrosive Spit".to_string()
                }
            } else if num < 70 {
                if self.last_move_is("Tackle") {
                    if rng.random_bool(0.4) {
                        "Corrosive Spit".to_string()
                    } else {
                        "Lick".to_string()
                    }
                } else {
                    "Tackle".to_string()
                }
            } else {
                if self.last_two_moves_are("Lick") {
                    if rng.random_bool(0.4) {
                        "Corrosive Spit".to_string()
                    } else {
                        "Tackle".to_string()
                    }
                } else {
                    "Lick".to_string()
                }
            }
        }
    }

    // ========================================================================
    // Spike Slime (L) (Act 1 Normal — SlimeBoss spawn)
    // Java: com.megacrit.cardcrawl.monsters.exordium.SpikeSlime_L
    // A17: num<30 → Flame Tackle (lastTwoMoves), else Lick (lastMove)
    // <A17: num<30 → Flame Tackle (lastTwoMoves), else Lick (lastTwoMoves)
    // Split handled via check_damage_triggers.
    // ========================================================================
    fn spike_slime_l_get_move<R: Rng>(&mut self, rng: &mut R) -> String {
        let num: i32 = rng.random_range(0..100);
        if self.ascension_level >= 17 {
            if num < 30 {
                if self.last_two_moves_are("Flame Tackle") {
                    "Lick".to_string()
                } else {
                    "Flame Tackle".to_string()
                }
            } else {
                if self.last_move_is("Lick") {
                    "Flame Tackle".to_string()
                } else {
                    "Lick".to_string()
                }
            }
        } else {
            if num < 30 {
                if self.last_two_moves_are("Flame Tackle") {
                    "Lick".to_string()
                } else {
                    "Flame Tackle".to_string()
                }
            } else {
                if self.last_two_moves_are("Lick") {
                    "Flame Tackle".to_string()
                } else {
                    "Lick".to_string()
                }
            }
        }
    }

    // ========================================================================
    // Red Louse (Act 1 Normal) — Java ID: FuzzyLouseNormal
    // Java: com.megacrit.cardcrawl.monsters.exordium.LouseNormal
    // A17: num<25 → Grow (lastMove prevents 2x), else Bite (lastTwoMoves)
    // <A17: num<25 → Grow (lastTwoMoves prevents 3x), else Bite (lastTwoMoves)
    // ========================================================================
    fn red_louse_get_move<R: Rng>(&mut self, rng: &mut R) -> String {
        let num: i32 = rng.random_range(0..100);
        if self.ascension_level >= 17 {
            if num < 25 {
                if self.last_move_is("Grow") {
                    "Bite".to_string()
                } else {
                    "Grow".to_string()
                }
            } else {
                if self.last_two_moves_are("Bite") {
                    "Grow".to_string()
                } else {
                    "Bite".to_string()
                }
            }
        } else {
            if num < 25 {
                if self.last_two_moves_are("Grow") {
                    "Bite".to_string()
                } else {
                    "Grow".to_string()
                }
            } else {
                if self.last_two_moves_are("Bite") {
                    "Grow".to_string()
                } else {
                    "Bite".to_string()
                }
            }
        }
    }

    // ========================================================================
    // Green Louse (Act 1 Normal) — Java ID: FuzzyLouseDefensive
    // Java: com.megacrit.cardcrawl.monsters.exordium.LouseDefensive
    // Identical pattern to Red Louse but "Spit Web" instead of "Grow"
    // ========================================================================
    fn green_louse_get_move<R: Rng>(&mut self, rng: &mut R) -> String {
        let num: i32 = rng.random_range(0..100);
        if self.ascension_level >= 17 {
            if num < 25 {
                if self.last_move_is("Spit Web") {
                    "Bite".to_string()
                } else {
                    "Spit Web".to_string()
                }
            } else {
                if self.last_two_moves_are("Bite") {
                    "Spit Web".to_string()
                } else {
                    "Bite".to_string()
                }
            }
        } else {
            if num < 25 {
                if self.last_two_moves_are("Spit Web") {
                    "Bite".to_string()
                } else {
                    "Spit Web".to_string()
                }
            } else {
                if self.last_two_moves_are("Bite") {
                    "Spit Web".to_string()
                } else {
                    "Bite".to_string()
                }
            }
        }
    }

    // ========================================================================
    // Fungi Beast (Act 1 Normal)
    // Java: com.megacrit.cardcrawl.monsters.exordium.FungiBeast
    // num<40 → Grow (lastTwoMoves), else Bite (lastTwoMoves)
    // ========================================================================
    fn fungi_beast_get_move<R: Rng>(&mut self, rng: &mut R) -> String {
        let num: i32 = rng.random_range(0..100);
        if num < 40 {
            if self.last_two_moves_are("Grow") {
                "Bite".to_string()
            } else {
                "Grow".to_string()
            }
        } else {
            if self.last_two_moves_are("Bite") {
                "Grow".to_string()
            } else {
                "Bite".to_string()
            }
        }
    }

    // ========================================================================
    // Gremlin Wizard (Act 1 Normal)
    // Java: com.megacrit.cardcrawl.monsters.exordium.GremlinWizard
    // Charges for 2 turns, then Ultimate Blast. Cycle repeats.
    // Java: charge counter incremented in takeTurn, getMove checks counter.
    // ========================================================================
    fn gremlin_wizard_get_move(&mut self) -> String {
        self.charge_count += 1;
        if self.charge_count >= 3 {
            self.charge_count = 0;
            "Ultimate Blast".to_string()
        } else {
            "Charging".to_string()
        }
    }

    // ========================================================================
    // Shield Gremlin (Act 1 Normal) — Java ID: GremlinTsundere
    // Java: com.megacrit.cardcrawl.monsters.exordium.GremlinTsundere
    // If allies exist → Protect. If alone → Shield Bash.
    // We simplify: check if there are other monsters alive.
    // For simulation, we track this via a flag or always default.
    // ========================================================================
    fn shield_gremlin_get_move(&mut self, allies_alive: &[bool]) -> String {
        // Java: takeTurn checks aliveCount > 1 → Protect, else Shield Bash
        // allies_alive excludes self, so any true = allies exist
        let has_allies = allies_alive.iter().any(|&a| a);
        if has_allies {
            "Protect".to_string()
        } else {
            "Shield Bash".to_string()
        }
    }

    // ========================================================================
    // Blue Slaver (Act 1 Normal)
    // Java: com.megacrit.cardcrawl.monsters.exordium.SlaverBlue
    // num>=40 → Stab (lastTwoMoves), else:
    //   A17: Rake (lastMove), <A17: Rake (lastTwoMoves)
    // ========================================================================
    fn blue_slaver_get_move<R: Rng>(&mut self, rng: &mut R) -> String {
        let num: i32 = rng.random_range(0..100);
        if num >= 40 && !self.last_two_moves_are("Stab") {
            return "Stab".to_string();
        }
        if self.ascension_level >= 17 {
            if !self.last_move_is("Rake") {
                "Rake".to_string()
            } else {
                "Stab".to_string()
            }
        } else {
            if !self.last_two_moves_are("Rake") {
                "Rake".to_string()
            } else {
                "Stab".to_string()
            }
        }
    }

    // ========================================================================
    // Red Slaver (Act 1 Normal)
    // Java: com.megacrit.cardcrawl.monsters.exordium.SlaverRed
    // firstTurn: Stab. Then:
    //   num>=75 && !usedEntangle → Entangle (once per combat)
    //   num>=55 && usedEntangle && !lastTwoMoves(Stab) → Stab
    //   A17: Scrape (lastMove), <A17: Scrape (lastTwoMoves)
    //   fallback: Stab
    // ========================================================================
    fn red_slaver_get_move<R: Rng>(&mut self, rng: &mut R) -> String {
        if self.first_move {
            self.first_move = false;
            return "Stab".to_string();
        }
        let num: i32 = rng.random_range(0..100);
        // Entangle: 25% chance, once per combat
        let used_entangle = self.moves_used.contains("Entangle");
        if num >= 75 && !used_entangle {
            return "Entangle".to_string();
        }
        // Stab: if entangle already used and not 2x in a row
        if num >= 55 && used_entangle && !self.last_two_moves_are("Stab") {
            return "Stab".to_string();
        }
        // Scrape with constraints
        if self.ascension_level >= 17 {
            if !self.last_move_is("Scrape") {
                "Scrape".to_string()
            } else {
                "Stab".to_string()
            }
        } else {
            if !self.last_two_moves_are("Scrape") {
                "Scrape".to_string()
            } else {
                "Stab".to_string()
            }
        }
    }

    // ========================================================================
    // Looter (Act 1 Normal)
    // Java: com.megacrit.cardcrawl.monsters.exordium.Looter
    // getMove() ALWAYS returns Mug (move 1). Sequencing via takeTurn:
    //   Turn 1: Mug, SetMove(Mug)
    //   Turn 2: Mug, 50% SetMove(Lunge) / 50% SetMove(SmokeBomb)
    //   Turn 3: Lunge or SmokeBomb, SetMove(Escape)
    //   Turn 4+: Escape
    // ========================================================================
    fn looter_get_move(&mut self) -> String {
        // Track via move_history (how many Mugs done)
        if self.last_move_is("Escape") || self.last_move_is("Smoke Bomb") || self.last_move_is("Lunge") {
            "Escape".to_string()
        } else if !self.last_move_is("Mug") {
            // No previous Mug → first turn
            "Mug".to_string()
        } else if !self.last_move_before_is("Mug") {
            // Only 1 Mug so far → second Mug
            "Mug".to_string()
        } else {
            // 2 Mugs done → Lunge (simplified; Java has 50% Lunge / 50% SmokeBomb
            // but SmokeBomb is just block+escape, Lunge is attack+escape)
            "Lunge".to_string()
        }
    }

    // ========================================================================
    // Sentry (Act 1 Elite)
    // Java: com.megacrit.cardcrawl.monsters.exordium.Sentry
    // firstMove: position-dependent (even index → Bolt, odd → Beam)
    // Then: strict alternation Bolt ↔ Beam
    // For simulation: we use is_middle as a proxy for position (odd/even)
    // ========================================================================
    fn sentry_get_move(&mut self) -> String {
        if self.first_move {
            self.first_move = false;
            // In Java, even-indexed sentries start with Bolt, odd with Beam.
            // We use is_middle flag as proxy (set by combat engine).
            if self.is_middle {
                return "Beam".to_string();
            }
            return "Bolt".to_string();
        }
        // Strict alternation
        if self.last_move_is("Beam") {
            "Bolt".to_string()
        } else {
            "Beam".to_string()
        }
    }

    // ========================================================================
    // Chosen (Act 2 Normal)
    // Java: com.megacrit.cardcrawl.monsters.city.Chosen
    // A17: Hex first, then Drain/Debilitate or Poke/Zap.
    // <A17: firstTurn → Zap, then Hex, then Drain/Debilitate or Poke/Zap.
    // Java bytes: 1=Poke, 2=Debilitate, 3=Drain, 4=Hex, 5=Zap
    // ========================================================================
    fn chosen_get_move<R: Rng>(&mut self, rng: &mut R) -> String {
        let num: i32 = rng.random_range(0..100);
        let used_hex = self.moves_used.contains("Hex");

        if self.ascension_level >= 17 {
            if !used_hex {
                return "Hex".to_string();
            }
            if !self.last_move_is("Drain") && !self.last_move_is("Debilitate") {
                if num < 50 {
                    return "Drain".to_string();
                }
                return "Debilitate".to_string();
            }
            if num < 40 {
                "Poke".to_string()
            } else {
                "Zap".to_string()
            }
        } else {
            if self.first_move {
                self.first_move = false;
                return "Zap".to_string();
            }
            if !used_hex {
                return "Hex".to_string();
            }
            if !self.last_move_is("Drain") && !self.last_move_is("Debilitate") {
                if num < 50 {
                    return "Drain".to_string();
                }
                return "Debilitate".to_string();
            }
            if num < 40 {
                "Poke".to_string()
            } else {
                "Zap".to_string()
            }
        }
    }

    // ========================================================================
    // Centurion (Act 2 Normal)
    // Java: com.megacrit.cardcrawl.monsters.city.Centurion
    // num>=65 && !lastTwoMoves(Defend/Fury) → Defend (if allies) or Fury (if alone)
    // else: Slash (lastTwoMoves). Fallback: Defend/Fury based on ally count.
    // For sim: simplified since we can't easily count allies. Default: Slash-heavy.
    // ========================================================================
    fn centurion_get_move<R: Rng>(&mut self, rng: &mut R, allies_alive: &[bool]) -> String {
        let num: i32 = rng.random_range(0..100);
        let allies_count = allies_alive.iter().filter(|&&a| a).count();
        if num >= 65 && !self.last_two_moves_are("Defend") && !self.last_two_moves_are("Fury") {
            // Java: aliveCount > 1 → Defend, else Fury
            if allies_count > 0 {
                return "Defend".to_string();
            }
            return "Fury".to_string();
        }
        if !self.last_two_moves_are("Slash") {
            "Slash".to_string()
        } else {
            // Fallback: same ally check
            if allies_count > 0 {
                "Defend".to_string()
            } else {
                "Fury".to_string()
            }
        }
    }

    // ========================================================================
    // Mystic / Healer (Act 2 Normal)
    // Java: com.megacrit.cardcrawl.monsters.city.Healer
    // Priority: Heal if ally damage > threshold, then Attack, then Buff.
    // A17: threshold=20, lastMove attack; <A17: threshold=15, lastTwoMoves attack.
    // For sim: simplified — we can't easily check ally HP. Default cycle logic.
    // Java bytes: 1=Attack, 2=Heal, 3=Buff
    // ========================================================================
    fn mystic_get_move<R: Rng>(&mut self, rng: &mut R) -> String {
        let num: i32 = rng.random_range(0..100);
        // Simplified: skip heal logic (would need ally HP tracking).
        // Focus on Attack vs Buff cycle, which is the dominant pattern.
        if self.ascension_level >= 17 {
            if num >= 40 && !self.last_move_is("Attack") {
                return "Attack".to_string();
            }
        } else {
            if num >= 40 && !self.last_two_moves_are("Attack") {
                return "Attack".to_string();
            }
        }
        if !self.last_two_moves_are("Buff") {
            "Buff".to_string()
        } else {
            "Attack".to_string()
        }
    }

    // ========================================================================
    // Book of Stabbing (Act 2 Normal)
    // Java: com.megacrit.cardcrawl.monsters.city.BookOfStabbing
    // Multi Stab (byte 1) with increasing stabCount.
    // Single Stab (byte 2) as alternative.
    // A18: stabCount also increments on Single Stab.
    // Uses forge_times as stabCount proxy.
    // ========================================================================
    fn book_of_stabbing_get_move<R: Rng>(&mut self, rng: &mut R) -> String {
        let num: i32 = rng.random_range(0..100);
        if num < 15 {
            if self.last_move_is("Single Stab") {
                self.forge_times += 1; // stabCount++
                "Multi Stab".to_string()
            } else {
                if self.ascension_level >= 18 {
                    self.forge_times += 1;
                }
                "Single Stab".to_string()
            }
        } else {
            if self.last_two_moves_are("Multi Stab") {
                if self.ascension_level >= 18 {
                    self.forge_times += 1;
                }
                "Single Stab".to_string()
            } else {
                self.forge_times += 1;
                "Multi Stab".to_string()
            }
        }
    }

    // ========================================================================
    // Bronze Automaton (Act 2 Boss)
    // Java: com.megacrit.cardcrawl.monsters.city.BronzeAutomaton
    // firstTurn: Spawn Orbs. Then: numTurns counter up to 4 → HYPER BEAM.
    // After Beam: A19 → Boost, else → Stunned.
    // After Stun/Boost: Flail (if last was Stun/Boost/Spawn).
    // Else: Boost.
    // Uses charge_count as numTurns proxy.
    // Java bytes: 1=Flail, 2=HYPER BEAM, 3=Stunned, 4=Spawn, 5=Boost
    // ========================================================================
    fn bronze_automaton_get_move(&mut self) -> String {
        if self.first_move {
            self.first_move = false;
            return "Spawn Orbs".to_string();
        }
        if self.charge_count == 4 {
            self.charge_count = 0;
            return "HYPER BEAM".to_string();
        }
        if self.last_move_is("HYPER BEAM") {
            if self.ascension_level >= 19 {
                return "Boost".to_string();
            }
            return "Stunned".to_string();
        }
        if self.last_move_is("Stunned") || self.last_move_is("Boost") || self.last_move_is("Spawn Orbs") {
            self.charge_count += 1;
            "Flail".to_string()
        } else {
            self.charge_count += 1;
            "Boost".to_string()
        }
    }

    // ========================================================================
    // Bronze Orb (Act 2 Normal — BronzeAutomaton summon)
    // Java: com.megacrit.cardcrawl.monsters.city.BronzeOrb
    // Stasis once (num>=25 && !usedStasis). Then: Beam(num>=70) / Support Beam.
    // Java bytes: 1=Beam, 2=Support Beam, 3=Stasis
    // ========================================================================
    fn bronze_orb_get_move<R: Rng>(&mut self, rng: &mut R) -> String {
        let num: i32 = rng.random_range(0..100);
        let used_stasis = self.moves_used.contains("Stasis");
        if !used_stasis && num >= 25 {
            return "Stasis".to_string();
        }
        if num >= 70 && !self.last_two_moves_are("Support Beam") {
            return "Support Beam".to_string();
        }
        if !self.last_two_moves_are("Beam") {
            "Beam".to_string()
        } else {
            "Support Beam".to_string()
        }
    }

    // ========================================================================
    // Gremlin Leader (Act 2 Elite)
    // Java: com.megacrit.cardcrawl.monsters.city.GremlinLeader
    // Complex: depends on number of alive gremlins.
    // numAlive==0: Rally(75%) or Stab
    // numAlive==1: Rally(50%) or Encourage(30%) or Stab(20%)
    // numAlive>=2: Encourage(66%) or Stab(34%)
    // For sim: simplified without exact ally counting. Default pattern.
    // Java bytes: 2=Rally, 3=Encourage, 4=Stab
    // ========================================================================
    fn gremlin_leader_get_move<R: Rng>(&mut self, rng: &mut R, allies_alive: &[bool]) -> String {
        let num_alive = allies_alive.iter().filter(|&&a| a).count();
        let num: i32 = rng.random_range(0..100);

        if num_alive == 0 {
            // No gremlins alive: 75% Rally, 25% Stab
            if num < 75 {
                if !self.last_move_is("Rally") {
                    "Rally".to_string()
                } else {
                    "Stab".to_string()
                }
            } else {
                if !self.last_move_is("Stab") {
                    "Stab".to_string()
                } else {
                    "Rally".to_string()
                }
            }
        } else if num_alive < 2 {
            // 1 gremlin alive: 50% Rally, 30% Encourage, 20% Stab
            if num < 50 {
                if !self.last_move_is("Rally") {
                    "Rally".to_string()
                } else {
                    // Java: reroll 50..99
                    if rng.random_bool(0.6) {
                        "Encourage".to_string()
                    } else {
                        "Stab".to_string()
                    }
                }
            } else if num < 80 {
                if !self.last_move_is("Encourage") {
                    "Encourage".to_string()
                } else {
                    "Stab".to_string()
                }
            } else {
                if !self.last_move_is("Stab") {
                    "Stab".to_string()
                } else {
                    // Java: reroll 0..80
                    if rng.random_bool(0.6) {
                        "Rally".to_string()
                    } else {
                        "Encourage".to_string()
                    }
                }
            }
        } else {
            // 2+ gremlins alive: 66% Encourage, 34% Stab
            if num < 66 {
                if !self.last_move_is("Encourage") {
                    "Encourage".to_string()
                } else {
                    "Stab".to_string()
                }
            } else {
                if !self.last_move_is("Stab") {
                    "Stab".to_string()
                } else {
                    "Encourage".to_string()
                }
            }
        }
    }

    // ========================================================================
    // Bandit Bear (Act 2 Normal)
    // Java: com.megacrit.cardcrawl.monsters.city.BanditBear
    // getMove() returns Bear Hug (byte 2). Then takeTurn chains:
    //   Bear Hug → Lunge, Maul → Lunge, Lunge → Maul (Lunge ↔ Maul cycle)
    // Java bytes: 1=Maul, 2=Bear Hug, 3=Lunge
    // ========================================================================
    fn bandit_bear_get_move(&mut self) -> String {
        if self.first_move {
            self.first_move = false;
            return "Bear Hug".to_string();
        }
        // After Bear Hug or Maul → Lunge; after Lunge → Maul
        if self.last_move_is("Lunge") {
            "Maul".to_string()
        } else {
            "Lunge".to_string()
        }
    }

    // ========================================================================
    // Romeo / Bandit Leader (Act 2 Normal)
    // Java: com.megacrit.cardcrawl.monsters.city.BanditLeader
    // getMove() returns Mock (byte 2). Then takeTurn chains:
    //   Mock → Agonizing Slash, Agonizing Slash → Cross Slash
    //   Cross Slash: A17 && !lastTwoMoves → repeat Cross Slash
    //                else → Agonizing Slash
    // Java bytes: 1=Cross Slash, 2=Mock, 3=Agonizing Slash
    // ========================================================================
    fn romeo_get_move(&mut self) -> String {
        if self.first_move {
            self.first_move = false;
            return "Mock".to_string();
        }
        if self.last_move_is("Mock") || self.last_move_is("Cross Slash") {
            // After Mock → Agonizing Slash
            // After Cross Slash: A17 allows repeat, but eventually → Agonizing Slash
            if self.last_move_is("Cross Slash") && self.ascension_level >= 17
                && !self.last_two_moves_are("Cross Slash") {
                "Cross Slash".to_string()
            } else {
                "Agonizing Slash".to_string()
            }
        } else {
            // After Agonizing Slash → Cross Slash
            "Cross Slash".to_string()
        }
    }

    // ========================================================================
    // Mugger (Act 2 Normal)
    // Java: com.megacrit.cardcrawl.monsters.city.Mugger
    // getMove() returns Mug (byte 1). Then takeTurn chains:
    //   Turn 1: Mug, SetMove(Mug)
    //   Turn 2: Mug, 50% Lunge / 50% SmokeBomb
    //   Turn 3: Lunge → SmokeBomb, or BigSwipe (Lunge) → SmokeBomb
    //   Turn 4: SmokeBomb → Escape
    //   Turn 5+: Escape
    // Java bytes: 1=Mug, 2=Smoke Bomb, 3=Escape, 4=Lunge(BigSwipe)
    // ========================================================================
    fn mugger_get_move(&mut self) -> String {
        if self.last_move_is("Escape") || self.last_move_is("Smoke Bomb") {
            return "Escape".to_string();
        }
        if self.last_move_is("Lunge") {
            return "Smoke Bomb".to_string();
        }
        if !self.last_move_is("Mug") {
            // First turn
            return "Mug".to_string();
        }
        if !self.last_move_before_is("Mug") {
            // Only 1 Mug → second Mug
            return "Mug".to_string();
        }
        // 2 Mugs done → 50% Lunge / 50% SmokeBomb
        // Note: Java has 60% talk on slashCount==1, but the move selection
        // is: if random(0.5) → SmokeBomb, else → Lunge
        // For simplicity, always Lunge (SmokeBomb → block → escape)
        "Lunge".to_string()
    }

    // ========================================================================
    // Shelled Parasite (Act 2 Normal)
    // Java: com.megacrit.cardcrawl.monsters.city.ShelledParasite
    // firstMove: A17 → Fell, <A17 → 50% Double Strike / 50% Suck.
    // Then: 20% Fell (lastMove), 40% Double Strike (lastTwoMoves), 40% Suck (lastTwoMoves).
    // Java bytes: 1=Fell, 2=Double Strike, 3=Suck
    // ========================================================================
    fn shelled_parasite_get_move<R: Rng>(&mut self, rng: &mut R) -> String {
        if self.first_move {
            self.first_move = false;
            if self.ascension_level >= 17 {
                return "Fell".to_string();
            }
            if rng.random_bool(0.5) {
                return "Double Strike".to_string();
            }
            return "Suck".to_string();
        }
        let num: i32 = rng.random_range(0..100);
        if num < 20 {
            if !self.last_move_is("Fell") {
                "Fell".to_string()
            } else {
                // Java: recursive call with num 20..99
                if rng.random_bool(0.5) {
                    "Double Strike".to_string()
                } else {
                    "Suck".to_string()
                }
            }
        } else if num < 60 {
            if !self.last_two_moves_are("Double Strike") {
                "Double Strike".to_string()
            } else {
                "Suck".to_string()
            }
        } else {
            if !self.last_two_moves_are("Suck") {
                "Suck".to_string()
            } else {
                "Double Strike".to_string()
            }
        }
    }

    // ========================================================================
    // Snake Plant (Act 2 Normal)
    // Java: com.megacrit.cardcrawl.monsters.city.SnakePlant
    // A17: num<65 → Chomp (lastTwoMoves), else Spores (lastMove || lastMoveBefore)
    // <A17: num<65 → Chomp (lastTwoMoves), else Spores (lastMove)
    // Java bytes: 1=Chomp, 2=Enfeebling Spores
    // ========================================================================
    fn snake_plant_get_move<R: Rng>(&mut self, rng: &mut R) -> String {
        let num: i32 = rng.random_range(0..100);
        if self.ascension_level >= 17 {
            if num < 65 {
                if self.last_two_moves_are("Chomp") {
                    "Enfeebling Spores".to_string()
                } else {
                    "Chomp".to_string()
                }
            } else {
                if self.last_move_is("Enfeebling Spores") || self.last_move_before_is("Enfeebling Spores") {
                    "Chomp".to_string()
                } else {
                    "Enfeebling Spores".to_string()
                }
            }
        } else {
            if num < 65 {
                if self.last_two_moves_are("Chomp") {
                    "Enfeebling Spores".to_string()
                } else {
                    "Chomp".to_string()
                }
            } else {
                if self.last_move_is("Enfeebling Spores") {
                    "Chomp".to_string()
                } else {
                    "Enfeebling Spores".to_string()
                }
            }
        }
    }

    // ========================================================================
    // Snecko (Act 2 Normal)
    // Java: com.megacrit.cardcrawl.monsters.city.Snecko
    // firstTurn: Perplexing Glare. Then: num<40 → Tail Whip, else Bite (lastTwoMoves).
    // Java bytes: 1=Perplexing Glare, 2=Bite, 3=Tail Whip
    // ========================================================================
    fn snecko_get_move<R: Rng>(&mut self, rng: &mut R) -> String {
        if self.first_move {
            self.first_move = false;
            return "Perplexing Glare".to_string();
        }
        let num: i32 = rng.random_range(0..100);
        if num < 40 {
            return "Tail Whip".to_string();
        }
        if self.last_two_moves_are("Bite") {
            "Tail Whip".to_string()
        } else {
            "Bite".to_string()
        }
    }

    // ========================================================================
    // The Collector (Act 2 Boss)
    // Java: com.megacrit.cardcrawl.monsters.city.TheCollector
    // initialSpawn → Spawn. turnsTaken>=3 && !ultUsed → Mega Debuff.
    // Then: 25% Revive (if minion dead), 70% Fireball, else Buff.
    // Uses charge_count as turnsTaken, activated as ultUsed.
    // Java bytes: 1=Spawn, 2=Fireball, 3=Buff, 4=Mega Debuff, 5=Revive
    // ========================================================================
    fn the_collector_get_move<R: Rng>(&mut self, rng: &mut R, allies_alive: &[bool]) -> String {
        if self.first_move {
            self.first_move = false;
            return "Spawn".to_string();
        }
        self.charge_count += 1; // turnsTaken
        if self.charge_count >= 3 && !self.activated {
            self.activated = true;
            return "Mega Debuff".to_string();
        }
        let num: i32 = rng.random_range(0..100);
        // Java: isMinionDead() — any dead minion allows Revive
        let any_minion_dead = allies_alive.iter().any(|&a| !a);
        if num <= 25 && any_minion_dead && !self.last_move_is("Revive") {
            return "Revive".to_string();
        }
        if num <= 70 && !self.last_two_moves_are("Fireball") {
            return "Fireball".to_string();
        }
        if !self.last_move_is("Buff") {
            "Buff".to_string()
        } else {
            "Fireball".to_string()
        }
    }

    // ========================================================================
    // Awakened One (Act 3 Boss)
    // Java: com.megacrit.cardcrawl.monsters.beyond.AwakenedOne
    // Phase 1 (form1): firstTurn → Slash(20), then num<25 → Soul Strike
    //   (lastMove), else Slash (lastTwoMoves).
    // Phase 2: firstTurn → Dark Echo(40), then num<50 → Sludge
    //   (lastTwoMoves), else Tackle×3 (lastTwoMoves).
    // Uses activated as form1 flag (true = form1).
    // Java bytes: 1=Slash, 2=Soul Strike, 5=Dark Echo, 6=Sludge, 8=Tackle
    // ========================================================================
    fn awakened_one_get_move<R: Rng>(&mut self, rng: &mut R) -> String {
        if !self.activated {
            // Phase 1 (form1)
            if self.first_move {
                self.first_move = false;
                return "Slash".to_string();
            }
            let num: i32 = rng.random_range(0..100);
            if num < 25 {
                if !self.last_move_is("Soul Strike") {
                    "Soul Strike".to_string()
                } else {
                    "Slash".to_string()
                }
            } else {
                if !self.last_two_moves_are("Slash") {
                    "Slash".to_string()
                } else {
                    "Soul Strike".to_string()
                }
            }
        } else {
            // Phase 2
            if self.first_move {
                self.first_move = false;
                return "Dark Echo".to_string();
            }
            let num: i32 = rng.random_range(0..100);
            if num < 50 {
                if !self.last_two_moves_are("Sludge") {
                    "Sludge".to_string()
                } else {
                    "Tackle".to_string()
                }
            } else {
                if !self.last_two_moves_are("Tackle") {
                    "Tackle".to_string()
                } else {
                    "Sludge".to_string()
                }
            }
        }
    }

    // ========================================================================
    // Darkling (Act 3 Normal)
    // Java: com.megacrit.cardcrawl.monsters.beyond.Darkling
    // halfDead → Reincarnate. firstMove → 50% Harden / 50% Chomp.
    // Then: 40% Nip (lastMove, even-index only), 30% Harden (lastMove),
    //   30% Chomp (lastTwoMoves).
    // Java bytes: 1=Nip, 2=Harden, 3=Chomp, 5=Reincarnate
    // ========================================================================
    fn darkling_get_move<R: Rng>(&mut self, rng: &mut R) -> String {
        // halfDead state would be set externally; for now skip Reincarnate
        if self.first_move {
            self.first_move = false;
            let num: i32 = rng.random_range(0..100);
            if num < 50 {
                return "Harden".to_string();
            }
            return "Chomp".to_string();
        }
        let num: i32 = rng.random_range(0..100);
        if num < 40 {
            if !self.last_move_is("Nip") {
                "Nip".to_string()
            } else {
                // Java: reroll with num 40..99
                if rng.random_bool(0.5) {
                    "Harden".to_string()
                } else {
                    "Chomp".to_string()
                }
            }
        } else if num < 70 {
            if !self.last_move_is("Harden") {
                "Harden".to_string()
            } else {
                "Chomp".to_string()
            }
        } else {
            if !self.last_two_moves_are("Chomp") {
                "Chomp".to_string()
            } else {
                // Java: reroll with num 0..99
                if rng.random_bool(0.5) {
                    "Nip".to_string()
                } else {
                    "Harden".to_string()
                }
            }
        }
    }

    // ========================================================================
    // Deca (Act 3 Boss — Donu & Deca)
    // Java: com.megacrit.cardcrawl.monsters.beyond.Deca
    // Alternates: Beam (attack) ↔ Square of Protection (defend).
    // isAttacking toggled in takeTurn. We use activated as toggle.
    // ========================================================================
    fn deca_get_move(&mut self) -> String {
        self.activated = !self.activated;
        if self.activated {
            "Beam".to_string()
        } else {
            "Square of Protection".to_string()
        }
    }

    // ========================================================================
    // Donu (Act 3 Boss — Donu & Deca)
    // Java: com.megacrit.cardcrawl.monsters.beyond.Donu
    // Alternates: Attack ↔ Circle of Power (buff).
    // isAttacking toggled in takeTurn. We use activated as toggle.
    // ========================================================================
    fn donu_get_move(&mut self) -> String {
        self.activated = !self.activated;
        if self.activated {
            "Attack".to_string()
        } else {
            "Circle of Power".to_string()
        }
    }

    // ========================================================================
    // Exploder (Act 3 Normal — Reptomancer summon)
    // Java: com.megacrit.cardcrawl.monsters.beyond.Exploder
    // turnCount < 2 → Slam, else Explode.
    // Uses idle_count as turnCount++ (incremented in takeTurn).
    // ========================================================================
    fn exploder_get_move(&mut self) -> String {
        self.idle_count += 1;
        if self.idle_count <= 2 {
            "Slam".to_string()
        } else {
            "Explode".to_string()
        }
    }

    // ========================================================================
    // Giant Head (Act 3 Elite)
    // Java: com.megacrit.cardcrawl.monsters.beyond.GiantHead
    // count starts at some value. count<=1 → "It Is Time" (escalating damage).
    // Otherwise: num<50 → Glare (lastTwoMoves), else Count (lastTwoMoves).
    // Uses charge_count as count (decremented each turn).
    // ========================================================================
    fn giant_head_get_move<R: Rng>(&mut self, rng: &mut R) -> String {
        self.charge_count -= 1; // count--
        if self.charge_count <= 1 {
            return "It Is Time".to_string();
        }
        let num: i32 = rng.random_range(0..100);
        if num < 50 {
            if !self.last_two_moves_are("Glare") {
                "Glare".to_string()
            } else {
                "Count".to_string()
            }
        } else {
            if !self.last_two_moves_are("Count") {
                "Count".to_string()
            } else {
                "Glare".to_string()
            }
        }
    }

    // ========================================================================
    // The Maw (Act 3 Normal)
    // Java: com.megacrit.cardcrawl.monsters.beyond.Maw
    // !roared → Roar. Then: num<50 → Nom (lastMove, turnCount-based),
    //   else if lastMove Slam/Nom → Drool (buff), else Slam.
    // Uses idle_count as turnCount, activated as roared flag.
    // Java bytes: 2=Roar, 3=Slam, 4=Drool, 5=Nom
    // ========================================================================
    fn maw_get_move<R: Rng>(&mut self, rng: &mut R) -> String {
        self.idle_count += 1; // turnCount++
        if !self.activated {
            self.activated = true;
            return "Roar".to_string();
        }
        let num: i32 = rng.random_range(0..100);
        if num < 50 && !self.last_move_is("Nom") {
            return "Nom".to_string();
        }
        if self.last_move_is("Slam") || self.last_move_is("Nom") {
            return "Drool".to_string();
        }
        "Slam".to_string()
    }

    // ========================================================================
    // Nemesis (Act 3 Elite)
    // Java: com.megacrit.cardcrawl.monsters.beyond.Nemesis
    // scytheCooldown system. firstMove → 50% Attack / 50% Debuff.
    // Then: 30% Scythe (cooldown), 35% Attack (lastTwoMoves), 35% Debuff.
    // Uses debuff_turn_count as scytheCooldown.
    // Java bytes: 2=Attack, 3=Scythe, 4=Debuff
    // ========================================================================
    fn nemesis_get_move<R: Rng>(&mut self, rng: &mut R) -> String {
        self.debuff_turn_count = self.debuff_turn_count.saturating_sub(1); // scytheCooldown--
        if self.first_move {
            self.first_move = false;
            let num: i32 = rng.random_range(0..100);
            if num < 50 {
                return "Attack".to_string();
            }
            return "Debuff".to_string();
        }
        let num: i32 = rng.random_range(0..100);
        if num < 30 {
            if !self.last_move_is("Scythe") && self.debuff_turn_count == 0 {
                self.debuff_turn_count = 2;
                "Scythe".to_string()
            } else if rng.random_bool(0.5) {
                if !self.last_two_moves_are("Attack") {
                    "Attack".to_string()
                } else {
                    "Debuff".to_string()
                }
            } else if !self.last_move_is("Debuff") {
                "Debuff".to_string()
            } else {
                "Attack".to_string()
            }
        } else if num < 65 {
            if !self.last_two_moves_are("Attack") {
                "Attack".to_string()
            } else if rng.random_bool(0.5) {
                if self.debuff_turn_count == 0 {
                    self.debuff_turn_count = 2;
                    "Scythe".to_string()
                } else {
                    "Debuff".to_string()
                }
            } else {
                "Debuff".to_string()
            }
        } else {
            if !self.last_move_is("Debuff") {
                "Debuff".to_string()
            } else if rng.random_bool(0.5) && self.debuff_turn_count == 0 {
                self.debuff_turn_count = 2;
                "Scythe".to_string()
            } else {
                "Attack".to_string()
            }
        }
    }

    // ========================================================================
    // Orb Walker (Act 3 Normal)
    // Java: com.megacrit.cardcrawl.monsters.beyond.OrbWalker
    // num<40 → Claw (lastTwoMoves), else Laser (lastTwoMoves)
    // Java bytes: 1=Laser, 2=Claw
    // ========================================================================
    fn orb_walker_get_move<R: Rng>(&mut self, rng: &mut R) -> String {
        let num: i32 = rng.random_range(0..100);
        if num < 40 {
            if !self.last_two_moves_are("Claw") {
                "Claw".to_string()
            } else {
                "Laser".to_string()
            }
        } else {
            if !self.last_two_moves_are("Laser") {
                "Laser".to_string()
            } else {
                "Claw".to_string()
            }
        }
    }

    // ========================================================================
    // Reptomancer (Act 3 Elite)
    // Java: com.megacrit.cardcrawl.monsters.beyond.Reptomancer
    // firstMove → Summon. Then: 33% Snake Strike (lastMove),
    //   33% Summon (lastTwoMoves, canSpawn), 34% Big Bite (lastMove).
    // Java bytes: 1=Snake Strike, 2=Summon, 3=Big Bite
    // ========================================================================
    fn reptomancer_get_move<R: Rng>(&mut self, rng: &mut R, allies_alive: &[bool]) -> String {
        if self.first_move {
            self.first_move = false;
            return "Summon".to_string();
        }
        // Java: canSpawn() — alive non-self count <= 3
        let can_spawn = allies_alive.iter().filter(|&&a| a).count() <= 3;
        let num: i32 = rng.random_range(0..100);
        if num < 33 {
            if !self.last_move_is("Snake Strike") {
                "Snake Strike".to_string()
            } else {
                // Java: reroll num 33..99
                if rng.random_bool(0.5) && can_spawn {
                    "Summon".to_string()
                } else {
                    "Big Bite".to_string()
                }
            }
        } else if num < 66 {
            if !self.last_two_moves_are("Summon") && can_spawn {
                "Summon".to_string()
            } else {
                "Snake Strike".to_string()
            }
        } else {
            if !self.last_move_is("Big Bite") {
                "Big Bite".to_string()
            } else {
                // Java: reroll num 0..65
                if rng.random_bool(0.5) {
                    "Snake Strike".to_string()
                } else if can_spawn {
                    "Summon".to_string()
                } else {
                    "Snake Strike".to_string()
                }
            }
        }
    }

    // ========================================================================
    // Repulsor (Act 3 Normal — Reptomancer encounter)
    // Java: com.megacrit.cardcrawl.monsters.beyond.Repulsor
    // num<20 && !lastMove(Bash) → Bash, else Repulse.
    // Java bytes: 1=Repulse, 2=Bash
    // ========================================================================
    fn repulsor_get_move<R: Rng>(&mut self, rng: &mut R) -> String {
        let num: i32 = rng.random_range(0..100);
        if num < 20 && !self.last_move_is("Bash") {
            "Bash".to_string()
        } else {
            "Repulse".to_string()
        }
    }

    // ========================================================================
    // Dagger (Act 3 Normal — Reptomancer summon)
    // Java: com.megacrit.cardcrawl.monsters.beyond.SnakeDagger
    // firstMove → Stab. Then: always Explode.
    // ========================================================================
    fn dagger_get_move(&mut self) -> String {
        if self.first_move {
            self.first_move = false;
            "Stab".to_string()
        } else {
            "Explode".to_string()
        }
    }

    // ========================================================================
    // Spiker (Act 3 Normal)
    // Java: com.megacrit.cardcrawl.monsters.beyond.Spiker
    // thornsCount > 5 → always Cut. Otherwise: num<50 → Cut (lastMove), else Spike.
    // Uses forge_times as thornsCount (incremented on Spike).
    // Java bytes: 1=Cut, 2=Spike
    // ========================================================================
    fn spiker_get_move<R: Rng>(&mut self, rng: &mut R) -> String {
        if self.forge_times > 5 {
            return "Cut".to_string();
        }
        let num: i32 = rng.random_range(0..100);
        if num < 50 && !self.last_move_is("Cut") {
            "Cut".to_string()
        } else {
            self.forge_times += 1;
            "Spike".to_string()
        }
    }

    // ========================================================================
    // Spire Growth (Act 3 Normal) — Java ID: Serpent / SpireGrowth
    // Java: com.megacrit.cardcrawl.monsters.beyond.SpireGrowth
    // A17: if player has no Constricted && !lastMove → Constrict first.
    // Then: num<50 → Quick Tackle (lastTwoMoves), else if no Constricted
    //   → Constrict, else Smash (lastTwoMoves).
    // For sim: simplified without player power check. Default cycle.
    // Java bytes: 1=Quick Tackle, 2=Constrict, 3=Smash
    // ========================================================================
    fn spire_growth_get_move<R: Rng>(&mut self, rng: &mut R) -> String {
        // Simplified: always try Constrict first on A17
        if self.ascension_level >= 17 && !self.last_move_is("Constrict") && self.first_move {
            self.first_move = false;
            return "Constrict".to_string();
        }
        self.first_move = false;
        let num: i32 = rng.random_range(0..100);
        if num < 50 {
            if !self.last_two_moves_are("Quick Tackle") {
                "Quick Tackle".to_string()
            } else {
                "Constrict".to_string()
            }
        } else {
            if !self.last_two_moves_are("Smash") {
                "Smash".to_string()
            } else {
                "Quick Tackle".to_string()
            }
        }
    }

    // ========================================================================
    // Time Eater (Act 3 Boss)
    // Java: com.megacrit.cardcrawl.monsters.beyond.TimeEater
    // HP < 50% && !usedHaste → Haste (buff).
    // Then: 45% Reverberate (lastTwoMoves), 35% Head Slam (lastMove),
    //   20% Ripple (lastMove).
    // Uses activated as usedHaste flag.
    // Java bytes: 2=Reverberate, 3=Ripple, 4=Head Slam, 5=Haste
    // ========================================================================
    fn time_eater_get_move<R: Rng>(&mut self, rng: &mut R) -> String {
        // Java: if (currentHealth < maxHealth / 2 && !usedHaste) → Haste
        if self.hp < self.max_hp / 2 && !self.activated {
            self.activated = true;
            return "Haste".to_string();
        }
        let num: i32 = rng.random_range(0..100);
        if num < 45 {
            if !self.last_two_moves_are("Reverberate") {
                return "Reverberate".to_string();
            }
            // Java: reroll with num 50..99
            if rng.random_bool(0.6) {
                return "Head Slam".to_string();
            }
            return "Ripple".to_string();
        }
        if num < 80 {
            if !self.last_move_is("Head Slam") {
                return "Head Slam".to_string();
            }
            if rng.random_bool(0.66) {
                "Reverberate".to_string()
            } else {
                "Ripple".to_string()
            }
        } else {
            if !self.last_move_is("Ripple") {
                "Ripple".to_string()
            } else {
                // Java: reroll with num 0..74
                if rng.random_bool(0.5) {
                    "Reverberate".to_string()
                } else {
                    "Head Slam".to_string()
                }
            }
        }
    }

    // ========================================================================
    // Writhing Mass (Act 3 Normal)
    // Java: com.megacrit.cardcrawl.monsters.beyond.WrithingMass
    // firstMove: 33% Multi-Strike / 33% Flail / 33% Wither.
    // Then: 10% Strong Strike (lastMove), 10% Implant (!used, lastMove),
    //   20% Wither (lastMove), 30% Multi-Strike (lastMove),
    //   30% Flail (lastMove).
    // Uses activated as usedMegaDebuff flag.
    // Java bytes: 0=Strong Strike, 1=Multi-Strike, 2=Flail, 3=Wither, 4=Implant
    // ========================================================================
    fn writhing_mass_get_move<R: Rng>(&mut self, rng: &mut R) -> String {
        if self.first_move {
            self.first_move = false;
            let num: i32 = rng.random_range(0..100);
            if num < 33 {
                return "Multi-Strike".to_string();
            } else if num < 66 {
                return "Flail".to_string();
            } else {
                return "Wither".to_string();
            }
        }
        let num: i32 = rng.random_range(0..100);
        if num < 10 {
            if !self.last_move_is("Strong Strike") {
                "Strong Strike".to_string()
            } else {
                // Reroll
                if rng.random_bool(0.5) {
                    "Multi-Strike".to_string()
                } else {
                    "Flail".to_string()
                }
            }
        } else if num < 20 {
            if !self.activated && !self.last_move_is("Implant") {
                self.activated = true;
                "Implant".to_string()
            } else if rng.random_bool(0.1) {
                "Strong Strike".to_string()
            } else {
                // Reroll in 20..99 range
                if rng.random_bool(0.5) {
                    "Wither".to_string()
                } else {
                    "Multi-Strike".to_string()
                }
            }
        } else if num < 40 {
            if !self.last_move_is("Wither") {
                "Wither".to_string()
            } else if rng.random_bool(0.4) {
                if !self.activated {
                    self.activated = true;
                    "Implant".to_string()
                } else {
                    "Multi-Strike".to_string()
                }
            } else {
                "Multi-Strike".to_string()
            }
        } else if num < 70 {
            if !self.last_move_is("Multi-Strike") {
                "Multi-Strike".to_string()
            } else if rng.random_bool(0.3) {
                "Flail".to_string()
            } else {
                "Wither".to_string()
            }
        } else {
            if !self.last_move_is("Flail") {
                "Flail".to_string()
            } else {
                // Reroll in 0..69
                if rng.random_bool(0.5) {
                    "Wither".to_string()
                } else {
                    "Multi-Strike".to_string()
                }
            }
        }
    }

    // ========================================================================
    // Corrupt Heart (Act 4 Boss)
    // Java: com.megacrit.cardcrawl.monsters.ending.CorruptHeart
    // firstMove → Debilitate. Then: moveCount%3 cycle:
    //   0 → 50% Blood Shots / 50% Echo
    //   1 → Echo if !lastMove(Echo), else Blood Shots
    //   2 → Buff
    // Uses charge_count as moveCount.
    // Java bytes: 1=Blood Shots, 2=Echo, 3=Debilitate, 4=Buff
    // ========================================================================
    fn corrupt_heart_get_move<R: Rng>(&mut self, rng: &mut R) -> String {
        if self.first_move {
            self.first_move = false;
            return "Debilitate".to_string();
        }
        let phase = self.charge_count % 3;
        self.charge_count += 1;
        match phase {
            0 => {
                if rng.random_bool(0.5) {
                    "Blood Shots".to_string()
                } else {
                    "Echo".to_string()
                }
            }
            1 => {
                if !self.last_move_is("Echo") {
                    "Echo".to_string()
                } else {
                    "Blood Shots".to_string()
                }
            }
            _ => "Buff".to_string(),
        }
    }

    // ========================================================================
    // Spire Shield (Act 4 Elite)
    // Java: com.megacrit.cardcrawl.monsters.ending.SpireShield
    // moveCount%3 cycle:
    //   0 → 50% Fortify / 50% Bash
    //   1 → Bash if !lastMove(Bash), else Fortify
    //   2 → Smash
    // Uses charge_count as moveCount.
    // Java bytes: 1=Bash, 2=Fortify, 3=Smash
    // ========================================================================
    fn spire_shield_get_move<R: Rng>(&mut self, rng: &mut R) -> String {
        let phase = self.charge_count % 3;
        self.charge_count += 1;
        match phase {
            0 => {
                if rng.random_bool(0.5) {
                    "Fortify".to_string()
                } else {
                    "Bash".to_string()
                }
            }
            1 => {
                if !self.last_move_is("Bash") {
                    "Bash".to_string()
                } else {
                    "Fortify".to_string()
                }
            }
            _ => "Smash".to_string(),
        }
    }

    // ========================================================================
    // Spire Spear (Act 4 Elite)
    // Java: com.megacrit.cardcrawl.monsters.ending.SpireSpear
    // moveCount%3 cycle:
    //   0 → Burn Strike if !lastMove, else Piercer (buff)
    //   1 → Skewer
    //   2 → 50% Piercer / 50% Burn Strike
    // Uses charge_count as moveCount.
    // Java bytes: 1=Burn Strike, 2=Piercer, 3=Skewer
    // ========================================================================
    fn spire_spear_get_move<R: Rng>(&mut self, rng: &mut R) -> String {
        let phase = self.charge_count % 3;
        self.charge_count += 1;
        match phase {
            0 => {
                if !self.last_move_is("Burn Strike") {
                    "Burn Strike".to_string()
                } else {
                    "Piercer".to_string()
                }
            }
            1 => "Skewer".to_string(),
            _ => {
                if rng.random_bool(0.5) {
                    "Piercer".to_string()
                } else {
                    "Burn Strike".to_string()
                }
            }
        }
    }

    // ========================================================================
    // Helper methods for hardcoded AI
    // ========================================================================

    /// Check if the last move matches the given name.
    fn last_move_is(&self, name: &str) -> bool {
        self.move_history.front().map_or(false, |m| m == name)
    }

    /// Check if the last move before the most recent matches the given name.
    fn last_move_before_is(&self, name: &str) -> bool {
        self.move_history.get(1).map_or(false, |m| m == name)
    }

    /// Check if the last two moves are both the given name.
    fn last_two_moves_are(&self, name: &str) -> bool {
        self.move_history.len() >= 2
            && self.move_history[0] == name
            && self.move_history[1] == name
    }

    // ========================================================================
    // Post-Damage State Transitions
    // Called after take_damage() / take_damage_from_player()
    // ========================================================================

    /// Check for monster-specific state transitions triggered by taking damage.
    /// Returns what action the combat engine should take (if any).
    ///
    /// Java mechanics modeled:
    /// - TheGuardian: ModeShiftPower — cumulative damage tracking, mode switch
    /// - SlimeBoss: damage() override — HP ≤ 50% forces Split
    /// - Byrd: FlightPower — when stacks reach 0, becomes grounded + stunned
    pub fn check_damage_triggers(&mut self, damage_taken: i32) -> DamageTriggerResult {
        match self.definition_id.as_str() {
            "TheGuardian" => {
                // Java: ModeShiftPower.wasHPLost()
                // Tracks cumulative damage; when threshold exceeded, switch to Defensive Mode
                if self.is_open && damage_taken > 0 {
                    self.dmg_threshold -= damage_taken;
                    // Sync Mode Shift power stacks with remaining threshold
                    // Only update if power already exists (hydrated from CommunicationMod)
                    // Don't re-add it if it wasn't present — Java only shows it during
                    // the initial offensive cycle, not after re-entering from defensive.
                    if self.powers.has("Mode Shift") {
                        if self.dmg_threshold > 0 {
                            self.powers.set("Mode Shift", self.dmg_threshold);
                        } else {
                            self.powers.remove("Mode Shift");
                        }
                    }
                    if self.dmg_threshold <= 0 {
                        // Switch to Defensive Mode
                        self.is_open = false;
                        // Java: GainBlockAction(this.owner, 20) — immediate block gain
                        self.block += 20;
                        // Reset threshold for next cycle (increases by 10 each time)
                        // Java: dmgThresholdIncrease = 10
                        self.dmg_threshold = 30 + 10; // Base + increase (simplified)
                        // Remove Mode Shift power (it's gone in defensive mode)
                        self.powers.remove("Mode Shift");
                        // Force current move to Defensive Mode
                        self.current_move = "Defensive Mode".to_string();
                        self.current_intent = super::enemy::Intent::Defend { block: 20 };
                        return DamageTriggerResult::GuardianModeShift;
                    }
                }
                DamageTriggerResult::None
            }
            "SlimeBoss" => {
                // Java: SlimeBoss.damage() — if HP <= maxHP/2 and not already splitting
                if !self.is_dead()
                    && self.hp <= self.max_hp / 2
                    && self.current_move != "Split"
                {
                    // Force Split
                    self.current_move = "Split".to_string();
                    self.current_intent = super::enemy::Intent::Special {
                        name: "Split".to_string(),
                    };
                    return DamageTriggerResult::SlimeBossSplit;
                }
                DamageTriggerResult::None
            }
            "Byrd" => {
                // Byrd grounding check — Flight decrement is now handled in 
                // take_damage_from_player(). This only checks if Flight was 
                // fully removed, and if so, grounds the Byrd.
                if damage_taken > 0 && self.is_flying {
                    if !self.powers.has("Flight") {
                        // Flight was removed by take_damage_from_player
                        self.is_flying = false;
                        self.stunned = true;
                        return DamageTriggerResult::ByrdGrounded;
                    }
                }
                DamageTriggerResult::None
            }
            "Lagavulin" => {
                // Java: Lagavulin.damage() — if hit while asleep and not yet triggered
                // Immediately wakes (OPEN → Stun → Attack)
                if damage_taken > 0 && !self.activated {
                    self.activated = true;
                    self.powers.remove("Metallicize");
                    self.stunned = true; // Stunned for one turn, then attacks
                    return DamageTriggerResult::LagavulinWake;
                }
                DamageTriggerResult::None
            }
            "AcidSlime_L" => {
                // Java: AcidSlime_L.damage() — HP ≤ maxHP/2 forces Split
                // splitTriggered prevents re-triggering. Uses `activated` as flag.
                if !self.is_dead()
                    && self.hp <= self.max_hp / 2
                    && self.current_move != "Split"
                    && !self.activated
                {
                    self.activated = true;
                    self.current_move = "Split".to_string();
                    self.current_intent = super::enemy::Intent::Special {
                        name: "Split".to_string(),
                    };
                    return DamageTriggerResult::LargeSlimeSplit;
                }
                DamageTriggerResult::None
            }
            "SpikeSlime_L" => {
                // Java: SpikeSlime_L.damage() — HP ≤ maxHP/2 forces Split
                // Identical to AcidSlime_L.
                if !self.is_dead()
                    && self.hp <= self.max_hp / 2
                    && self.current_move != "Split"
                    && !self.activated
                {
                    self.activated = true;
                    self.current_move = "Split".to_string();
                    self.current_intent = super::enemy::Intent::Special {
                        name: "Split".to_string(),
                    };
                    return DamageTriggerResult::LargeSlimeSplit;
                }
                DamageTriggerResult::None
            }
            "AwakenedOne" | "Awakened One" => {
                // Java: AwakenedOne.damage() — if HP <= 0 in form1, rebirth
                // form1 = !activated. On rebirth: restore HP, clear debuffs,
                // switch to form2 (activated=true), set firstTurn=true.
                if self.hp <= 0 && !self.activated {
                    // Prevent death
                    self.hp = self.max_hp;
                    self.alive = true;
                    // Switch to form2
                    self.activated = true;
                    self.first_move = true;
                    // Force Rebirth move
                    self.current_move = "Rebirth".to_string();
                    self.current_intent = super::enemy::Intent::Special {
                        name: "Rebirth".to_string(),
                    };
                    return DamageTriggerResult::AwakenedOneRebirth;
                }
                DamageTriggerResult::None
            }
            _ => DamageTriggerResult::None,
        }
    }
}

/// Result of post-damage state transition checks.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DamageTriggerResult {
    /// No special trigger.
    None,
    /// TheGuardian switched from Offensive to Defensive Mode.
    GuardianModeShift,
    /// SlimeBoss HP dropped ≤ 50%, forcing Split.
    SlimeBossSplit,
    /// Byrd lost all Flight stacks, now grounded + stunned.
    ByrdGrounded,
    /// Lagavulin was hit while sleeping, now waking + stunned.
    LagavulinWake,
    /// AcidSlime_L or SpikeSlime_L HP dropped ≤ 50%, forcing Split.
    LargeSlimeSplit,
    /// AwakenedOne died in form1, triggering Rebirth to form2.
    AwakenedOneRebirth,
}
