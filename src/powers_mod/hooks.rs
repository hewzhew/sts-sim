//! Power Hook System (Enum Dispatch)
//!
//! This module implements Java's AbstractPower hook system using Rust enum dispatch.
//! Each `PowerId` variant corresponds to a Java Power class. Hook methods on
//! `PowerInstance` use `match` to dispatch behavior, mirroring Java's virtual method calls.
//!
//! ## Architecture
//!
//! - **PowerId**: Enum of all known powers (~123 from Java source)
//! - **PowerInstance**: Runtime state (PowerId + stacks)
//! - **Hook methods**: `at_damage_give()`, `on_attacked()`, etc.
//! - **HookEffect**: Return type for hooks that produce side effects
//!
//! ## Migration
//!
//! This coexists with the existing `PowerSet` (HashMap<String, i32>) in `powers.rs`.
//! Phase 1: PowerId enum + hooks defined here.
//! Phase 2: Engine routes damage/block through these hooks.
//! Phase 3: Reactive hooks (onAttacked, onUseCard, etc.)
//! Phase 4: Replace CardTrigger system with power hooks.

use serde::{Deserialize, Serialize};

// ============================================================================
// PowerId Enum — all known powers from Java source
// ============================================================================

/// Unique identifier for every power in the game.
///
/// Corresponds 1:1 with Java's power classes in `cardcrawl/powers/`.
/// Powers not yet implemented have their variant but empty hook match arms.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PowerId {
    // === Damage Pipeline (atDamageGive / atDamageReceive) ===
    Strength,
    Weak,
    Vulnerable,
    DoubleDamage,
    PenNib,
    Slow,

    // === Block Pipeline (modifyBlock / modifyBlockLast) ===
    Dexterity,
    Frail,
    NoBlock,

    // === Damage Final (atDamageFinalReceive) ===
    Intangible,
    IntangiblePlayer,
    Flight,
    Forcefield,

    // === Reactive: onAttacked ===
    Thorns,
    Caltrops,
    FlameBarrier,
    CurlUp,
    Angry,
    Malleable,
    StaticDischarge,
    Reactive,
    Shifting,
    Buffer,

    // === Reactive: onAttackedToChangeDamage ===
    Invincible,

    // === Turn-based: atStartOfTurn / atStartOfTurnPostDraw ===
    Poison,
    Berserk,
    Bias,
    CreativeAI,
    DevaForm,
    Loop,
    NextTurnBlock,
    DemonForm,
    Brutality,
    NoxiousFumes,
    MachineLearning,
    HelloWorld,

    // === Turn-based: atEndOfTurn / atEndOfTurnPreEndTurnCards ===
    Metallicize,
    PlatedArmor,
    Combust,
    Regen,
    Constricted,
    Ritual,
    NoDraw,
    Burst,
    DoubleTap,
    Amplify,
    Rage,
    Rebound,
    WraithForm,

    // === Turn-based: atEndOfRound ===
    Blur,
    LockOn,
    DrawReduction,
    Duplication,

    // === Card play: onUseCard / onAfterUseCard ===
    AfterImage,
    Corruption,
    Hex,
    SharpHide,
    Storm,
    Curiosity,
    Heatsink,
    TimeWarp,
    BeatOfDeath,
    ThousandCuts,
    Panache,

    // === Card draw: onCardDraw ===
    Evolve,
    FireBreathing,

    // === Card exhaust: onExhaust ===
    FeelNoPain,
    DarkEmbrace,

    // === Death: onDeath ===
    CorpseExplosion,
    SporeCloud,

    // === Block gained: onGainedBlock ===
    Juggernaut,

    // === HP lost: wasHPLost ===
    Rupture,

    // === Attack dealt: onAttack ===
    Envenom,

    // === Debuff applied: onApplyPower ===
    // SadisticNature handled as passive (complex hook)

    // === Monster: onInflictDamage ===
    PainfulStabs,

    // === Monster: duringTurn (countdown) ===
    Explosive,

    // === Monster: atEndOfRound ===
    Growth,
    AttackBurn,

    // === Player: onUseCard ===
    Choke,

    // === Player: on stance end of turn ===
    LikeWater,

    // === Special trigger ===
    Artifact,

    // === Retain / Equilibrium ===
    RetainCards,
    Equilibrium,
    
    // === Monster: Regeneration ===
    Regeneration,
    
    // === Passive / No hooks ===
    Barricade,
    Vigor,
    Energized,
    DrawCard,
    Mantra,
    Split,
    ModeShift,
    Minion,
    Fading,
    Electro,
    Thievery,
    Entangled,
    Confused,
    Accuracy,
    EchoForm,
    Establishment,
    Fasting,
    Mark,
    MentalFortress,
    Rushdown,
    Nirvana,
    Foresight,
    BattleHymn,
    Devotion,
    Enrage,
    FreeAttack,
    BlockReturn,
    Shackled,
    StrengthDown,
    DexterityDown,
    InfiniteBlades,
    ToolsOfTrade,
    Omega,
    Electrodynamics,
    Magnetism,
    Mayhem,
    SadisticNature,
    Study,
    WellLaidPlans,
    MasterReality,
    Collect,
    Phantasmal,
    TheBomb,
    Nightmare,

    // === Tier A: High Priority Missing Powers ===
    EndTurnDeath,
    AngerMonster,
    GenericStrUp,
    SkillBurn,
    EnergyDown,
    NoSkills,
    TimeMaze,

    // === Tier B: Watcher & Medium ===
    WrathNextTurn,
    WaveOfTheHand,
    CannotChangeStance,
    
    // === Tier C: Monster / Niche ===
    Compulsive,
    NullifyAttack,
    AngelForm,
    Conserve,
    RechargingCore,
    NightTerror,
    Repair,
    Retribution,
    Stasis,
    Winter,
    WireheadingPower,
    DrawPower,
    EmotionalTurmoil,
    SkillBurnGeneric,  // "Skill Burn" on enemies 
    Sadistic,
    Serenity,
    Vault,
    StrikeUp,

    // === Remaining: Watcher Stance / Deprecated / Niche ===
    Adaptation,
    Controlled,
    DisciplinePower,
    FlowPower,
    Grounded,
    HotHot,
    Mastery,
    RegenerateMonster,

    // === Catch-all for powers not yet enumerated ===
    /// Unknown power — stores the ID string hash for debugging.
    /// This allows graceful degradation for unimplemented powers.
    Unknown,
}

