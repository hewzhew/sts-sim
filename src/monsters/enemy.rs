//! Enemy AI system for Slay the Spire simulator.
//!
//! Monster data is loaded from `monsters_verified.json` (v5 schema).
//! AI decisions are hardcoded in `hardcoded_ai.rs`.

use rand::Rng;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};

use crate::powers::PowerSet;

// ============================================================================
// Intent - The result of AI planning
// ============================================================================

/// What the enemy intends to do this turn.
/// This is the "interpreted" result, not just the move name.
#[derive(Debug, Clone, PartialEq, Default)]
pub enum Intent {
    /// Attack for damage
    Attack { damage: i32, times: i32 },
    /// Attack all enemies (for multi-target)
    AttackAll { damage: i32 },
    /// Defend/Block
    Defend { block: i32 },
    /// Apply a buff to self
    Buff { name: String, amount: i32 },
    /// Apply a debuff to player
    Debuff { name: String, amount: i32 },
    /// Attack and debuff combined
    AttackDebuff { damage: i32, debuff: String, amount: i32 },
    /// Attack and defend combined
    AttackDefend { damage: i32, block: i32 },
    /// Add status cards to the player's deck (e.g., Dazed, Burn, Wound, Slimed)
    AddCard { card: String, amount: i32, destination: String },
    /// Summon minions
    Summon { monster: String, count: i32 },
    /// Special/Unknown move
    Special { name: String },
    /// Sleeping/Inactive
    Sleep,
    /// Stunned (skip turn)
    Stunned,
    /// Escape from battle
    Escape,
    /// Unknown (fallback)
    #[default]
    Unknown,
}

// ============================================================================
// V5 JSON Schema Types — loaded from monsters_verified.json
// ============================================================================

/// A value that can be either a fixed integer or a {min, max} range.
/// Used for damage, HP, effect amounts, etc.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum V5Value {
    Fixed(i32),
    Range { min: i32, max: i32 },
}

impl V5Value {
    /// Get the fixed value, or the min of a range.
    pub fn base(&self) -> i32 {
        match self {
            V5Value::Fixed(v) => *v,
            V5Value::Range { min, .. } => *min,
        }
    }
    
    /// Roll a value within the range (or return fixed).
    pub fn roll<R: Rng>(&self, rng: &mut R) -> i32 {
        match self {
            V5Value::Fixed(v) => *v,
            V5Value::Range { min, max } => {
                if min == max { *min } else { rng.random_range(*min..=*max) }
            }
        }
    }
}

/// HP range as {min, max} object.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct V5HpRange {
    pub min: i32,
    pub max: i32,
}

impl V5HpRange {
    /// Roll HP within the range.
    pub fn roll<R: Rng>(&self, rng: &mut R) -> i32 {
        if self.min == self.max {
            self.min
        } else {
            rng.random_range(self.min..=self.max)
        }
    }
}

/// An effect in the v5 schema: {"id": "Weak", "amount": 2, "target": "player"}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct V5Effect {
    pub id: String,
    #[serde(default)]
    pub amount: Option<V5Value>,
    #[serde(default = "default_target_self")]
    pub target: String,
}

fn default_target_self() -> String { "self".to_string() }

/// A card action in the v5 schema: {"id": "Dazed", "amount": 2, "destination": "discard"}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct V5Card {
    pub id: String,
    pub amount: i32,
    pub destination: String,
}

/// A move definition in the v5 schema.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct V5Move {
    pub name: String,
    #[serde(default)]
    pub damage: Option<V5Value>,
    #[serde(default)]
    pub hits: Option<i32>,
    #[serde(default)]
    pub block: Option<i32>,
    #[serde(default)]
    pub effects: Option<Vec<V5Effect>>,
    #[serde(default)]
    pub cards: Option<Vec<V5Card>>,
}

/// Ascension override — partial overrides that layer on top of base values.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct V5AscOverride {
    #[serde(default)]
    pub hp: Option<V5HpRange>,
    #[serde(default)]
    pub moves: Option<HashMap<String, serde_json::Value>>,
    #[serde(default)]
    pub pre_battle: Option<Vec<V5Effect>>,
    #[serde(default)]
    pub end_turn_effects: Option<Vec<V5Effect>>,
}

/// Complete monster definition from v5 JSON schema.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonsterDefinition {
    /// Rust MonsterId enum variant name (e.g. "AcidSlime_L")
    pub id: String,
    /// Display name (e.g. "Acid Slime (L)")
    pub name: String,
    /// Java source ID (e.g. "AcidSlime_L")
    #[serde(default)]
    pub java_id: Option<String>,
    /// Monster type: "normal", "elite", "boss", "minion"
    #[serde(rename = "type")]
    pub monster_type: String,
    /// Act number
    pub act: i32,
    /// HP range
    pub hp: V5HpRange,
    /// Moves keyed by byte ID string ("1", "2", etc.)
    #[serde(default)]
    pub moves: HashMap<String, V5Move>,
    /// Pre-battle effects (Java's usePreBattleAction)
    #[serde(default)]
    pub pre_battle: Option<Vec<V5Effect>>,
    /// End-of-turn effects
    #[serde(default)]
    pub end_turn_effects: Option<Vec<V5Effect>>,
    /// Ascension overrides (cumulative)
    #[serde(default)]
    pub ascension: Option<HashMap<String, V5AscOverride>>,
    /// Notes (ignored by Rust, prefixed with _)
    #[serde(rename = "_notes", default)]
    pub notes: Option<String>,
}

// ============================================================================
// Monster State - Runtime state during combat
// ============================================================================

/// Runtime state for a monster during combat.
#[derive(Debug, Clone)]
pub struct MonsterState {
    /// Reference to the monster definition
    pub definition_id: String,
    /// Monster's display name
    pub name: String,
    /// Current HP
    pub hp: i32,
    /// Maximum HP
    pub max_hp: i32,
    /// Current block
    pub block: i32,
    /// Current turn number (1-indexed)
    pub turn: i32,
    /// History of moves used (most recent first)
    pub move_history: VecDeque<String>,
    /// Set of moves used at least once this combat
    pub moves_used: std::collections::HashSet<String>,
    /// Current phase (for multi-phase bosses)
    pub phase: i32,
    /// Charging counter (for Charging logic)
    pub charge_count: i32,
    /// Cycle index (for Cycle logic)
    pub cycle_index: usize,
    /// Is this monster the "middle" one (for Darkling)
    pub is_middle: bool,
    /// All buffs/debuffs using the unified Power system
    pub powers: PowerSet,
    /// Is the monster alive
    pub alive: bool,
    /// Is the monster stunned this turn
    pub stunned: bool,
    /// Current intent (what the monster will do this turn)
    pub current_intent: Intent,
    /// Current move name
    pub current_move: String,
    /// Buffs gained this turn (these don't trigger end-of-turn effects yet)
    pub buffs_gained_this_turn: std::collections::HashSet<String>,
    /// Whether this is an elite enemy (for PreservedInsect)
    pub is_elite: bool,

    // --- CommunicationMod Specific Extracted Variables ---
    /// Is the monster currently escaping
    pub is_escaping: bool,
    /// Whether Time Eater has used Haste (heal) / Champ half HP / AwakenedOne phase 1
    pub misc_bool: bool,
    /// Book of stabbing stabs / Spiker thorns / Louse bite damage
    pub misc_int: i32,
    /// Whether this is a boss enemy (for Pantograph)
    pub is_boss: bool,
    // === Hardcoded AI state fields ===
    /// Champ: HP threshold reached (triggers Anger + Execute priority)
    pub threshold_reached: bool,
    /// Champ: Turn counter for Taunt every 4 turns
    pub num_turns_champ: i32,
    /// Champ: Number of times Defensive Stance used (capped at 2)
    pub forge_times: i32,
    /// Byrd: Whether the Byrd is currently flying
    pub is_flying: bool,
    /// Generic first move flag (Byrd, SphericGuardian, Darkling)
    pub first_move: bool,
    /// SphericGuardian: second move flag
    pub second_move: bool,
    /// Hexaghost: Whether activated (first turn is Activate)
    pub activated: bool,
    /// Hexaghost: Orb active count (determines move sequence)
    pub orb_count: i32,
    /// TheGuardian: Whether in open (offensive) mode
    pub is_open: bool,
    /// TheGuardian: Damage threshold for mode shift
    pub dmg_threshold: i32,
    /// Spiker: thorns buff count for AI decision
    pub thorns_count: i32,
    /// Ascension level (needed for hardcoded AI decisions)
    pub ascension_level: i32,
    /// Lagavulin: idle turn counter (sleeps for 3 turns before waking)
    pub idle_count: i32,
    /// Lagavulin: tracks consecutive attacks before forced Siphon Soul
    pub debuff_turn_count: i32,
}