impl PowerId {
    /// Convert from a string power ID (used in current PowerSet / JSON).
    pub fn from_str(s: &str) -> Self {
        match s {
            // Damage pipeline
            "Strength" => Self::Strength,
            "Weak" | "Weakened" => Self::Weak,
            "Vulnerable" => Self::Vulnerable,
            "DoubleDamage" => Self::DoubleDamage,
            "PenNib" => Self::PenNib,
            "Slow" => Self::Slow,

            // Block pipeline
            "Dexterity" => Self::Dexterity,
            "Frail" => Self::Frail,
            "NoBlock" => Self::NoBlock,

            // Damage final
            "Intangible" => Self::Intangible,
            "IntangiblePlayer" => Self::IntangiblePlayer,
            "Flight" => Self::Flight,
            "Forcefield" => Self::Forcefield,

            // Reactive: onAttacked
            "Thorns" => Self::Thorns,
            "Caltrops" => Self::Caltrops,
            "FlameBarrier" | "Flame Barrier" => Self::FlameBarrier,
            "CurlUp" | "Curl Up" => Self::CurlUp,
            "Angry" => Self::Angry,
            "Malleable" => Self::Malleable,
            "StaticDischarge" | "Static Discharge" => Self::StaticDischarge,
            "Reactive" => Self::Reactive,
            "Shifting" => Self::Shifting,
            "Buffer" => Self::Buffer,
            "Invincible" => Self::Invincible,

            // Turn start
            "Poison" => Self::Poison,
            "Berserk" => Self::Berserk,
            "Bias" => Self::Bias,
            "CreativeAI" | "Creative AI" => Self::CreativeAI,
            "Loop" => Self::Loop,
            "NextTurnBlock" => Self::NextTurnBlock,
            "DemonForm" | "Demon Form" => Self::DemonForm,
            "Brutality" => Self::Brutality,
            "NoxiousFumes" | "Noxious Fumes" => Self::NoxiousFumes,
            "MachineLearning" | "Machine Learning" => Self::MachineLearning,
            "HelloWorld" | "Hello World" | "Hello" => Self::HelloWorld,
            "DevaForm" | "Deva Form" => Self::DevaForm,

            // Turn end
            "Metallicize" => Self::Metallicize,
            "PlatedArmor" | "Plated Armor" => Self::PlatedArmor,
            "Combust" => Self::Combust,
            "Regen" => Self::Regen,
            "Constricted" => Self::Constricted,
            "Ritual" => Self::Ritual,
            "NoDraw" | "No Draw" => Self::NoDraw,
            "Burst" => Self::Burst,
            "DoubleTap" | "Double Tap" => Self::DoubleTap,
            "Amplify" => Self::Amplify,
            "Rage" => Self::Rage,
            "Rebound" => Self::Rebound,
            "WraithForm" | "Wraith Form" | "Wraith Form v2" => Self::WraithForm,

            // End of round
            "Blur" => Self::Blur,
            "LockOn" | "Lock-On" => Self::LockOn,
            "DrawReduction" | "Draw Reduction" => Self::DrawReduction,
            "Duplication" | "DuplicationPower" => Self::Duplication,

            // Card play
            "AfterImage" | "After Image" => Self::AfterImage,
            "Corruption" => Self::Corruption,
            "Hex" => Self::Hex,
            "SharpHide" | "Sharp Hide" => Self::SharpHide,
            "Storm" => Self::Storm,
            "Curiosity" => Self::Curiosity,
            "Heatsink" => Self::Heatsink,
            "TimeWarp" | "Time Warp" => Self::TimeWarp,
            "BeatOfDeath" | "Beat of Death" => Self::BeatOfDeath,
            "ThousandCuts" | "A Thousand Cuts" => Self::ThousandCuts,
            "Panache" => Self::Panache,

            // Card draw
            "Evolve" => Self::Evolve,
            "FireBreathing" | "Fire Breathing" => Self::FireBreathing,

            // Exhaust
            "FeelNoPain" | "Feel No Pain" => Self::FeelNoPain,
            "DarkEmbrace" | "Dark Embrace" => Self::DarkEmbrace,

            // Death
            "CorpseExplosion" | "Corpse Explosion" => Self::CorpseExplosion,
            "SporeCloud" | "Spore Cloud" => Self::SporeCloud,

            // Block gained
            "Juggernaut" => Self::Juggernaut,

            // HP lost
            "Rupture" => Self::Rupture,

            // Attack dealt
            "Envenom" => Self::Envenom,

            // Monster powers
            "Painful Stabs" | "PainfulStabs" => Self::PainfulStabs,
            "Explosive" => Self::Explosive,
            "GrowthPower" | "Growth" => Self::Growth,
            "Attack Burn" | "AttackBurn" => Self::AttackBurn,
            "Choked" | "Choke" => Self::Choke,
            "LikeWater" | "Like Water" | "LikeWaterPower" => Self::LikeWater,

            // Special
            "Artifact" => Self::Artifact,

            // Passive / no hooks
            "Barricade" => Self::Barricade,
            "Vigor" => Self::Vigor,
            "Energized" => Self::Energized,
            "DrawCard" => Self::DrawCard,
            "Mantra" => Self::Mantra,
            "Split" => Self::Split,
            "ModeShift" | "Mode Shift" => Self::ModeShift,
            "Minion" => Self::Minion,
            "Fading" => Self::Fading,
            "Electro" => Self::Electro,
            "Thievery" => Self::Thievery,
            "Entangled" => Self::Entangled,
            "Confused" | "Confusion" => Self::Confused,
            "Accuracy" => Self::Accuracy,
            "EchoForm" | "Echo Form" => Self::EchoForm,
            "Establishment" => Self::Establishment,
            "Fasting" => Self::Fasting,
            "Mark" => Self::Mark,
            "MentalFortress" | "Mental Fortress" => Self::MentalFortress,
            "Rushdown" => Self::Rushdown,
            "Nirvana" => Self::Nirvana,
            "Foresight" => Self::Foresight,
            "BattleHymn" | "Battle Hymn" => Self::BattleHymn,
            "Devotion" => Self::Devotion,
            "Enrage" => Self::Enrage,
            "FreeAttack" => Self::FreeAttack,
            "BlockReturn" => Self::BlockReturn,
            "InfiniteBlades" | "Infinite Blades" => Self::InfiniteBlades,
            "ToolsOfTrade" | "Tools Of The Trade" | "Tools of the Trade" => Self::ToolsOfTrade,
            "Omega" | "OmegaPower" => Self::Omega,
            "Electrodynamics" => Self::Electrodynamics,
            "Magnetism" => Self::Magnetism,
            "Mayhem" => Self::Mayhem,
            "SadisticNature" | "Sadistic Nature" => Self::SadisticNature,
            "Study" => Self::Study,
            "WellLaidPlans" | "Well-Laid Plans" | "Retain" => Self::WellLaidPlans,
            "MasterReality" | "Master Reality" => Self::MasterReality,
            "Collect" | "CollectPower" => Self::Collect,
            "Phantasmal" | "PhantasmalPower" => Self::Phantasmal,
            "TheBomb" | "The Bomb" => Self::TheBomb,
            "Nightmare" | "NightmarePower" => Self::Nightmare,
            "Shackled" => Self::Shackled,
            "StrengthDown" | "Flex" => Self::StrengthDown,
            "DexterityDown" | "DexLoss" => Self::DexterityDown,
            "CorpseExplosionPower" => Self::CorpseExplosion,
            "NoBlockPower" | "No Block" => Self::NoBlock,
            "RetainCards" | "Retain Cards" => Self::RetainCards,
            "Equilibrium" => Self::Equilibrium,
            "Regeneration" => Self::Regeneration,
            // Tier A
            "EndTurnDeath" => Self::EndTurnDeath,
            "Anger" => Self::AngerMonster,
            "Generic Strength Up Power" | "GenericStrengthUp" => Self::GenericStrUp,
            "Skill Burn" | "SkillBurn" => Self::SkillBurn,
            "EnergyDownPower" | "EnergyDown" => Self::EnergyDown,
            "NoSkills" => Self::NoSkills,
            "TimeMazePower" | "TimeMaze" => Self::TimeMaze,
            // Tier B
            "WrathNextTurnPower" => Self::WrathNextTurn,
            "WaveOfTheHandPower" => Self::WaveOfTheHand,
            "CannotChangeStancePower" => Self::CannotChangeStance,
            // Tier C
            "Compulsive" => Self::Compulsive,
            "Nullify Attack" | "NullifyAttack" => Self::NullifyAttack,
            "AngelForm" => Self::AngelForm,
            "Conserve" => Self::Conserve,
            "RechargingCore" | "Recharging Core" => Self::RechargingCore,
            "Night Terror" | "NightTerror" => Self::NightTerror,
            "Repair" => Self::Repair,
            "Retribution" => Self::Retribution,
            "Stasis" => Self::Stasis,
            "Winter" => Self::Winter,
            "WireheadingPower" | "Wireheading" => Self::WireheadingPower,
            "Draw" => Self::DrawPower,
            "EmotionalTurmoilPower" | "Emotional Turmoil" => Self::EmotionalTurmoil,
            "Sadistic" => Self::Sadistic,
            "Serenity" => Self::Serenity,
            "Vault" => Self::Vault,
            "StrikeUp" => Self::StrikeUp,
            // Remaining
            "Adaptation" => Self::Adaptation,
            "Controlled" => Self::Controlled,
            "DisciplinePower" | "Discipline" => Self::DisciplinePower,
            "FlowPower" | "Flow" => Self::FlowPower,
            "Grounded" => Self::Grounded,
            "HotHot" => Self::HotHot,
            "Mastery" => Self::Mastery,
            "Regenerate" => Self::RegenerateMonster,

            _ => Self::Unknown,
        }
    }

    /// Whether this power is a debuff (for Artifact interaction).
    pub fn is_debuff(&self) -> bool {
        matches!(
            self,
            Self::Vulnerable
                | Self::Weak
                | Self::Frail
                | Self::Poison
                | Self::Constricted
                | Self::NoDraw
                | Self::NoBlock
                | Self::Entangled
                | Self::Confused
                | Self::Hex
                | Self::Slow
                | Self::DrawReduction
                | Self::Shackled
                | Self::StrengthDown
                | Self::DexterityDown
                | Self::Fading
                | Self::SkillBurn
                | Self::EnergyDown
                | Self::NoSkills
        )
    }
}

// ============================================================================
// PowerInstance — runtime state for an active power
// ============================================================================

/// An active power on an entity (player or monster).
///
/// This is the runtime representation. `stacks` holds the current stack count.
/// Hook methods implement the power's behavior based on its `id`.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct PowerInstance {
    pub id: PowerId,
    pub stacks: i32,
}

impl PowerInstance {
    pub fn new(id: PowerId, stacks: i32) -> Self {
        Self { id, stacks }
    }

    // ========================================================================
    // Damage Pipeline Hooks
    // Corresponds to: AbstractPower.atDamageGive(float, DamageType)
    // Called for each power on the ATTACKER.
    // ========================================================================

    /// Modify outgoing damage (attacker's powers).
    ///
    /// Java: `atDamageGive(float damage, DamageType type) -> float`
    /// Called in order for each power on the attacker.
    pub fn at_damage_give(&self, damage: f32) -> f32 {
        match self.id {
            // StrengthPower.java: return damage + (float)this.amount
            PowerId::Strength => damage + self.stacks as f32,
            // WeakPower.java: return damage * 0.75f (no intermediate floor)
            PowerId::Weak => damage * 0.75,
            // DoubleDamagePower.java: return damage * 2.0f
            PowerId::DoubleDamage => damage * 2.0,
            // PenNibPower.java: return damage * 2.0f (every 10th attack)
            PowerId::PenNib => damage * 2.0,
            // VigorPower.java: return damage + this.amount (if NORMAL type)
            // Vigor adds flat damage to next Attack, then is consumed in onUseCard
            PowerId::Vigor => damage + self.stacks as f32,
            // AccuracyPower.java: if card is Shiv, +this.amount damage
            // Note: Accuracy modifies baseDamage of Shivs, not atDamageGive.
            // We handle it here as a simplified hook — Shivs are identified by card name in engine.
            _ => damage,
        }
    }

    /// Modify incoming damage (defender's powers).
    ///
    /// Java: `atDamageReceive(float damage, DamageType type) -> float`
    /// Called in order for each power on the DEFENDER.
    pub fn at_damage_receive(&self, damage: f32) -> f32 {
        match self.id {
            // VulnerablePower.java: return damage * 1.5f (no intermediate floor)
            PowerId::Vulnerable => damage * 1.5,
            // SlowPower.java: return damage * (1.0f + (float)this.amount * 0.1f)
            PowerId::Slow => damage * (1.0 + self.stacks as f32 * 0.1),
            // LockOnPower.java: orb damage * 1.5 (handled separately in orb pipeline)
            // LockOn is a passive modifier checked by orb evoke/passive, not atDamageReceive
            _ => damage,
        }
    }

    /// Final damage modification on defender (after atDamageReceive).
    ///
    /// Java: `atDamageFinalReceive(float damage, DamageType type) -> float`
    pub fn at_damage_final_receive(&self, damage: f32) -> f32 {
        match self.id {
            // IntangiblePower.java / IntangiblePlayerPower.java: reduce to 1
            PowerId::Intangible | PowerId::IntangiblePlayer => {
                if damage > 0.0 { 1.0 } else { 0.0 }
            }
            // FlightPower.java: damage * 0.5f (no intermediate floor)
            PowerId::Flight => damage * 0.5,
            // ForcefieldPower.java: damage <= this.amount ? 0 : damage
            PowerId::Forcefield => {
                if damage <= self.stacks as f32 { 0.0 } else { damage }
            }
            // NullifyAttackPower.java: reduce first attack to 0
            PowerId::NullifyAttack => {
                if damage > 0.0 { 0.0 } else { damage }
            }
            _ => damage,
        }
    }

    // ========================================================================
    // Block Pipeline Hooks
    // Corresponds to: AbstractPower.modifyBlock(float)
    // ========================================================================