impl MonsterState {
    /// Create a new monster state from a definition.
    pub fn new<R: Rng>(def: &MonsterDefinition, rng: &mut R, ascension_level: i32) -> Self {
        // Determine HP from base or ascension override
        let hp_range = Self::resolve_hp(def, ascension_level);
        let hp = hp_range.roll(rng);
        let mut monster = Self {
            definition_id: def.id.clone(),
            name: def.name.clone(),
            hp,
            max_hp: hp,
            block: 0,
            turn: 0,
            move_history: VecDeque::with_capacity(8),
            moves_used: std::collections::HashSet::new(),
            phase: 1,
            charge_count: 0,
            cycle_index: 0,
            is_middle: false,
            powers: PowerSet::new(),
            alive: true,
            stunned: false,
            current_intent: Intent::Unknown,
            current_move: String::new(),
            buffs_gained_this_turn: std::collections::HashSet::new(),
            is_elite: def.monster_type == "elite",
            is_boss: def.monster_type == "boss",
            is_escaping: false,
            misc_bool: false,
            misc_int: 0,
            threshold_reached: false,
            num_turns_champ: 0,
            forge_times: 0,
            is_flying: true,
            first_move: true,
            second_move: true,
            activated: false,
            orb_count: 0,
            is_open: true,
            dmg_threshold: 30,
            thorns_count: 0,
            ascension_level,
            idle_count: 0,
            debuff_turn_count: 0,
        };
        
        // Apply pre-battle effects from v5 JSON
        let pre_battle = Self::resolve_pre_battle(def, ascension_level);
        if let Some(effects) = pre_battle {
            monster.apply_v5_pre_battle(&effects);
        }
        
        monster
    }
    
    /// Resolve HP range considering ascension overrides (cumulative).
    fn resolve_hp(def: &MonsterDefinition, ascension_level: i32) -> V5HpRange {
        let mut hp = def.hp.clone();
        if let Some(ref asc_map) = def.ascension {
            let mut keys: Vec<i32> = asc_map.keys().filter_map(|k| k.parse().ok()).collect();
            keys.sort();
            for k in keys {
                if ascension_level >= k {
                    if let Some(ovr) = asc_map.get(&k.to_string()) {
                        if let Some(ref new_hp) = ovr.hp {
                            hp = new_hp.clone();
                        }
                    }
                }
            }
        }
        hp
    }
    
    /// Resolve pre-battle effects considering ascension overrides.
    fn resolve_pre_battle(def: &MonsterDefinition, ascension_level: i32) -> Option<Vec<V5Effect>> {
        let mut result = def.pre_battle.clone();
        if let Some(ref asc_map) = def.ascension {
            let mut keys: Vec<i32> = asc_map.keys().filter_map(|k| k.parse().ok()).collect();
            keys.sort();
            for k in keys {
                if ascension_level >= k {
                    if let Some(ovr) = asc_map.get(&k.to_string()) {
                        if ovr.pre_battle.is_some() {
                            result = ovr.pre_battle.clone();
                        }
                    }
                }
            }
        }
        result
    }
    
    /// Apply v5 pre-battle effects to this monster.
    fn apply_v5_pre_battle(&mut self, effects: &[V5Effect]) {
        for eff in effects {
            let amount = eff.amount.as_ref().map(|v| v.base()).unwrap_or(1);
            match eff.id.as_str() {
                "Block" => {
                    self.block = amount;
                    game_log!("  ⚙️ {} starts with {} Block", self.name, amount);
                }
                _ => {
                    // Apply as a power
                    self.powers.apply(&eff.id, amount, None);
                    game_log!("  ⚙️ {} starts with {} {}", self.name, amount, eff.id);
                }
            }
        }
    }
    
    /// Create a simple test monster without loading from definition.
    pub fn new_simple(name: impl Into<String>, hp: i32) -> Self {
        let name_str = name.into();
        Self {
            definition_id: name_str.clone(),
            name: name_str,
            hp,
            max_hp: hp,
            block: 0,
            turn: 0,
            move_history: VecDeque::with_capacity(8),
            moves_used: std::collections::HashSet::new(),
            phase: 1,
            charge_count: 0,
            cycle_index: 0,
            is_middle: false,
            powers: PowerSet::new(),
            alive: true,
            stunned: false,
            current_intent: Intent::Unknown,
            current_move: String::new(),
            buffs_gained_this_turn: std::collections::HashSet::new(),
            is_elite: false,
            is_boss: false,
            is_escaping: false,
            misc_bool: false,
            misc_int: 0,
            threshold_reached: false,
            num_turns_champ: 0,
            forge_times: 0,
            is_flying: true,
            first_move: true,
            second_move: true,
            activated: false,
            orb_count: 0,
            is_open: true,
            dmg_threshold: 30,
            thorns_count: 0,
            ascension_level: 0,
            idle_count: 0,
            debuff_turn_count: 0,
        }
    }

    // ========================================================================
    // Power/Buff Helper Methods (Compatibility Layer)
    // ========================================================================