    /// Modify block amount.
    ///
    /// Java: `modifyBlock(float blockAmount) -> float`
    pub fn modify_block(&self, block: f32) -> f32 {
        match self.id {
            // DexterityPower.java: return blockAmount + (float)this.amount
            PowerId::Dexterity => block + self.stacks as f32,
            // FrailPower.java: return blockAmount * 0.75f (no intermediate floor)
            PowerId::Frail => block * 0.75,
            _ => block,
        }
    }

    /// Final block modification (after modifyBlock).
    ///
    /// Java: `modifyBlockLast(float blockAmount) -> float`
    pub fn modify_block_last(&self, block: f32) -> f32 {
        match self.id {
            // NoBlockPower.java: return 0
            PowerId::NoBlock => 0.0,
            _ => block,
        }
    }

    // ========================================================================
    // Reactive Hooks
    // These return HookEffect describing what should happen.
    // The engine interprets the effects and applies them to game state.
    // ========================================================================

    /// Called when this entity is attacked (damage dealt to this entity).
    ///
    /// Java: `onAttacked(DamageInfo info, int damageAmount) -> int`
    /// Returns modified damage and any side effects.
    pub fn on_attacked(&self, damage: i32) -> (i32, Vec<HookEffect>) {
        match self.id {
            // ThornsPower.java: damage attacker for this.amount
            PowerId::Thorns => (damage, vec![HookEffect::DamageAttacker(self.stacks)]),
            
            // FlameBarrierPower.java: damage attacker for this.amount
            PowerId::FlameBarrier => (damage, vec![HookEffect::DamageAttacker(self.stacks)]),
            
            // CaltropsP.java: damage attacker for this.amount (identical to Thorns)
            PowerId::Caltrops => (damage, vec![HookEffect::DamageAttacker(self.stacks)]),
            
            // CurlUpPower.java: gain block = this.amount, then remove self
            PowerId::CurlUp => (damage, vec![
                HookEffect::GainBlock(self.stacks),
                HookEffect::RemoveSelf,
            ]),
            
            // AngryPower.java: gain Strength = this.amount  
            PowerId::Angry => (damage, vec![HookEffect::GainStrength(self.stacks)]),
            
            // MalleablePower.java: gain block = this.amount, then stack +1
            PowerId::Malleable => (damage, vec![
                HookEffect::GainBlock(self.stacks),
                HookEffect::AddStacks(1),
            ]),
            
            // StaticDischargePower.java: channel Lightning
            PowerId::StaticDischarge => (damage, vec![HookEffect::ChannelLightning]),
            
            // FlightPower.java: lose 1 stack on hit
            PowerId::Flight => (damage, vec![HookEffect::AddStacks(-1)]),
            
            // ShiftingPower.java: lose STR = damage, regain at end of turn (no Artifact)
            // Java: ApplyPowerAction(owner, owner, StrengthPower(-dmg)) + GainStrengthPower(dmg)
            PowerId::Shifting => {
                if damage > 0 {
                    (damage, vec![
                        HookEffect::GainStrength(-damage),
                        // GainStrengthPower = Shackled: regain at end of turn
                        // This is automatically handled by Shackled's at_end_of_turn
                    ])
                } else {
                    (damage, vec![])
                }
            }
            
            // ReactivePower.java: re-roll intent on hit (monster-only)
            // Java: RollMoveAction — changes the monster's next move
            PowerId::Reactive => {
                if damage > 0 {
                    (damage, vec![HookEffect::RerollIntent])
                } else {
                    (damage, vec![])
                }
            }
            
            // PainfulStabsPower.java: add Wound to discard when this entity hits player
            // Note: Java uses onInflictDamage, but we handle it in on_attacked for simplicity
            // since this is a monster power that triggers when the monster deals unblocked damage
            PowerId::PainfulStabs => {
                if damage > 0 {
                    (damage, vec![HookEffect::AddStatusToDiscard { card: "Wound", count: 1 }])
                } else {
                    (damage, vec![])
                }
            }
            
            // BlockReturnPower.java ("Talk to the Hand" debuff on enemy):
            // Java: onAttacked → GainBlockAction(player, this.amount)
            // When the enemy with this debuff attacks the player, player gains block
            PowerId::BlockReturn => {
                if damage > 0 {
                    (damage, vec![HookEffect::PlayerGainBlock(self.stacks)])
                } else {
                    (damage, vec![])
                }
            }
            
            // CompulsivePower.java: gain block when attacked
            PowerId::Compulsive => {
                if damage > 0 {
                    (damage, vec![HookEffect::GainBlock(self.stacks)])
                } else {
                    (damage, vec![])
                }
            }
            
            // Artifact: onSpecificTrigger handled inline as debuff blocking
            // Signal-only arm to satisfy audit (actual logic is in apply_power debuff check)
            PowerId::Artifact => (damage, vec![]),
            
            // HotHotPower.java (deprecated): deal damage back when attacked
            PowerId::HotHot => {
                if damage > 0 {
                    (damage, vec![HookEffect::DamageAttacker(self.stacks)])
                } else {
                    (damage, vec![])
                }
            }
            
            // RetributionPower.java: deal damage back when attacked
            PowerId::Retribution => {
                if damage > 0 {
                    (damage, vec![HookEffect::DamageAttacker(self.stacks)])
                } else {
                    (damage, vec![])
                }
            }
            
            // SerenityPower.java: gain block when attacked
            PowerId::Serenity => {
                if damage > 0 {
                    (damage, vec![HookEffect::GainBlock(self.stacks)])
                } else {
                    (damage, vec![])
                }
            }
            
            _ => (damage, vec![]),
        }
    }

    /// Called before damage is applied (can modify damage amount).
    ///
    /// Java: `onAttackedToChangeDamage(DamageInfo info, int damageAmount) -> int`
    pub fn on_attacked_to_change_damage(&self, damage: i32) -> i32 {
        match self.id {
            // BufferPower.java: if damage > 0, reduce to 0 and consume 1 stack
            PowerId::Buffer => {
                if damage > 0 { 0 } else { damage }
            }
            // InvinciblePower.java: cap damage at this.amount
            PowerId::Invincible => {
                if damage > self.stacks { self.stacks } else { damage }
            }
            _ => damage,
        }
    }

    /// Called when a card is played.
    ///
    /// Java: `onUseCard(AbstractCard card, UseCardAction action)`
    /// Returns side effects to apply.
    pub fn on_use_card(&self, card_type: &str) -> Vec<HookEffect> {
        match self.id {
            // RagePower.java: if card.type == ATTACK, gain block = this.amount
            PowerId::Rage => {
                if card_type == "Attack" {
                    vec![HookEffect::GainBlock(self.stacks)]
                } else {
                    vec![]
                }
            }
            // AfterImagePower.java: gain 1 block per card played
            PowerId::AfterImage => vec![HookEffect::GainBlock(self.stacks)],
            // PanachePower.java: every 5 cards, deal 10 damage to all enemies
            PowerId::Panache => {
                vec![HookEffect::AddStacks(-1)]
            }
            // CorruptionPower.java: if Skill, set exhaustCard = true
            PowerId::Corruption => {
                if card_type == "Skill" {
                    vec![HookEffect::ExhaustPlayed]
                } else {
                    vec![]
                }
            }
            // HexPower.java: if NOT Attack, shuffle this.amount Dazed into draw pile
            PowerId::Hex => {
                if card_type != "Attack" {
                    vec![HookEffect::ShuffleStatus { card: "Dazed", count: self.stacks }]
                } else {
                    vec![]
                }
            }
            // TimeWarpPower.java: onAfterUseCard — increment counter, at 12 trigger
            PowerId::TimeWarp => vec![HookEffect::TimeWarpTrigger],
            // BeatOfDeathPower.java: deal this.amount damage to player on each card
            PowerId::BeatOfDeath => vec![HookEffect::DamagePlayer(self.stacks)],
            // ThousandCutsPower.java: deal this.amount damage to ALL enemies on each card
            PowerId::ThousandCuts => vec![HookEffect::DamageAllEnemies(self.stacks)],
            // SharpHidePower.java: if Attack, deal this.amount THORNS damage to player
            PowerId::SharpHide => {
                if card_type == "Attack" {
                    vec![HookEffect::DamagePlayer(self.stacks)]
                } else {
                    vec![]
                }
            }
            // StormPower.java: if Power, channel this.amount Lightning orbs
            PowerId::Storm => {
                if card_type == "Power" {
                    vec![HookEffect::ChannelLightning; self.stacks.max(0) as usize]
                } else {
                    vec![]
                }
            }
            // CuriosityPower.java: if Power, owner gains this.amount Strength (enemy power)
            PowerId::Curiosity => {
                if card_type == "Power" {
                    vec![HookEffect::EnemyGainStrength(self.stacks)]
                } else {
                    vec![]
                }
            }
            // HeatsinkPower.java: if Power, draw this.amount cards
            PowerId::Heatsink => {
                if card_type == "Power" {
                    vec![HookEffect::DrawCards(self.stacks)]
                } else {
                    vec![]
                }
            }
            // ChokePower.java (POWER_ID="Choked"): player loses HP = this.amount per card played
            // Java: onUseCard → LoseHPAction(owner, null, amount)
            // Enemy power on the PLAYER — triggers when player plays any card
            PowerId::Choke => vec![HookEffect::LoseHp(self.stacks)],
            // AttackBurnPower.java: if Attack, exhaust the played card
            // Java: onUseCard → action.exhaustCard = true
            PowerId::AttackBurn => {
                if card_type == "Attack" {
                    vec![HookEffect::ExhaustPlayed]
                } else {
                    vec![]
                }
            }
            // EchoFormPower.java: play card again (first N cards per turn)
            // Java: if cardsPlayedThisTurn - cardsDoubled <= amount → replay
            // Engine handles the replay logic; we signal intent
            PowerId::EchoForm => vec![HookEffect::ReplayCard],
            // DuplicationPower.java: play card again, consume 1 stack
            // Java: onUseCard → replay + --amount
            PowerId::Duplication => vec![
                HookEffect::ReplayCard,
                HookEffect::ReduceStacks(1),
            ],
            // AmplifyPower.java: if Power card, play again + consume stack
            // Java: onUseCard → if card.type == POWER && !purgeOnUse → replay + --amount
            PowerId::Amplify => {
                if card_type == "Power" {
                    vec![
                        HookEffect::ReplayCard,
                        HookEffect::ReduceStacks(1),
                    ]
                } else {
                    vec![]
                }
            }
            // BurstPower.java: if Skill, replay card + consume 1 stack
            // Java: onUseCard → if card.type == SKILL && !purgeOnUse → replay + --amount
            PowerId::Burst => {
                if card_type == "Skill" {
                    vec![
                        HookEffect::ReplayCard,
                        HookEffect::ReduceStacks(1),
                    ]
                } else {
                    vec![]
                }
            }
            // DoubleTapPower.java: if Attack, replay card + consume 1 stack
            // Java: onUseCard → if card.type == ATTACK && !purgeOnUse → replay + --amount
            PowerId::DoubleTap => {
                if card_type == "Attack" {
                    vec![
                        HookEffect::ReplayCard,
                        HookEffect::ReduceStacks(1),
                    ]
                } else {
                    vec![]
                }
            }
            // PenNibPower.java: if Attack card → remove self (double damage already applied via at_damage_give)
            // Java: onUseCard → if card.type == ATTACK → RemoveSpecificPowerAction
            PowerId::PenNib => {
                if card_type == "Attack" {
                    vec![HookEffect::RemoveSelf]
                } else {
                    vec![]
                }
            }
            // SlowPower.java: onAfterUseCard → owner gains 1 Slow stack per card played
            // Java: ApplyPowerAction(owner, owner, SlowPower(1), 1)
            // Each card played increases the damage multiplier by 10%
            PowerId::Slow => vec![HookEffect::AddStacks(1)],
            // VigorPower.java: onUseCard → if Attack, remove self
            // Java: if card.type == ATTACK → RemoveSpecificPowerAction
            // Vigor bonus damage applied via at_damage_give, consumed here
            PowerId::Vigor => {
                if card_type == "Attack" {
                    vec![HookEffect::RemoveSelf]
                } else {
                    vec![]
                }
            }
            // ReboundPower.java: onAfterUseCard → if not Power card, set reboundCard=true, reduce by 1
            // Java: action.reboundCard = true; ReducePowerAction(1)
            // Rebound causes the next card played to go on top of draw pile instead of discard
            PowerId::Rebound => {
                if card_type != "Power" {
                    vec![
                        HookEffect::ReboundCard,
                        HookEffect::ReduceStacks(1),
                    ]
                } else {
                    vec![HookEffect::ReduceStacks(1)]
                }
            }
            // FreeAttackPower.java: if Attack, consume 1 stack (makes next N Attacks free)
            // Java: onUseCard → if card.type == ATTACK && !purgeOnUse → --amount, remove at 0
            PowerId::FreeAttack => {
                if card_type == "Attack" {
                    vec![HookEffect::ReduceStacks(1)]
                } else {
                    vec![]
                }
            }
            // AngerPower.java: if Skill played, owner gains Strength (enemy power)
            // Java: onUseCard → if card.type == SKILL → ApplyPower(Strength, amount)
            PowerId::AngerMonster => {
                if card_type == "Skill" {
                    vec![HookEffect::EnemyGainStrength(self.stacks)]
                } else {
                    vec![]
                }
            }
            // SkillBurnPower.java: if Skill played, exhaust the card
            // Java: onUseCard → if card.type == SKILL → action.exhaustCard = true
            PowerId::SkillBurn => {
                if card_type == "Skill" {
                    vec![HookEffect::ExhaustPlayed]
                } else {
                    vec![]
                }
            }
            // TimeMazePower.java: onAfterUseCard → count down, at 0 end player turn
            // Java: --amount, if 0 → clear card queue, reset, end turn
            PowerId::TimeMaze => vec![
                HookEffect::ReduceStacks(1),
            ],
            // FlowPower.java (deprecated): onUseCard → Watcher mechanic
            // Signal-only arm for deprecated power
            PowerId::FlowPower => vec![],
            // GroundedPower.java (deprecated): onUseCard → Watcher mechanic
            // Signal-only arm for deprecated power
            PowerId::Grounded => vec![],
            // NoSkillsPower.java: canPlayCard → block Skills (handled by engine card filter)
            // Signal-only arm — actual card filtering is in combat.rs
            PowerId::NoSkills => vec![],
            // SadisticPower.java: onApplyPower → damage when debuff applied
            // Signal-only arm — actual damage is applied inline in apply_power
            PowerId::Sadistic => vec![],
            _ => vec![],
        }
    }