    /// Get Strength value (convenience method)
    #[inline]
    pub fn strength(&self) -> i32 {
        self.powers.get("Strength")
    }

    /// Add/apply a buff by name (convenience method)
    pub fn add_buff(&mut self, name: &str, amount: i32) {
        self.powers.apply(name, amount, None);
    }

    /// Get buff stacks by name (convenience method)
    #[inline]
    pub fn get_buff(&self, name: &str) -> i32 {
        self.powers.get(name)
    }

    /// Check if monster has a buff (convenience method)
    #[inline]
    pub fn has_buff(&self, name: &str) -> bool {
        self.powers.has(name)
    }

    // ========================================================================
    // Intent Helper Methods (for RL observation encoding)
    // ========================================================================

    /// Get intent information for RL encoding.
    ///
    /// Returns a tuple of (intent_type_id, damage_value):
    /// - Intent Type ID: 0=None/Unknown, 1=Attack, 2=Defend, 3=Buff, 4=Debuff, 5=Special
    /// - Damage Value: The damage amount for attack intents (0.0 for non-attacks)
    ///
    /// This method is optimized for speed (no allocations).
    #[inline]
    pub fn get_intent_info(&self) -> (u8, f32) {
        match &self.current_intent {
            Intent::Attack { damage, times } => (1, (*damage * *times) as f32),
            Intent::AttackAll { damage } => (1, *damage as f32),
            Intent::AttackDebuff { damage, .. } => (1, *damage as f32),
            Intent::AttackDefend { damage, .. } => (1, *damage as f32),
            Intent::Defend { .. } => (2, 0.0),
            Intent::Buff { .. } => (3, 0.0),
            Intent::Debuff { .. } => (4, 0.0),
            Intent::AddCard { .. } => (4, 0.0), // Treated as debuff for RL encoding
            Intent::Summon { .. } => (5, 0.0),
            Intent::Special { .. } => (5, 0.0),
            Intent::Sleep => (0, 0.0),
            Intent::Stunned => (0, 0.0),
            Intent::Escape => (5, 0.0),
            Intent::Unknown => (0, 0.0),
        }
    }

    /// Get the damage value from the current intent.
    /// Returns 0 if the intent is not an attack.
    #[inline]
    pub fn get_intent_damage(&self) -> i32 {
        match &self.current_intent {
            Intent::Attack { damage, times } => damage * times,
            Intent::AttackAll { damage } => *damage,
            Intent::AttackDebuff { damage, .. } => *damage,
            Intent::AttackDefend { damage, .. } => *damage,
            _ => 0,
        }
    }