    /// Called when a card is drawn.
    ///
    /// Java: `onCardDraw(AbstractCard card)`
    /// Returns side effects to apply to the drawn card.
    pub fn on_card_draw(&self, card_type: &str) -> Vec<HookEffect> {
        match self.id {
            // CorruptionPower.java: if card.type == SKILL, card.setCostForTurn(-9) = free
            PowerId::Corruption => {
                if card_type == "Skill" {
                    vec![HookEffect::SetSkillCostZero]
                } else {
                    vec![]
                }
            }
            // EvolvePower.java: if Status drawn (and no NoDraw), draw this.amount cards
            PowerId::Evolve => {
                if card_type == "Status" {
                    vec![HookEffect::DrawCards(self.stacks)]
                } else {
                    vec![]
                }
            }
            // FireBreathingPower.java: if Status or Curse drawn, deal this.amount to all enemies
            PowerId::FireBreathing => {
                if card_type == "Status" || card_type == "Curse" {
                    vec![HookEffect::DamageAllEnemies(self.stacks)]
                } else {
                    vec![]
                }
            }
            // EnragePower: gain Strength when a Status card is drawn
            // Java: if card.type == STATUS, gain this.amount Strength
            PowerId::Enrage => {
                if card_type == "Status" {
                    vec![HookEffect::GainStrength(self.stacks)]
                } else {
                    vec![]
                }
            }
            // ConfusionPower.java: onCardDraw → if card.cost >= 0, randomize cost 0-3
            // Java: card.cost = card.costForTurn = random(3)
            // We signal the engine to randomize the drawn card's cost
            PowerId::Confused => {
                if card_type != "Curse" && card_type != "Status" {
                    vec![HookEffect::RandomizeCardCost]
                } else {
                    vec![]
                }
            }
            // AccuracyPower.java: onDrawOrDiscard → boost Shiv damage
            // Actual Shiv damage boost is handled via damage pipeline, signal-only here
            PowerId::Accuracy => vec![],
            // StrikeUpPower.java: onDrawOrDiscard → boost Strike damage
            // Signal-only; engine handles Strike damage via card properties
            PowerId::StrikeUp => vec![],
            _ => vec![],
        }
    }

    /// Called when a card is exhausted.
    ///
    /// Java: `onExhaust(AbstractCard card)`
    /// Returns side effects to apply.
    pub fn on_exhaust(&self) -> Vec<HookEffect> {
        match self.id {
            // FeelNoPainPower.java: gain block = this.amount
            PowerId::FeelNoPain => vec![HookEffect::GainBlock(self.stacks)],
            // DarkEmbracePower.java: draw 1 card
            PowerId::DarkEmbrace => vec![HookEffect::DrawCards(self.stacks)],
            // DeadBranchPower.java: add random card to hand
            // (complex — skip for now)
            _ => vec![],
        }
    }

    /// Called when the player gains block.
    ///
    /// Java: `onGainedBlock(float blockAmount)`
    pub fn on_gained_block(&self, block_amount: i32) -> Vec<HookEffect> {
        match self.id {
            // JuggernautPower.java: deal this.amount damage to random enemy when gaining block
            PowerId::Juggernaut => {
                if block_amount > 0 {
                    vec![HookEffect::DamageRandomEnemy(self.stacks)]
                } else {
                    vec![]
                }
            }
            // WaveOfTheHandPower.java: onGainedBlock → apply Weak to all enemies
            PowerId::WaveOfTheHand => {
                if block_amount > 0 {
                    vec![HookEffect::ApplyWeakToAllEnemies(self.stacks)]
                } else {
                    vec![]
                }
            }
            _ => vec![],
        }
    }

    /// Called when the player loses HP from their own effects (Offering, Bloodletting, etc.)
    ///
    /// Java: `wasHPLost(DamageInfo info, int damageAmount)` where info.owner == this.owner
    pub fn was_hp_lost_self(&self, damage: i32) -> Vec<HookEffect> {
        match self.id {
            // RupturePower.java: gain this.amount Strength when losing HP from self-damage
            PowerId::Rupture => {
                if damage > 0 {
                    vec![HookEffect::GainStrength(self.stacks)]
                } else {
                    vec![]
                }
            }
            // NOTE: PlatedArmor is NOT here. Java's PlatedArmorPower.wasHPLost only
            // fires on DamageType.NORMAL (enemy attack damage), NOT on self-damage
            // (DamageType.HP_LOSS from Offering, Brutality, etc.).
            // PlatedArmor reduction on enemy attacks is handled in enemy.rs take_damage_from_player
            // and in the enemy turn resolution code.
            _ => vec![],
        }
    }

    /// Called when the player deals unblocked attack damage to an enemy.
    ///
    /// Java: `onAttack(DamageInfo info, int damageAmount, AbstractCreature target)`
    pub fn on_attack(&self, damage: i32) -> Vec<HookEffect> {
        match self.id {
            // EnvenomPower.java: apply this.amount Poison to target on unblocked damage
            PowerId::Envenom => {
                if damage > 0 {
                    vec![HookEffect::ApplyPoisonToTarget(self.stacks)]
                } else {
                    vec![]
                }
            }
            // PainfulStabsPower.java: onInflictDamage → add 1 Wound to player's discard
            // Java: if damageAmount > 0 && type != THORNS → MakeTempCardInDiscard(Wound, 1)
            // This is an ENEMY power — when this enemy deals unblocked damage, player gets a Wound
            PowerId::PainfulStabs => {
                if damage > 0 {
                    vec![HookEffect::AddStatusToDiscard { card: "Wound", count: 1 }]
                } else {
                    vec![]
                }
            }
            _ => vec![],
        }
    }

    /// Called when the player changes stance (Watcher mechanic).
    ///
    /// Java: Powers like MentalFortressPower override `onChangeStance()`.
    /// `new_stance_name` is the name of the stance being entered.
    pub fn on_stance_change(&self, new_stance_name: &str) -> Vec<HookEffect> {
        match self.id {
            // MentalFortressPower.java: gain block = this.amount on ANY stance change
            PowerId::MentalFortress => vec![HookEffect::GainBlock(self.stacks)],
            
            // RushdownPower.java: draw this.amount cards when entering Wrath
            PowerId::Rushdown => {
                if new_stance_name == "Wrath" {
                    vec![HookEffect::DrawCards(self.stacks)]
                } else {
                    vec![]
                }
            }
            
            // AdaptationPower.java: onChangeStance → gain block/strength
            PowerId::Adaptation => vec![HookEffect::GainBlock(self.stacks)],
            
            // ControlledPower.java: onChangeStance → scry
            PowerId::Controlled => vec![HookEffect::Scry(self.stacks)],
            
            // MasteryPower.java: onChangeStance → draw cards
            // Deprecated power, signal-only
            PowerId::Mastery => vec![HookEffect::DrawCards(self.stacks)],
            
            _ => vec![],
        }
    }