    /// Get a human-readable intent type string (for debugging).
    pub fn get_intent_type_name(&self) -> &'static str {
        match &self.current_intent {
            Intent::Attack { .. } => "Attack",
            Intent::AttackAll { .. } => "AttackAll",
            Intent::AttackDebuff { .. } => "AttackDebuff",
            Intent::AttackDefend { .. } => "AttackDefend",
            Intent::Defend { .. } => "Defend",
            Intent::Buff { .. } => "Buff",
            Intent::Debuff { .. } => "Debuff",
            Intent::AddCard { .. } => "Debuff",
            Intent::Summon { .. } => "Summon",
            Intent::Special { .. } => "Special",
            Intent::Sleep => "Sleep",
            Intent::Stunned => "Stunned",
            Intent::Escape => "Escape",
            Intent::Unknown => "Unknown",
        }
    }

    // ========================================================================
    // Compatibility methods (matching old Enemy interface)
    // ========================================================================

    /// Alias for hp (for compatibility with old Enemy code)
    pub fn current_hp(&self) -> i32 {
        self.hp
    }
    
    /// Check if the monster is dead
    pub fn is_dead(&self) -> bool {
        !self.alive || self.hp <= 0
    }
    
    /// Take damage, accounting for block and intangible.
    /// Note: Vulnerable is NOT applied here — it's in calculate_card_damage() (Phase A).
    /// Verified: Java's AbstractCreature.damage() does not apply Vulnerable;
    ///           VulnerablePower.atDamageReceive() is called in AbstractCard.calculateCardDamage().
    pub fn take_damage(&mut self, mut damage: i32) -> i32 {
        // Apply intangible (reduce all damage to 1)
        if self.powers.has("Intangible") {
            damage = damage.min(1).max(0);
        }
        
        // Apply block first
        let blocked = damage.min(self.block);
        self.block -= blocked;
        let actual_damage = damage - blocked;
        
        self.hp = (self.hp - actual_damage).max(0);
        if self.hp <= 0 {
            self.alive = false;
        }
        // Post-damage state transitions (Guardian ModeShift, SlimeBoss Split, Byrd grounding)
        self.check_damage_triggers(actual_damage);
        actual_damage
    }
    
    /// Take damage from a PLAYER attack, with relic and power hooks applied.
    /// Java: AbstractMonster.damage() → relic.onAttackToChangeDamage() + power hooks
    /// 
    /// Returns `(actual_damage, pending_block)`:
    /// - `actual_damage`: HP actually lost by this enemy
    /// - `pending_block`: block to add AFTER all hits resolve (Java queues GainBlockAction)
    /// 
    /// The caller MUST apply `pending_block` after all hits of a multi-hit card are done.
    /// This matches Java's behavior where GainBlockAction is queued during onAttacked()
    /// and only resolves after the entire DamageAction completes.
    pub fn take_damage_from_player(&mut self, mut damage: i32, has_boot: bool) -> (i32, i32) {
        // NOTE: Intangible and Flight damage reduction are handled in
        // calculate_damage_hooked → atDamageFinalReceive, so the `damage`
        // parameter already has those applied. Do NOT apply them again here.
        
        // Apply block first (Java: decrementBlock)
        let blocked = damage.min(self.block);
        self.block -= blocked;
        let mut actual_damage = damage - blocked;
        
        // Boot: minimum 5 unblocked damage for player attacks
        if has_boot && actual_damage > 0 && actual_damage < 5 {
            game_log!("  👢 Boot: damage increased from {} to 5", actual_damage);
            actual_damage = 5;
        }
        
        // === onAttacked hooks (fire before HP loss) ===
        // In Java, GainBlockAction is QUEUED — it fires AFTER the entire DamageAction.
        // We accumulate pending_block and return it; caller applies after all hits.
        let mut pending_block = 0;
        
        // Curl Up: gain block on first attack, then remove power
        // CurlUpPower.java L34-42: if !triggered && damage < currentHP && damage > 0 && NORMAL
        if self.powers.has("Curl Up") && actual_damage > 0 && actual_damage < self.hp {
            let curl_amount = self.powers.get("Curl Up");
            pending_block += curl_amount;  // Queued — applied after all hits
            game_log!("  🐛 Curl Up: queued {} block (applied after all hits)", curl_amount);
            self.powers.remove("Curl Up");
        }
        
        // Malleable: gain block = amount, then increment amount
        // MalleablePower.java L60-71: if damage < currentHP && damage > 0 && NORMAL
        if self.powers.has("Malleable") && actual_damage > 0 && actual_damage < self.hp {
            let malleable_amount = self.powers.get("Malleable");
            pending_block += malleable_amount;  // Queued — applied after all hits
            game_log!("  🛡️ Malleable: queued {} block (next: {})", malleable_amount, malleable_amount + 1);
            self.powers.set("Malleable", malleable_amount + 1);
        }
        
        // Angry: gain Strength when attacked
        // AngryPower.java L31-35: onAttacked → addToBot(ApplyPower(Strength, this.amount))
        // Java: ApplyPowerAction.update() checks isDeadOrEscaped() → skips if dead
        // So Angry Strength gain only applies if the monster survives
        if self.powers.has("Angry") && actual_damage > 0 && self.hp > actual_damage {
            let angry_amount = self.powers.get("Angry");
            self.powers.apply("Strength", angry_amount, None);
            game_log!("  😡 Angry: gained {} Strength!", angry_amount);
        }

        
        self.hp = (self.hp - actual_damage).max(0);
        if self.hp <= 0 {
            self.alive = false;
        }
        
        // === wasHPLost hooks (fire after HP loss) ===
        
        // Plated Armor: decrement by 1 when taking unblocked damage
        // PlatedArmorPower.java L54-58: if damage > 0 && NORMAL && from enemy
        if self.powers.has("Plated Armor") && actual_damage > 0 {
            let current = self.powers.get("Plated Armor");
            if current > 1 {
                self.powers.set("Plated Armor", current - 1);
                game_log!("  🔩 Plated Armor: reduced to {}", current - 1);
            } else {
                self.powers.remove("Plated Armor");
                game_log!("  🔩 Plated Armor: removed!");
            }
        }
        
        // Flight: decrement by 1 per attack (Java: onAttacked → ReducePowerAction)
        // FlightPower.java L63-69: if damage > 0 && willLive && NORMAL
        if self.powers.has("Flight") && damage > 0 && self.hp > 0 {
            let current = self.powers.get("Flight");
            if current > 1 {
                self.powers.set("Flight", current - 1);
                game_log!("  🦅 Flight: reduced to {}", current - 1);
            } else {
                self.powers.remove("Flight");
                game_log!("  🦅 Flight: removed! (grounded)");
            }
        }
        
        // Post-damage state transitions (Guardian ModeShift, SlimeBoss Split, Byrd grounding)
        self.check_damage_triggers(actual_damage);
        (actual_damage, pending_block)
    }
    
    /// Apply a status effect (debuff) to this monster.
    /// Checks for Artifact first — if present, consumes 1 Artifact stack and blocks the debuff.
    /// Corresponds to: ApplyPowerAction.java lines 125-131
    pub fn apply_status(&mut self, status: &str, stacks: i32) -> bool {
        // Artifact only blocks debuffs, not buffs (Strength, Ritual, etc.)
        // In Java, the check is: powerToApply.type == PowerType.DEBUFF
        let is_debuff = matches!(status,
            "Vulnerable" | "Weak" | "Frail" | "Poison" | "Constricted"
            | "Entangled" | "NoDraw" | "NoBlock" | "Slow" | "Hex"
            | "Confused" | "DrawReduction" | "Choked" | "CorpseExplosion"
        );
        
        if is_debuff && self.powers.has("Artifact") {
            let artifact_stacks = self.powers.get("Artifact");
            if artifact_stacks > 0 {
                if artifact_stacks <= 1 {
                    self.powers.remove("Artifact");
                } else {
                    self.powers.apply("Artifact", -1, None);
                }
                game_log!("  \u{1f6e1}\u{fe0f} {} negated {} with Artifact! ({} stacks remaining)",
                    self.name, status, (artifact_stacks - 1).max(0));
                return false; // debuff was blocked
            }
        }
        
        let is_new = !self.powers.has(status);
        self.powers.apply(status, stacks, None);
        // Mark as gained this turn (for buffs like Ritual that shouldn't trigger immediately)
        if is_new {
            self.buffs_gained_this_turn.insert(status.to_string());
        }
        true // debuff was applied
    }
    
    /// Clear the "buffs gained this turn" tracking. Call at the end of each turn.
    pub fn clear_new_buff_tracking(&mut self) {
        self.buffs_gained_this_turn.clear();
    }
    
    /// Check if a buff was just gained this turn (and shouldn't trigger end-of-turn effects yet).
    pub fn is_buff_new_this_turn(&self, buff: &str) -> bool {
        self.buffs_gained_this_turn.contains(buff)
    }
    
    /// Remove stacks of a buff. If `all` is true, remove all stacks.
    /// Returns the number of stacks actually removed.
    pub fn remove_buff(&mut self, buff: &str, amount: i32, all: bool) -> i32 {
        if all {
            self.powers.remove(buff)
        } else {
            self.powers.remove_stacks(buff, amount)
        }
    }
    
    /// Gain block
    pub fn gain_block(&mut self, amount: i32) {
        self.block += amount.max(0);
    }
    
    /// Clear block at start of turn
    pub fn clear_block(&mut self) {
        self.block = 0;
    }
    
    /// Get status stacks (compatible with old Enemy.statuses)
    pub fn get_status(&self, status: &str) -> i32 {
        self.powers.get(status)
    }
    
    /// Get HP as a percentage (0-100).
    pub fn hp_percent(&self) -> i32 {
        if self.max_hp == 0 { 0 } else { (self.hp * 100) / self.max_hp }
    }
    
    /// Count consecutive uses of a move at the start of history.
    pub fn consecutive_uses(&self, move_name: &str) -> i32 {
        let mut count = 0;
        for m in &self.move_history {
            if m == move_name {
                count += 1;
            } else {
                break;
            }
        }
        count
    }
    
    /// Check if a move has been used this combat.
    pub fn has_used_move(&self, move_name: &str) -> bool {
        self.moves_used.contains(move_name)
    }
    
    /// Record a move being used.
    pub fn record_move(&mut self, move_name: &str) {
        self.move_history.push_front(move_name.to_string());
        self.moves_used.insert(move_name.to_string());
        // Keep history bounded
        if self.move_history.len() > 10 {
            self.move_history.pop_back();
        }
    }
    
    /// Get the last move used, if any.
    pub fn last_move(&self) -> Option<&str> {
        self.move_history.front().map(|s| s.as_str())
    }
    
    pub fn resolve_intent(&self, move_name: &str, def: &MonsterDefinition) -> Intent {
        let strength = self.strength();
        
        // Special states first
        match move_name {
            "Stunned" | "stunned" => return Intent::Stunned,
            "Sleeping" | "sleeping" | "sleep" => return Intent::Sleep,
            "Escape" | "escape" | "flee" => return Intent::Escape,
            _ => {}
        }
        
        // Find the move in v5 definition (search by name across all byte-ID entries)
        if let Some(move_def) = def.moves.values().find(|m| m.name == move_name) {
            let base_damage = move_def.damage.as_ref().map(|v| v.base()).unwrap_or(0);
            let has_damage = base_damage > 0;
            let has_block = move_def.block.unwrap_or(0) > 0;
            let effects = move_def.effects.as_deref().unwrap_or(&[]);
            let cards = move_def.cards.as_deref().unwrap_or(&[]);
            
            // Classify effects
            let buff_names = ["Strength", "Ritual", "Metallicize", "Angry", "CurlUp",
                "ModeShift", "SharpHide", "PlatedArmor", "Thorns", "Artifact",
                "Regrow", "Barricade", "Flight", "Intangible", "Explosive"];
            let debuff_names = ["Weak", "Vulnerable", "Frail", "Poison", "Constricted",
                "Entangled", "DrawReduction", "Hex"];
            
            let buff_eff = effects.iter().find(|e| {
                e.target == "self" && buff_names.iter().any(|b| e.id == *b)
            });
            let debuff_eff = effects.iter().find(|e| {
                e.target == "player" && (debuff_names.iter().any(|d| e.id == *d))
            });
            
            let total_damage = base_damage + strength;
            
            match (has_damage, has_block, debuff_eff, buff_eff) {
                // Attack + Debuff
                (true, _, Some(debuff), _) => {
                    Intent::AttackDebuff {
                        damage: total_damage,
                        debuff: debuff.id.clone(),
                        amount: debuff.amount.as_ref().map(|v| v.base()).unwrap_or(1),
                    }
                }
                // Attack + Block
                (true, true, _, _) => {
                    Intent::AttackDefend {
                        damage: total_damage,
                        block: move_def.block.unwrap_or(0),
                    }
                }
                // Pure Attack (no debuff, no buff)
                (true, false, None, None) => {
                    let times = move_def.hits.unwrap_or(1);
                    Intent::Attack { damage: total_damage, times }
                }
                // Attack + Buff (has damage, no block, no debuff, has buff)
                (true, false, None, Some(_)) => {
                    let times = move_def.hits.unwrap_or(1);
                    Intent::Attack { damage: total_damage, times }
                }
                // Pure Block
                (false, true, _, _) => {
                    Intent::Defend { block: move_def.block.unwrap_or(0) }
                }
                // No damage, no block — check cards, buffs, debuffs
                (false, false, _, _) => {
                    if let Some(card) = cards.first() {
                        Intent::AddCard {
                            card: card.id.clone(),
                            amount: card.amount,
                            destination: card.destination.clone(),
                        }
                    } else if let Some(buff) = buff_eff {
                        Intent::Buff {
                            name: buff.id.clone(),
                            amount: buff.amount.as_ref().map(|v| v.base()).unwrap_or(1),
                        }
                    } else if let Some(debuff) = debuff_eff {
                        Intent::Debuff {
                            name: debuff.id.clone(),
                            amount: debuff.amount.as_ref().map(|v| v.base()).unwrap_or(1),
                        }
                    } else {
                        Intent::Special { name: move_name.to_string() }
                    }
                }
            }
        } else {
            // Move not found — special intent
            Intent::Special { name: move_name.to_string() }
        }
    }
    
    /// Plan the next move: use hardcoded AI, then resolve intent from JSON data.
    pub fn plan_next_move<R: Rng>(
        &mut self,
        def: &MonsterDefinition,
        rng: &mut R,
        allies_alive: &[bool],
    ) {
        self.turn += 1;
        
        // Handle stunned state
        if self.stunned {
            self.stunned = false;
            self.current_move = "Stunned".to_string();
            self.current_intent = Intent::Stunned;
            return;
        }
        
        // All monsters use hardcoded AI
        if let Some(move_name) = self.hardcoded_get_move(rng, allies_alive) {
            self.record_move(&move_name);
            let intent = self.resolve_intent(&move_name, def);
            self.current_move = move_name;
            self.current_intent = intent;
        }
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use rand::SeedableRng;
    use rand_xoshiro::Xoshiro256StarStar;
    
    fn make_rng() -> Xoshiro256StarStar {
        Xoshiro256StarStar::seed_from_u64(12345)
    }
    
    #[test]
    fn test_v5_value_fixed() {
        let v: V5Value = serde_json::from_str("11").unwrap();
        assert_eq!(v.base(), 11);
        let mut rng = make_rng();
        assert_eq!(v.roll(&mut rng), 11);
    }
    
    #[test]
    fn test_v5_value_range() {
        let v: V5Value = serde_json::from_str(r#"{"min": 5, "max": 7}"#).unwrap();
        assert_eq!(v.base(), 5);
        let mut rng = make_rng();
        let rolled = v.roll(&mut rng);
        assert!(rolled >= 5 && rolled <= 7, "rolled {} not in [5,7]", rolled);
    }
    
    #[test]
    fn test_v5_hp_range() {
        let hp = V5HpRange { min: 48, max: 54 };
        let mut rng = make_rng();
        let rolled = hp.roll(&mut rng);
        assert!(rolled >= 48 && rolled <= 54);
    }
    
    #[test]
    fn test_v5_monster_def_deserialize() {
        let json = r#"{
            "id": "Cultist",
            "name": "Cultist",
            "java_id": "Cultist",
            "type": "normal",
            "act": 1,
            "hp": {"min": 48, "max": 54},
            "moves": {
                "3": {"name": "Incantation", "effects": [{"id": "Ritual", "amount": 3, "target": "self"}]},
                "1": {"name": "Dark Strike", "damage": 6}
            },
            "pre_battle": null
        }"#;
        let def: MonsterDefinition = serde_json::from_str(json).unwrap();
        assert_eq!(def.id, "Cultist");
        assert_eq!(def.name, "Cultist");
        assert_eq!(def.monster_type, "normal");
        assert_eq!(def.act, 1);
        assert_eq!(def.hp.min, 48);
        assert_eq!(def.hp.max, 54);
        assert_eq!(def.moves.len(), 2);
        let dark_strike = def.moves.get("1").unwrap();
        assert_eq!(dark_strike.name, "Dark Strike");
        assert_eq!(dark_strike.damage.as_ref().unwrap().base(), 6);
    }
    
    #[test]
    fn test_resolve_intent_from_v5() {
        let mut moves = HashMap::new();
        moves.insert("1".to_string(), V5Move {
            name: "Chomp".to_string(),
            damage: Some(V5Value::Fixed(11)),
            hits: None,
            block: None,
            effects: None,
            cards: None,
        });
        moves.insert("2".to_string(), V5Move {
            name: "Bellow".to_string(),
            damage: None,
            hits: None,
            block: None,
            effects: Some(vec![V5Effect {
                id: "Strength".to_string(),
                amount: Some(V5Value::Fixed(3)),
                target: "self".to_string(),
            }]),
            cards: None,
        });
        
        let def = MonsterDefinition {
            id: "JawWorm".to_string(),
            name: "Jaw Worm".to_string(),
            java_id: Some("JawWorm".to_string()),
            monster_type: "normal".to_string(),
            act: 1,
            hp: V5HpRange { min: 40, max: 44 },
            moves,
            pre_battle: None,
            end_turn_effects: None,
            ascension: None,
            notes: None,
        };
        
        let state = MonsterState::new_simple("Jaw Worm", 42);
        
        // Test attack intent
        let intent = state.resolve_intent("Chomp", &def);
        assert!(matches!(intent, Intent::Attack { damage: 11, times: 1 }));
        
        // Test buff intent
        let intent2 = state.resolve_intent("Bellow", &def);
        assert!(matches!(intent2, Intent::Buff { .. }));
    }
    
    #[test]
    fn test_history_constraint() {
        let mut state = MonsterState {
            definition_id: "Test".to_string(),
            name: "Test Monster".to_string(),
            hp: 100,
            max_hp: 100,
            block: 0,
            turn: 0,
            move_history: VecDeque::new(),
            moves_used: std::collections::HashSet::new(),
            phase: 1,
            charge_count: 0,
            cycle_index: 0,
            is_middle: false,
            powers: PowerSet::new(),
            buffs_gained_this_turn: std::collections::HashSet::new(),
            alive: true,
            stunned: false,
            current_intent: Intent::Unknown,
            current_move: String::new(),
            is_elite: false,
            is_boss: false,
            is_escaping: false,
            misc_bool: false,
            misc_int: 0,
            threshold_reached: false,
            num_turns_champ: 0,
            forge_times: 0,
            is_flying: true,
            first_move: true,
            second_move: true,
            activated: false,
            orb_count: 0,
            is_open: true,
            dmg_threshold: 30,
            thorns_count: 0,
            ascension_level: 0,
            idle_count: 0,
            debuff_turn_count: 0,
        };
        
        // Record same move twice
        state.record_move("Tackle");
        state.record_move("Tackle");
        
        assert_eq!(state.consecutive_uses("Tackle"), 2);
        assert_eq!(state.consecutive_uses("Lick"), 0);
        
        // After a different move, consecutive resets
        state.record_move("Lick");
        assert_eq!(state.consecutive_uses("Tackle"), 0);
        assert_eq!(state.consecutive_uses("Lick"), 1);
    }
}