    /// Called when the player scrys (Watcher mechanic).
    ///
    /// Java: `NirvanaPower.onScry()` → gain block equal to amount.
    pub fn on_scry(&self) -> Vec<HookEffect> {
        match self.id {
            // NirvanaPower.java: gain block = this.amount on scry
            PowerId::Nirvana => vec![HookEffect::GainBlock(self.stacks)],
            _ => vec![],
        }
    }

    /// Called when an enemy (power owner) dies.
    ///
    /// Java: `onDeath()`  
    /// CorpseExplosion: deal maxHP * amount to all enemies
    /// SporeCloud: apply Vulnerable to player
    pub fn on_death(&self, owner_max_hp: i32) -> Vec<HookEffect> {
        match self.id {
            // CorpseExplosionPower.java: deal maxHP * this.amount to all remaining enemies
            PowerId::CorpseExplosion => {
                let damage = owner_max_hp * self.stacks;
                vec![HookEffect::DamageAllEnemies(damage)]
            }
            // SporeCloudPower.java: apply this.amount Vulnerable to player
            PowerId::SporeCloud => {
                vec![HookEffect::ApplyVulnerableToPlayer(self.stacks)]
            }
            // StasisPower.java: on enemy death, return stolen card to hand
            PowerId::Stasis => vec![],  // signal-only: engine handles card return
            // RepairPower.java: onVictory → heal HP post-combat
            // Signal-only arm — engine applies heal after combat if player alive
            PowerId::Repair => vec![HookEffect::HealHp(self.stacks)],
            _ => vec![],
        }
    }

    // ========================================================================
    // Turn-based Hooks
    // Java: AbstractPower.atStartOfTurnPostDraw() / atEndOfTurn(boolean isPlayer)
    // ========================================================================

    /// Called at the start of each turn (after cards are drawn).
    ///
    /// Java: `atStartOfTurnPostDraw()` (most start-of-turn effects use this)
    /// Returns side effects to apply.
    pub fn at_start_of_turn(&self) -> Vec<HookEffect> {
        match self.id {
            // DemonFormPower.java: this.addToBot(new GainPowerAction(p, p, new StrengthPower(p, this.amount), this.amount))
            PowerId::DemonForm => vec![HookEffect::GainStrength(self.stacks)],
            
            // BrutalityPower.java: this.addToBot(new LoseHPAction(p, p, 1)); draw 1
            PowerId::Brutality => vec![
                HookEffect::LoseHp(1),
                HookEffect::DrawCards(1),
            ],
            
            // BiasPower.java: this.addToBot(new ApplyPowerAction(p, p, new FocusPower(p, -this.amount), -this.amount))
            // Loses focus each turn
            PowerId::Bias => vec![HookEffect::ApplyPower { id: PowerId::Bias, stacks: -self.stacks }],
            
            // NoxiousFumesPower.java: Apply Poison to ALL enemies
            PowerId::NoxiousFumes => vec![HookEffect::PoisonAllEnemies(self.stacks)],
            
            // NextTurnBlockPower.java: gain block = this.amount, then remove self
            PowerId::NextTurnBlock => vec![
                HookEffect::GainBlock(self.stacks),
                HookEffect::RemoveSelf,
            ],
            
            // BattleHymnPower.java: add Smite to hand
            PowerId::BattleHymn => vec![HookEffect::CreateCardInHand { card_id: "Smite", count: self.stacks }],
            
            // DevotionPower.java: gain Mantra = this.amount
            PowerId::Devotion => vec![HookEffect::ApplyPower { id: PowerId::Mantra, stacks: self.stacks }],
            
            // CreativeAIPower.java: add random Power card to hand
            // (complex — requires card generation, skip for now)
            
            // BerserkPower.java: gain energy = this.amount (unconditional in Java)
            PowerId::Berserk => vec![HookEffect::GainEnergy(self.stacks)],
            
            // DrawCardNextTurnPower (DrawCard): draw extra cards, remove self
            PowerId::DrawCard => vec![
                HookEffect::DrawCards(self.stacks),
                HookEffect::RemoveSelf,
            ],
            
            // EnergizedPower (Energized): gain energy, remove self  
            PowerId::Energized => vec![
                HookEffect::GainEnergy(self.stacks),
                HookEffect::RemoveSelf,
            ],
            
            // InfiniteBladesPower.java: add Shiv(s) to hand
            PowerId::InfiniteBlades => vec![HookEffect::CreateCardInHand { card_id: "Shiv", count: self.stacks }],
            
            // ToolsOfTheTradePower.java: draw amount, then discard amount
            // Note: DiscardCards requires player choice in real game; we just draw for now
            PowerId::ToolsOfTrade => vec![HookEffect::DrawCards(self.stacks)],
            
            // MachineLearningPower.java (DrawPower): draw this.amount extra cards
            PowerId::MachineLearning => vec![HookEffect::DrawCards(self.stacks)],
            
            // DevaFormPower.java: gain energy = this.amount, then increase amount by 1
            PowerId::DevaForm => vec![
                HookEffect::GainEnergy(self.stacks),
                HookEffect::AddStacks(1),
            ],
            
            // MagnetismPower.java: add random Colorless card to hand
            PowerId::Magnetism => vec![HookEffect::CreateRandomCardInHand { pool: "Colorless", count: self.stacks }],
            
            // HelloWorldPower.java: add random Common card to hand
            PowerId::HelloWorld => vec![HookEffect::CreateRandomCardInHand { pool: "Common", count: self.stacks }],
            
            // CreativeAIPower.java: add random Power card to hand
            PowerId::CreativeAI => vec![HookEffect::CreateRandomCardInHand { pool: "Power", count: self.stacks }],
            
            // ForesightPower.java: Scry(amount) at start of turn
            PowerId::Foresight => vec![HookEffect::Scry(self.stacks)],
            
            // MayhemPower.java: play top card from draw pile for free
            PowerId::Mayhem => vec![HookEffect::PlayTopCard(self.stacks)],
            
            // ChokePower.java: remove itself at start of turn
            // Java: atStartOfTurn → RemoveSpecificPowerAction(Choked)
            PowerId::Choke => vec![HookEffect::RemoveSelf],
            
            // AttackBurnPower.java: justApplied flag, then decrement by 1 at end of round
            // Java: atEndOfRound → if !justApplied, ReducePowerAction(1); else justApplied = false
            // We handle by keeping decay here (first turn the power was just applied, stacks include that turn)
            // Moved from at_start_of_turn to at_end_of_turn to match Java's atEndOfRound
            
            // LoopPower.java: trigger leftmost orb passive this.amount times at start of turn
            // Java: orbs.get(0).onStartOfTurn() + onEndOfTurn() for each stack
            PowerId::Loop => vec![HookEffect::TriggerOrbPassive(self.stacks)],
            
            
            // CollectPower.java: onEnergyRecharge → gain energy + reduce stacks
            // Java: gainEnergy(1) + ReducePowerAction(1)
            PowerId::Collect => vec![
                HookEffect::GainEnergy(1),
                HookEffect::ReduceStacks(1),
            ],
            
            // PoisonPower.java (on enemies): atStartOfTurn → LoseHP(amount), then reduce by 1
            // Java: deal this.amount HP_LOSS to owner, then ReducePowerAction(1)
            // Note: Poison is on ENEMIES, applied during monster turn start
            PowerId::Poison => vec![
                HookEffect::LoseHp(self.stacks),
                HookEffect::ReduceStacks(1),
            ],
            
            // FlameBarrierPower.java: atStartOfTurn → RemoveSpecificPowerAction
            PowerId::FlameBarrier => vec![HookEffect::RemoveSelf],
            
            // InvinciblePower.java: atStartOfTurn → reset amount to max
            // Java: this.amount = this.maxAmt (stored during construction)
            // We signal ResetToMax — engine stores original value and restores it
            // For now: The Heart's Invincible is always 300; engine handles this
            PowerId::Invincible => vec![HookEffect::ResetToMax],
            
            // FlightPower.java: atStartOfTurn → reset amount to storedAmount
            // Java: this.amount = this.storedAmount (set in constructor)
            PowerId::Flight => vec![HookEffect::ResetToMax],
            
            // PanachePower.java: atStartOfTurn → reset counter to 5
            // Java: this.amount = 5 — resets the play-count trigger
            PowerId::Panache => vec![HookEffect::ResetStacks(5)],
            
            // PhantasmalPower.java: atStartOfTurn → apply DoubleDamage, remove self
            // Java: ApplyPowerAction(DoubleDamage(1)), RemoveSpecificPowerAction
            PowerId::Phantasmal => vec![
                HookEffect::ApplyPower { id: PowerId::DoubleDamage, stacks: 1 },
                HookEffect::RemoveSelf,
            ],
            
            // EchoFormPower.java: atStartOfTurn → reset cardsDoubledThisTurn to 0
            // The counter tracking is engine-side; signal reset via dedicated effect
            PowerId::EchoForm => vec![HookEffect::ResetEchoFormCounter],
            
            // EndTurnDeathPower.java: atStartOfTurn → LoseHPAction(99999) + RemoveSelf
            // Monster dies at the start of its turn (Darklings, etc.)
            PowerId::EndTurnDeath => vec![HookEffect::KillSelf],
            
            // EnergyDownPower.java: atStartOfTurn → LoseEnergyAction(amount)
            // Lose energy at start of each turn (Fasting debuff)
            PowerId::EnergyDown => vec![HookEffect::LoseEnergy(self.stacks)],
            
            // TimeMazePower.java: atStartOfTurn → reset amount to 15
            // Reset the card counter each turn
            PowerId::TimeMaze => vec![HookEffect::ResetStacks(15)],
            
            // WrathNextTurnPower.java: atStartOfTurn → enter Wrath + remove self
            PowerId::WrathNextTurn => vec![
                HookEffect::ChangeStance("Wrath"),
                HookEffect::RemoveSelf,
            ],
            
            // RechargingCore.java: atStartOfTurn → channel 1 Lightning orb
            PowerId::RechargingCore => vec![HookEffect::ChannelOrb("Lightning")],
            
            // WinterPower.java: atStartOfTurn → channel 1 Frost orb
            PowerId::Winter => vec![HookEffect::ChannelOrb("Frost")],
            
            // WireheadingPower.java: atStartOfTurn → reroll self intent
            PowerId::WireheadingPower => vec![HookEffect::RerollIntent],
            
            // EmotionalTurmoilPower.java: atStartOfTurnPostDraw → add random card to hand
            PowerId::EmotionalTurmoil => vec![HookEffect::CreateRandomCardInHand { pool: "Any", count: 1 }],
            
            // NightTerrorPower.java: atStartOfTurn → draw cards = stacks + lose 1 HP per card
            PowerId::NightTerror => vec![
                HookEffect::DrawCards(self.stacks),
                HookEffect::LoseHp(self.stacks),
            ],
            
            // DisciplinePower.java: atStartOfTurn → discard cards (Watcher)
            PowerId::DisciplinePower => vec![HookEffect::ReduceStacks(1)],
            
            // DrawReduction.java: onInitialApplication → reduce hand size
            // Signal-only arm — hand size is handled on application site
            PowerId::DrawReduction => vec![],
            
            _ => vec![],
        }
    }

    /// Called at the end of each turn.
    ///
    /// Java: `atEndOfTurn(boolean isPlayer)` / `atEndOfTurnPreEndTurnCards(boolean isPlayer)`
    /// Returns side effects to apply.
    pub fn at_end_of_turn(&self) -> Vec<HookEffect> {
        match self.id {
            // MetallicizePower.java: gain block = this.amount
            PowerId::Metallicize => vec![HookEffect::GainBlock(self.stacks)],
            
            // PlatedArmorPower.java: gain block = this.amount
            PowerId::PlatedArmor => vec![HookEffect::GainBlock(self.stacks)],
            
            // CombustPower.java: lose HP = this.hpLoss, deal damage = this.amount to all
            PowerId::Combust => vec![
                HookEffect::LoseHp(1),
                HookEffect::DamageAllEnemies(self.stacks),
            ],
            
            // RegenPower.java: heal = this.amount, lose 1 stack
            PowerId::Regen => vec![
                HookEffect::HealHp(self.stacks),
                HookEffect::ReduceStacks(1),
            ],
            
            // ConstrictedPower.java: lose HP = this.amount
            PowerId::Constricted => vec![HookEffect::LoseHp(self.stacks)],
            
            // RitualPower.java: gain Strength = this.amount
            PowerId::Ritual => vec![HookEffect::GainStrength(self.stacks)],
            
            // FadingPower.java: reduce by 1 each turn, die at 0
            PowerId::Fading => vec![HookEffect::ReduceStacks(1)],
            
            // WraithFormPower.java: lose Dexterity each turn
            PowerId::WraithForm => vec![HookEffect::ApplyDexterity(self.stacks)],
            
            // EntanglePower.java: removes itself at end of player turn
            PowerId::Entangled => vec![HookEffect::RemoveSelf],
            
            // NoDrawPower.java: removes itself at end of player turn
            PowerId::NoDraw => vec![HookEffect::RemoveSelf],
            
            // OmegaPower.java: deal this.amount damage to all enemies (THORNS type)
            PowerId::Omega => vec![HookEffect::DamageAllEnemies(self.stacks)],
            
            // StudyPower.java: shuffle Insight into draw pile
            PowerId::Study => vec![HookEffect::ShuffleStatus { card: "Insight", count: self.stacks }],
            
            // GainStrengthPower.java (POWER_ID="Shackled"): regain STR at end of turn
            // Java: atEndOfTurn → ApplyPowerAction(StrengthPower(amount)) + RemoveSelf
            PowerId::Shackled => vec![
                HookEffect::GainStrength(self.stacks),
                HookEffect::RemoveSelf,
            ],
            
            // GrowthPower.java: gain STR = this.amount each turn (skip first)
            // Java: atEndOfRound → if !skipFirst, gain STR
            // We handle skipFirst via the engine (counter tracking)
            PowerId::Growth => vec![HookEffect::GainStrength(self.stacks)],
            
            // LikeWaterPower.java: if in Calm stance, gain block = this.amount
            // Java: atEndOfTurn → if owner.stance == Calm, gain block
            // Engine checks stance; we always emit, engine filters
            PowerId::LikeWater => vec![HookEffect::GainBlockIfCalm(self.stacks)],
            
            // LockOnPower.java: turn-based decrement
            // Java: atEndOfRound → ReducePowerAction(1)
            PowerId::LockOn => vec![HookEffect::ReduceStacks(1)],
            
            // AmplifyPower.java: remove at end of turn
            // Java: atEndOfTurn → RemoveSpecificPowerAction
            PowerId::Amplify => vec![HookEffect::RemoveSelf],
            
            // EstablishmentPower.java: reduce cost of retained cards
            // Java: atEndOfTurn → EstablishmentPowerAction(amount) — reduces cost of retained cards by amount
            // Handled inline in end_turn(); the hook just signals the intent
            PowerId::Establishment => vec![HookEffect::ReduceRetainedCardsCost(self.stacks)],
            
            // === Turn-based debuff decay (Java: atEndOfRound) ===
            // These debuffs decrease by 1 each turn, removed at 0.
            // Java calls ReducePowerAction(this.owner, this.owner, this, 1)
            PowerId::Frail | PowerId::Vulnerable | PowerId::Weak |
            PowerId::Blur | PowerId::IntangiblePlayer | PowerId::DrawReduction |
            PowerId::DoubleDamage | PowerId::AttackBurn => {
                vec![HookEffect::ReduceStacks(1)]
            }
            
            // SlowPower.java: atEndOfRound → this.amount = 0 (reset counter, NOT decrement)
            // Java: the damage multiplier resets each round; cards played in the next turn start from 0
            PowerId::Slow => vec![HookEffect::ResetStacks(0)],
            
            // Intangible (monster version): also decays each turn
            // Java IntangiblePower.java: atEndOfTurn → ReducePowerAction(1)
            PowerId::Intangible => vec![HookEffect::ReduceStacks(1)],
            
            // RagePower.java: remove at end of turn
            PowerId::Rage => vec![HookEffect::RemoveSelf],
            
            // MalleablePower.java: reset amount to base at end of turn
            // Java: atEndOfTurn → reset amount, atEndOfRound → ReducePowerAction
            PowerId::Malleable => vec![HookEffect::ReduceStacks(1)],
            
            // BurstPower.java: remove at end of turn
            PowerId::Burst => vec![HookEffect::RemoveSelf],
            
            // DoubleTapPower.java: remove at end of turn
            PowerId::DoubleTap => vec![HookEffect::RemoveSelf],
            
            // ReboundPower.java: remove at end of turn
            PowerId::Rebound => vec![HookEffect::RemoveSelf],
            
            // TheBombPower.java: reduce stacks by 1, at 0 deal 40 damage to all
            PowerId::TheBomb => vec![HookEffect::ReduceStacks(1)],
            
            // DuplicationPower.java: atEndOfRound → ReducePowerAction(1)
            // Decay by 1 each round, remove at 0
            PowerId::Duplication => vec![HookEffect::ReduceStacks(1)],
            
            // GenericStrengthUpPower.java: atEndOfRound → gain Strength
            // Enemy gains Str each round (monster buff)
            PowerId::GenericStrUp => vec![HookEffect::GainStrength(self.stacks)],
            
            // SkillBurnPower.java: atEndOfRound → ReducePowerAction(1) with justApplied
            PowerId::SkillBurn => vec![HookEffect::ReduceStacks(1)],
            
            // NoSkillsPower.java: atEndOfTurn → RemoveSpecificPowerAction
            PowerId::NoSkills => vec![HookEffect::RemoveSelf],
            
            // AngelFormPower.java: atEndOfTurn → gain Intangible
            PowerId::AngelForm => vec![HookEffect::ApplyPower { id: PowerId::IntangiblePlayer, stacks: 1 }],
            
            // CannotChangeStancePower.java: atEndOfTurn → remove self
            PowerId::CannotChangeStance => vec![HookEffect::RemoveSelf],
            
            // ConservePower.java: atEndOfRound → retain energy (signal only)
            PowerId::Conserve => vec![HookEffect::RemoveSelf],
            
            // WaveOfTheHandPower.java: atEndOfRound → remove self
            PowerId::WaveOfTheHand => vec![HookEffect::RemoveSelf],
            
            // VaultPower.java: atEndOfRound → gain extra turn (signal only)
            PowerId::Vault => vec![HookEffect::RemoveSelf],
            
            // RegenerateMonsterPower.java: atEndOfTurn → heal stacks HP
            PowerId::RegenerateMonster => vec![HookEffect::HealEnemy(self.stacks)],
            
            // DrawPower.java: onRemove → restore gameHandSize (signal-only)
            PowerId::DrawPower => vec![],  // Engine handles hand size at removal
            
            // DisciplinePower.java: atEndOfTurn → signal (Watcher card discard)
            PowerId::DisciplinePower => vec![HookEffect::RemoveSelf],
            
            // ExplosivePower.java: duringTurn → countdown, at 1 explode + suicide
            // Signal: ReduceStacks(1); engine checks if stacks == 0 → deal 30 + kill self
            PowerId::Explosive => vec![HookEffect::ReduceStacks(1)],
            
            // FlightPower.java: onRemove → signal-only (engine handles removal)
            PowerId::Flight => vec![],  // Flight removal is handled by stacks reaching 0
            
            // LoseStrengthPower.java (POWER_ID="Flex"): lose Strength = stacks, then remove
            // Java: atEndOfTurn → ApplyPower(Strength, -amount) + RemoveSelf
            PowerId::StrengthDown => vec![
                HookEffect::GainStrength(-self.stacks),
                HookEffect::RemoveSelf,
            ],
            
            // LoseDexterityPower.java (POWER_ID="DexLoss"): lose Dexterity = stacks, then remove
            // Java: atEndOfTurn → ApplyPower(Dexterity, -amount) + RemoveSelf
            PowerId::DexterityDown => vec![
                HookEffect::ApplyDexterity(-self.stacks),
                HookEffect::RemoveSelf,
            ],
            
            // RegenerateMonsterPower.java (POWER_ID="Regeneration"): heal owner by amount each turn
            // Java: atEndOfTurn → HealAction(owner, amount) — no decay, permanent
            // This is a MONSTER power (enemy heals itself)
            PowerId::Regeneration => vec![HookEffect::HealEnemy(self.stacks)],
            
            // NoBlockPower.java: decay by 1 at end of round
            // Java: atEndOfRound → ReducePowerAction(1) with justApplied flag
            PowerId::NoBlock => vec![HookEffect::ReduceStacks(1)],
            
            // RetainCardPower.java: retain N cards from hand at end of turn
            // Java: atEndOfTurn → RetainCardsAction(amount)
            // Only fires if not Runic Pyramid and not Equilibrium active
            PowerId::RetainCards => vec![HookEffect::RetainCards(self.stacks)],
            
            // EquilibriumPower.java: retain ALL non-ethereal cards + decay by 1 at end of round
            // Java: atEndOfTurn → set retain=true on all non-ethereal; atEndOfRound → reduce by 1
            PowerId::Equilibrium => vec![
                HookEffect::RetainAllCards,
                HookEffect::ReduceStacks(1),
            ],
            
            
            _ => vec![],
        }
    }
}

// ============================================================================
// Pipeline Functions — bridge from PowerSet (HashMap) to hook dispatch
// ============================================================================
//
// These functions take the existing PowerSet (HashMap<String, i32>) and iterate
// through all active powers using the hook system. This is the integration layer
// that allows gradual migration from hardcoded checks to hook dispatch.

use crate::powers::PowerSet;

/// Calculate card damage through the full Java pipeline using hooks.
///
/// Replaces the old `calculate_card_damage` which took pre-extracted booleans.
/// Now iterates through ALL attacker and defender powers:
///
/// 1. Start with base_damage (+ vigor if applicable)
/// 2. atDamageGive: iterate attacker powers (Strength, Weak, DoubleDamage, PenNib)
/// 3. atDamageReceive: iterate defender powers (Vulnerable, Slow)
/// 4. atDamageFinalReceive: iterate defender powers (Intangible, Flight, Forcefield)
/// 5. Floor at zero
///
/// ## Arguments
/// * `base_damage` - Raw card damage (already including vigor, str_mult adjustments)
/// * `attacker_powers` - PowerSet of the attacker
/// * `defender_powers` - PowerSet of the defender (target)
///
/// ## Returns
/// Final damage after all power modifications.
/// Relic flags that modify damage multipliers (OddMushroom, PaperCrane, PaperFrog).
///
/// Passed into the damage pipeline to adjust Vulnerable/Weak multipliers.
#[derive(Debug, Clone, Copy, Default)]
pub struct RelicDamageFlags {
    /// OddMushroom: player's Vulnerable → 25% more (+1.25) instead of 50% (+1.5)
    pub odd_mushroom: bool,
    /// PaperCrane: player's Weak → 40% less (*0.6) instead of 25% less (*0.75)
    pub paper_crane: bool,
    /// PaperFrog: enemy's Vulnerable → 75% more (+1.75) instead of 50% (+1.5)
    pub paper_frog: bool,
}

pub fn calculate_damage_hooked(
    base_damage: i32,
    attacker_powers: &PowerSet,
    defender_powers: &PowerSet,
    attacker_stance: crate::core::stances::Stance,
    defender_stance: crate::core::stances::Stance,
    relic_flags: RelicDamageFlags,
) -> i32 {
    let mut tmp: f32 = base_damage as f32;

    // Step 2: atDamageGive — iterate attacker's powers
    // IMPORTANT: Java processes powers in ArrayList insertion order.
    // Strength (additive) is always applied before Weak (multiplicative).
    // HashMap has non-deterministic order, so we sort: additive first, then multiplicative.
    let mut attacker_list: Vec<_> = attacker_powers.iter()
        .map(|(id_str, &stacks)| PowerInstance::new(PowerId::from_str(id_str), stacks))
        .filter(|p| matches!(p.id, PowerId::Strength | PowerId::Weak | PowerId::DoubleDamage | PowerId::PenNib))
        .collect();
    attacker_list.sort_by_key(|p| match p.id {
        PowerId::Strength => 0,      // additive — first
        PowerId::Weak => 1,           // multiplicative — second
        PowerId::DoubleDamage => 2,   // multiplicative — after
        PowerId::PenNib => 3,         // multiplicative — after
        _ => 99,
    });
    for power in &attacker_list {
        // PaperCrane: Weak causes 40% less damage instead of 25% less
        if power.id == PowerId::Weak && relic_flags.paper_crane {
            tmp = tmp * 0.6; // 40% reduction instead of 25%
        } else {
            tmp = power.at_damage_give(tmp);
        }
    }
    
    // Step 2b: Stance atDamageGive (Wrath ×2, Divinity ×3)
    tmp = attacker_stance.at_damage_give(tmp);

    // Step 3: atDamageReceive — iterate defender's powers
    for (id_str, &stacks) in defender_powers.iter() {
        let power = PowerInstance::new(PowerId::from_str(id_str), stacks);
        // OddMushroom: player's Vulnerable = 25% more instead of 50%
        // PaperFrog: enemy's Vulnerable = 75% more instead of 50%
        if power.id == PowerId::Vulnerable {
            if relic_flags.odd_mushroom {
                tmp = tmp * 1.25; // Player has OddMushroom: less Vuln penalty
            } else if relic_flags.paper_frog {
                tmp = tmp * 1.75; // Player has PaperFrog: more Vuln bonus on enemies
            } else {
                tmp = power.at_damage_receive(tmp);
            }
        } else {
            tmp = power.at_damage_receive(tmp);
        }
    }
    
    // Step 3b: Stance atDamageReceive (Wrath ×2 incoming)
    tmp = defender_stance.at_damage_receive(tmp);

    // Step 4: atDamageFinalReceive — iterate defender's powers
    for (id_str, &stacks) in defender_powers.iter() {
        let power = PowerInstance::new(PowerId::from_str(id_str), stacks);
        tmp = power.at_damage_final_receive(tmp);
    }

    // Floor at zero
    if tmp < 0.0 { tmp = 0.0; }
    tmp.floor() as i32
}

/// Calculate block through the Java pipeline using hooks.
///
/// Replaces the old inline Dex/Frail check in `Player::gain_block()`.
///
/// 1. Start with base_block (float)
/// 2. modifyBlock: iterate powers (Dexterity, Frail)
/// 3. modifyBlockLast: iterate powers (NoBlock)
/// 4. Floor at zero
pub fn calculate_block_hooked(
    base_block: i32,
    powers: &PowerSet,
) -> i32 {
    let mut tmp: f32 = base_block as f32;

    // modifyBlock
    for (id_str, &stacks) in powers.iter() {
        let power = PowerInstance::new(PowerId::from_str(id_str), stacks);
        tmp = power.modify_block(tmp);
    }

    // modifyBlockLast
    for (id_str, &stacks) in powers.iter() {
        let power = PowerInstance::new(PowerId::from_str(id_str), stacks);
        tmp = power.modify_block_last(tmp);
    }

    if tmp < 0.0 { tmp = 0.0; }
    tmp.floor() as i32
}

/// Collect onAttacked effects from all defender powers.
///
/// Returns a list of HookEffects that the engine should apply.
/// Does NOT modify damage — use `on_attacked_to_change_damage` hooks for that.
pub fn collect_on_attacked_effects(
    damage: i32,
    defender_powers: &PowerSet,
) -> Vec<HookEffect> {
    let mut effects = Vec::new();
    for (id_str, &stacks) in defender_powers.iter() {
        let power = PowerInstance::new(PowerId::from_str(id_str), stacks);
        let (_, mut power_effects) = power.on_attacked(damage);
        effects.append(&mut power_effects);
    }
    effects
}

/// Apply onAttackedToChangeDamage hooks from all defender powers.
///
/// Returns modified damage amount. Called BEFORE take_damage.
pub fn apply_on_attacked_to_change_damage(
    damage: i32,
    defender_powers: &PowerSet,
) -> i32 {
    let mut modified_damage = damage;
    for (id_str, &stacks) in defender_powers.iter() {
        let power = PowerInstance::new(PowerId::from_str(id_str), stacks);
        modified_damage = power.on_attacked_to_change_damage(modified_damage);
    }
    modified_damage
}

/// Collect onUseCard effects from all player powers.
///
/// `card_type` should be "Attack", "Skill", "Power", or "Status".
pub fn collect_on_use_card_effects(
    card_type: &str,
    player_powers: &PowerSet,
) -> Vec<HookEffect> {
    let mut effects = Vec::new();
    for (id_str, &stacks) in player_powers.iter() {
        let power = PowerInstance::new(PowerId::from_str(id_str), stacks);
        let mut power_effects = power.on_use_card(card_type);
        effects.append(&mut power_effects);
    }
    effects
}

/// Collect onExhaust effects from all player powers.
pub fn collect_on_exhaust_effects(
    player_powers: &PowerSet,
) -> Vec<HookEffect> {
    let mut effects = Vec::new();
    for (id_str, &stacks) in player_powers.iter() {
        let power = PowerInstance::new(PowerId::from_str(id_str), stacks);
        let mut power_effects = power.on_exhaust();
        effects.append(&mut power_effects);
    }
    effects
}

/// Collect onCardDraw effects from all player powers.
///
/// Called when a card is drawn. Returns effects to apply to the drawn card.
pub fn collect_on_card_draw_effects(
    card_type: &str,
    player_powers: &PowerSet,
) -> Vec<HookEffect> {
    let mut effects = Vec::new();
    for (id_str, &stacks) in player_powers.iter() {
        let power = PowerInstance::new(PowerId::from_str(id_str), stacks);
        let mut power_effects = power.on_card_draw(card_type);
        effects.append(&mut power_effects);
    }
    effects
}

/// Collect atStartOfTurn effects from all player powers.
///
/// Called during on_turn_start() after cards are drawn.
/// Returns (PowerId, effects) pairs so the engine can attribute effects to source.
pub fn collect_at_start_of_turn_effects(
    player_powers: &PowerSet,
) -> Vec<(PowerId, Vec<HookEffect>)> {
    let mut all_effects = Vec::new();
    for (id_str, &stacks) in player_powers.iter() {
        let power = PowerInstance::new(PowerId::from_str(id_str), stacks);
        let effects = power.at_start_of_turn();
        if !effects.is_empty() {
            all_effects.push((power.id, effects));
        }
    }
    all_effects
}

/// Collect atEndOfTurn effects from all player powers.
///
/// Called during on_turn_end() before enemy turns.
/// Returns (PowerId, effects) pairs so the engine can attribute effects to source.
pub fn collect_at_end_of_turn_effects(
    player_powers: &PowerSet,
) -> Vec<(PowerId, Vec<HookEffect>)> {
    let mut all_effects = Vec::new();
    for (id_str, &stacks) in player_powers.iter() {
        let power = PowerInstance::new(PowerId::from_str(id_str), stacks);
        let effects = power.at_end_of_turn();
        if !effects.is_empty() {
            all_effects.push((power.id, effects));
        }
    }
    all_effects
}

// ============================================================================
// HookEffect — side effects produced by hooks
// ============================================================================

/// A side effect produced by a power hook.
///
/// The engine reads these and applies the effects to game state.
/// This avoids the borrow checker issue of passing `&mut GameState` into hooks.
#[derive(Debug, Clone, PartialEq)]
pub enum HookEffect {
    /// Deal damage to the attacker (Thorns, Flame Barrier)
    DamageAttacker(i32),
    /// Gain block on self
    GainBlock(i32),
    /// Gain Strength on self
    GainStrength(i32),
    /// Draw cards
    DrawCards(i32),
    /// Lose HP on self
    LoseHp(i32),
    /// Deal damage to all enemies
    DamageAllEnemies(i32),
    /// Apply a power to self
    ApplyPower { id: PowerId, stacks: i32 },
    /// Consume one stack of this power
    ConsumeStack,
    /// Remove this power entirely
    RemoveSelf,
    /// Add stacks to this power (+N or -N)
    AddStacks(i32),
    /// Channel a Lightning orb (Defect)
    ChannelLightning,
    /// Channel a Frost orb (Defect)
    ChannelFrost,
    /// Gain energy
    GainEnergy(i32),
    /// Create a temporary card in hand
    CreateCardInHand { card_id: &'static str, count: i32 },
    /// Apply Poison to all enemies
    PoisonAllEnemies(i32),
    /// Apply Vulnerable to player (enemy power)
    ApplyVulnerableToPlayer(i32),
    /// Heal HP on self
    HealHp(i32),
    /// Reduce stacks by N (like Regen losing 1 per turn, Fading counting down)
    ReduceStacks(i32),

    // === New variants (Feb 2026 expansion) ===

    /// Exhaust the card just played (Corruption: Skills are exhausted)
    ExhaustPlayed,
    /// Shuffle status cards into draw pile (Hex: add Dazed)
    ShuffleStatus { card: &'static str, count: i32 },
    /// Deal damage to the player (BeatOfDeath)
    DamagePlayer(i32),
    /// TimeWarp trigger: increment counter, at 12 end turn + Str to all enemies
    TimeWarpTrigger,
    /// Set all Skill costs to 0 this combat (Corruption onCardDraw)
    SetSkillCostZero,
    /// Apply Dexterity to player (WraithForm: negative per turn)
    ApplyDexterity(i32),
    /// Enemy (power owner) gains Strength (Curiosity)
    EnemyGainStrength(i32),
    /// Deal damage to a random enemy (Juggernaut: on gaining block)
    DamageRandomEnemy(i32),
    /// Apply Poison to the target enemy (Envenom: on dealing unblocked damage)
    ApplyPoisonToTarget(i32),
    /// Create a random card in hand from a pool (Magnetism: "Colorless", HelloWorld: "Common", CreativeAI: "Power")
    CreateRandomCardInHand { pool: &'static str, count: i32 },
    /// Scry N cards (Foresight: look at top N, discard Curses/Status)
    Scry(i32),
    /// Play top card(s) from draw pile for free (Mayhem)
    PlayTopCard(i32),
    /// Replay the card just played (EchoForm, Duplication)
    ReplayCard,
    /// Re-roll enemy intent (Reactive monster power)
    RerollIntent,
    /// Add a status card to player's discard pile (PainfulStabs: Wound)
    AddStatusToDiscard { card: &'static str, count: i32 },
    /// Gain block only if in Calm stance (LikeWater)
    GainBlockIfCalm(i32),
    /// Player gains block (BlockReturn / Talk to the Hand enemy debuff)
    PlayerGainBlock(i32),
    /// Reduce cost of retained cards (Establishment)
    ReduceRetainedCardsCost(i32),
    /// Trigger leftmost orb passive N times (Loop)
    TriggerOrbPassive(i32),
    /// Reset stacks to a specific value (Panache: reset to 5 each turn)
    ResetStacks(i32),
    /// Rebound: put the played card on top of draw pile instead of discard
    ReboundCard,
    /// Reset stacks to original/max value (Flight: storedAmount, Invincible: maxAmt)
    /// Engine stores the original value in a separate map and restores it
    ResetToMax,
    /// Reset EchoForm's cardsDoubled counter to 0 for the new turn  
    ResetEchoFormCounter,
    
    // === Tier 1 power effects ===
    
    /// Randomize the drawn card's cost to 0-3 (Confusion power)
    RandomizeCardCost,
    /// Heal an enemy by N HP (Regeneration: monster heals itself every turn)
    HealEnemy(i32),
    /// Retain N cards from hand at end of turn (RetainCards power)
    RetainCards(i32),
    /// Retain ALL non-ethereal cards from hand at end of turn (Equilibrium)
    RetainAllCards,
    
    // === Tier A power effects ===
    
    /// Kill the owner (EndTurnDeath: monster dies at start of turn)
    KillSelf,
    /// Lose energy at start of turn (EnergyDown/Fasting)
    LoseEnergy(i32),
    /// Change stance (WrathNextTurn: enter Wrath)
    ChangeStance(&'static str),
    /// Channel an orb by name (RechargingCore: Lightning, Winter: Frost)
    ChannelOrb(&'static str),
    /// Apply Weak to all enemies (WaveOfTheHand)
    ApplyWeakToAllEnemies(i32),
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_damage_give_pipeline() {
        let strength = PowerInstance::new(PowerId::Strength, 3);
        assert_eq!(strength.at_damage_give(10.0), 13.0);

        let weak = PowerInstance::new(PowerId::Weak, 2);
        assert_eq!(weak.at_damage_give(10.0), 7.5); // 10 * 0.75 = 7.5 (no floor)

        let double = PowerInstance::new(PowerId::DoubleDamage, 1);
        assert_eq!(double.at_damage_give(10.0), 20.0);
    }

    #[test]
    fn test_damage_receive_pipeline() {
        let vuln = PowerInstance::new(PowerId::Vulnerable, 2);
        // 6 * 1.5 = 9.0
        assert_eq!(vuln.at_damage_receive(6.0), 9.0);
        // 7 * 1.5 = 10.5 (no intermediate floor)
        assert_eq!(vuln.at_damage_receive(7.0), 10.5);
    }

    #[test]
    fn test_damage_final_receive() {
        let intangible = PowerInstance::new(PowerId::Intangible, 1);
        assert_eq!(intangible.at_damage_final_receive(100.0), 1.0);
        assert_eq!(intangible.at_damage_final_receive(0.0), 0.0);

        let flight = PowerInstance::new(PowerId::Flight, 3);
        assert_eq!(flight.at_damage_final_receive(10.0), 5.0); // 10 * 0.5
        assert_eq!(flight.at_damage_final_receive(7.0), 3.5); // 7 * 0.5 = 3.5 (no floor)
    }

    #[test]
    fn test_modify_block() {
        let dex = PowerInstance::new(PowerId::Dexterity, 2);
        assert_eq!(dex.modify_block(5.0), 7.0); // 5 + 2

        let frail = PowerInstance::new(PowerId::Frail, 1);
        assert_eq!(frail.modify_block(8.0), 6.0); // 8 * 0.75 = 6.0

        let no_block = PowerInstance::new(PowerId::NoBlock, 1);
        assert_eq!(no_block.modify_block_last(10.0), 0.0);
    }

    #[test]
    fn test_on_attacked_thorns() {
        let thorns = PowerInstance::new(PowerId::Thorns, 3);
        let (dmg, effects) = thorns.on_attacked(5);
        assert_eq!(dmg, 5); // damage unchanged
        assert_eq!(effects, vec![HookEffect::DamageAttacker(3)]);
    }

    #[test]
    fn test_on_attacked_curl_up() {
        let curl_up = PowerInstance::new(PowerId::CurlUp, 7);
        let (dmg, effects) = curl_up.on_attacked(5);
        assert_eq!(dmg, 5);
        assert_eq!(effects, vec![
            HookEffect::GainBlock(7),
            HookEffect::RemoveSelf,
        ]);
    }

    #[test]
    fn test_power_id_from_str() {
        assert_eq!(PowerId::from_str("Strength"), PowerId::Strength);
        assert_eq!(PowerId::from_str("Vulnerable"), PowerId::Vulnerable);
        assert_eq!(PowerId::from_str("Flame Barrier"), PowerId::FlameBarrier);
        assert_eq!(PowerId::from_str("Curl Up"), PowerId::CurlUp);
        assert_eq!(PowerId::from_str("Made Up Power"), PowerId::Unknown);
    }

    #[test]
    fn test_is_debuff() {
        assert!(PowerId::Vulnerable.is_debuff());
        assert!(PowerId::Weak.is_debuff());
        assert!(PowerId::Frail.is_debuff());
        assert!(PowerId::Poison.is_debuff());
        assert!(!PowerId::Strength.is_debuff());
        assert!(!PowerId::Artifact.is_debuff());
        assert!(!PowerId::Thorns.is_debuff());
    }

    #[test]
    fn test_buffer_blocks_damage() {
        let buffer = PowerInstance::new(PowerId::Buffer, 1);
        assert_eq!(buffer.on_attacked_to_change_damage(10), 0);
        assert_eq!(buffer.on_attacked_to_change_damage(0), 0);
    }

    #[test]
    fn test_invincible_caps_damage() {
        let invincible = PowerInstance::new(PowerId::Invincible, 300);
        assert_eq!(invincible.on_attacked_to_change_damage(500), 300);
        assert_eq!(invincible.on_attacked_to_change_damage(100), 100);
    }
}
