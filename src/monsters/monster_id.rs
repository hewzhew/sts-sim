//! Monster ID registry — canonical IDs for all monsters in Slay the Spire.
//!
//! Naming convention: game display name → PascalCase, strip "The"
//! Example: "Jaw Worm" → JawWorm, "The Champ" → Champ
//!
//! See `tests/id_comparison.txt` for full Java ID ↔ Rust ID mapping.

use std::fmt;

/// Canonical monster identifier.
/// Named after the in-game display name (PascalCase), NOT the Java internal ID.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MonsterId {
    // ========== Act 1 — Normal ==========
    AcidSlime_L,
    AcidSlime_M,
    AcidSlime_S,
    Cultist,
    FatGremlin,
    FungiBeast,
    GreenLouse,
    JawWorm,
    JawWorm_Hard,  // Act 3 "Jaw Worm Horde" variant with boosted stats
    Looter,
    MadGremlin,
    RedLouse,
    SneakyGremlin,
    ShieldGremlin,
    GremlinWizard,
    SpikeSlime_L,
    SpikeSlime_M,
    SpikeSlime_S,
    BlueSlaver,
    RedSlaver,

    // ========== Act 1 — Elite ==========
    GremlinNob,
    Lagavulin,
    Sentry,

    // ========== Act 1 — Boss ==========
    Guardian,
    Hexaghost,
    SlimeBoss,

    // ========== Act 2 — Normal ==========
    Bear,           // BanditBear
    BookOfStabbing,
    BronzeAutomaton,
    BronzeOrb,
    Byrd,
    Centurion,
    Chosen,
    Mugger,
    Mystic,        // Java: Healer
    Pointy,        // BanditChild
    Romeo,         // BanditLeader
    ShelledParasite,
    SnakePlant,
    Snecko,
    SphericGuardian,
    Taskmaster,    // SlaverBoss
    TorchHead,

    // ========== Act 2 — Elite ==========
    GremlinLeader,

    // ========== Act 2 — Boss ==========
    Champ,
    Collector,     // TheCollector

    // ========== Act 3 — Normal ==========
    Darkling,
    Exploder,
    Maw,           // The Maw
    OrbWalker,
    Repulsor,
    Spiker,
    SpireGrowth,   // Java: Serpent
    Transient,
    WrithingMass,

    // ========== Act 3 — Elite ==========
    GiantHead,
    Nemesis,
    Reptomancer,

    // ========== Act 3 — Boss ==========
    AwakenedOne,
    Deca,
    Donu,
    TimeEater,

    // ========== Act 4 ==========
    CorruptHeart,
    SpireShield,
    SpireSpear,

    // ========== Special / Minion ==========
    Dagger,        // Reptomancer's dagger
}

impl MonsterId {
    /// Get the in-game display name.
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::AcidSlime_L => "Acid Slime (L)",
            Self::AcidSlime_M => "Acid Slime (M)",
            Self::AcidSlime_S => "Acid Slime (S)",
            Self::Cultist => "Cultist",
            Self::FatGremlin => "Fat Gremlin",
            Self::FungiBeast => "Fungi Beast",
            Self::GreenLouse => "Green Louse",
            Self::JawWorm => "Jaw Worm",
            Self::JawWorm_Hard => "Jaw Worm",  // Same display name, different stats
            Self::Looter => "Looter",
            Self::MadGremlin => "Mad Gremlin",
            Self::RedLouse => "Red Louse",
            Self::SneakyGremlin => "Sneaky Gremlin",
            Self::ShieldGremlin => "Shield Gremlin",
            Self::GremlinWizard => "Gremlin Wizard",
            Self::SpikeSlime_L => "Spike Slime (L)",
            Self::SpikeSlime_M => "Spike Slime (M)",
            Self::SpikeSlime_S => "Spike Slime (S)",
            Self::BlueSlaver => "Blue Slaver",
            Self::RedSlaver => "Red Slaver",
            Self::GremlinNob => "Gremlin Nob",
            Self::Lagavulin => "Lagavulin",
            Self::Sentry => "Sentry",
            Self::Guardian => "The Guardian",
            Self::Hexaghost => "Hexaghost",
            Self::SlimeBoss => "Slime Boss",
            Self::Bear => "Bear",
            Self::BookOfStabbing => "Book of Stabbing",
            Self::BronzeAutomaton => "Bronze Automaton",
            Self::BronzeOrb => "Bronze Orb",
            Self::Byrd => "Byrd",
            Self::Centurion => "Centurion",
            Self::Chosen => "Chosen",
            Self::Mugger => "Mugger",
            Self::Mystic => "Mystic",
            Self::Pointy => "Pointy",
            Self::Romeo => "Romeo",
            Self::ShelledParasite => "Shelled Parasite",
            Self::SnakePlant => "Snake Plant",
            Self::Snecko => "Snecko",
            Self::SphericGuardian => "Spheric Guardian",
            Self::Taskmaster => "Taskmaster",
            Self::TorchHead => "Torch Head",
            Self::GremlinLeader => "Gremlin Leader",
            Self::Champ => "The Champ",
            Self::Collector => "The Collector",
            Self::Darkling => "Darkling",
            Self::Exploder => "Exploder",
            Self::Maw => "The Maw",
            Self::OrbWalker => "Orb Walker",
            Self::Repulsor => "Repulsor",
            Self::Spiker => "Spiker",
            Self::SpireGrowth => "Spire Growth",
            Self::Transient => "Transient",
            Self::WrithingMass => "Writhing Mass",
            Self::GiantHead => "Giant Head",
            Self::Nemesis => "Nemesis",
            Self::Reptomancer => "Reptomancer",
            Self::AwakenedOne => "Awakened One",
            Self::Deca => "Deca",
            Self::Donu => "Donu",
            Self::TimeEater => "Time Eater",
            Self::CorruptHeart => "Corrupt Heart",
            Self::SpireShield => "Spire Shield",
            Self::SpireSpear => "Spire Spear",
            Self::Dagger => "Dagger",
        }
    }

    /// Get the Java internal ID (for cross-referencing Java source code).
    /// Use this when searching `C:\Dev\rust\cardcrawl\monsters\`.
    pub fn java_id(&self) -> &'static str {
        match self {
            Self::AcidSlime_L => "AcidSlime_L",
            Self::AcidSlime_M => "AcidSlime_M",
            Self::AcidSlime_S => "AcidSlime_S",
            Self::Cultist => "Cultist",
            Self::FatGremlin => "GremlinFat",
            Self::FungiBeast => "FungiBeast",
            Self::GreenLouse => "FuzzyLouseDefensive",
            Self::JawWorm | Self::JawWorm_Hard => "JawWorm",
            Self::Looter => "Looter",
            Self::MadGremlin => "GremlinWarrior",
            Self::RedLouse => "FuzzyLouseNormal",
            Self::SneakyGremlin => "GremlinThief",
            Self::ShieldGremlin => "GremlinTsundere",
            Self::GremlinWizard => "GremlinWizard",
            Self::SpikeSlime_L => "SpikeSlime_L",
            Self::SpikeSlime_M => "SpikeSlime_M",
            Self::SpikeSlime_S => "SpikeSlime_S",
            Self::BlueSlaver => "SlaverBlue",
            Self::RedSlaver => "SlaverRed",
            Self::GremlinNob => "GremlinNob",
            Self::Lagavulin => "Lagavulin",
            Self::Sentry => "Sentry",
            Self::Guardian => "TheGuardian",
            Self::Hexaghost => "Hexaghost",
            Self::SlimeBoss => "SlimeBoss",
            Self::Bear => "BanditBear",
            Self::BookOfStabbing => "BookOfStabbing",
            Self::BronzeAutomaton => "BronzeAutomaton",
            Self::BronzeOrb => "BronzeOrb",
            Self::Byrd => "Byrd",
            Self::Centurion => "Centurion",
            Self::Chosen => "Chosen",
            Self::Mugger => "Mugger",
            Self::Mystic => "Healer",
            Self::Pointy => "BanditChild",
            Self::Romeo => "BanditLeader",
            Self::ShelledParasite => "Shelled Parasite",
            Self::SnakePlant => "SnakePlant",
            Self::Snecko => "Snecko",
            Self::SphericGuardian => "SphericGuardian",
            Self::Taskmaster => "SlaverBoss",
            Self::TorchHead => "TorchHead",
            Self::GremlinLeader => "GremlinLeader",
            Self::Champ => "Champ",
            Self::Collector => "TheCollector",
            Self::Darkling => "Darkling",
            Self::Exploder => "Exploder",
            Self::Maw => "Maw",
            Self::OrbWalker => "Orb Walker",
            Self::Repulsor => "Repulsor",
            Self::Spiker => "Spiker",
            Self::SpireGrowth => "Serpent",
            Self::Transient => "Transient",
            Self::WrithingMass => "WrithingMass",
            Self::GiantHead => "GiantHead",
            Self::Nemesis => "Nemesis",
            Self::Reptomancer => "Reptomancer",
            Self::AwakenedOne => "AwakenedOne",
            Self::Deca => "Deca",
            Self::Donu => "Donu",
            Self::TimeEater => "TimeEater",
            Self::CorruptHeart => "CorruptHeart",
            Self::SpireShield => "SpireShield",
            Self::SpireSpear => "SpireSpear",
            Self::Dagger => "Dagger",
        }
    }

    /// Resolve a MonsterId from a string that could be any of:
    /// - Rust canonical name ("JawWorm")
    /// - Display name ("Jaw Worm")
    /// - Java ID ("JawWorm")
    /// - Legacy name with spaces from JSON or CommunicationMod logs
    pub fn from_str_fuzzy(s: &str) -> Option<MonsterId> {
        // Exact match on display name first (most common from game logs)
        match s {
            "Acid Slime (L)" => return Some(Self::AcidSlime_L),
            "Acid Slime (M)" => return Some(Self::AcidSlime_M),
            "Acid Slime (S)" => return Some(Self::AcidSlime_S),
            "Cultist" => return Some(Self::Cultist),
            "Fat Gremlin" => return Some(Self::FatGremlin),
            "Fungi Beast" => return Some(Self::FungiBeast),
            "Green Louse" => return Some(Self::GreenLouse),
            "Jaw Worm" => return Some(Self::JawWorm),
            "Jaw Worm (Hard)" => return Some(Self::JawWorm_Hard),
            "Looter" => return Some(Self::Looter),
            "Mad Gremlin" => return Some(Self::MadGremlin),
            "Red Louse" => return Some(Self::RedLouse),
            "Sneaky Gremlin" => return Some(Self::SneakyGremlin),
            "Shield Gremlin" => return Some(Self::ShieldGremlin),
            "Gremlin Wizard" => return Some(Self::GremlinWizard),
            "Spike Slime (L)" => return Some(Self::SpikeSlime_L),
            "Spike Slime (M)" => return Some(Self::SpikeSlime_M),
            "Spike Slime (S)" => return Some(Self::SpikeSlime_S),
            "Blue Slaver" => return Some(Self::BlueSlaver),
            "Red Slaver" => return Some(Self::RedSlaver),
            "Gremlin Nob" => return Some(Self::GremlinNob),
            "Lagavulin" => return Some(Self::Lagavulin),
            "Sentry" => return Some(Self::Sentry),
            "The Guardian" => return Some(Self::Guardian),
            "Hexaghost" => return Some(Self::Hexaghost),
            "Slime Boss" => return Some(Self::SlimeBoss),
            "Bear" => return Some(Self::Bear),
            "Book of Stabbing" => return Some(Self::BookOfStabbing),
            "Bronze Automaton" => return Some(Self::BronzeAutomaton),
            "Bronze Orb" => return Some(Self::BronzeOrb),
            "Byrd" => return Some(Self::Byrd),
            "Centurion" => return Some(Self::Centurion),
            "Chosen" => return Some(Self::Chosen),
            "Mugger" => return Some(Self::Mugger),
            "Mystic" => return Some(Self::Mystic),
            "Pointy" => return Some(Self::Pointy),
            "Romeo" => return Some(Self::Romeo),
            "Shelled Parasite" => return Some(Self::ShelledParasite),
            "Snake Plant" => return Some(Self::SnakePlant),
            "Snecko" => return Some(Self::Snecko),
            "Spheric Guardian" => return Some(Self::SphericGuardian),
            "Taskmaster" => return Some(Self::Taskmaster),
            "Torch Head" => return Some(Self::TorchHead),
            "Gremlin Leader" => return Some(Self::GremlinLeader),
            "The Champ" => return Some(Self::Champ),
            "The Collector" => return Some(Self::Collector),
            "Darkling" => return Some(Self::Darkling),
            "Exploder" => return Some(Self::Exploder),
            "The Maw" => return Some(Self::Maw),
            "Orb Walker" => return Some(Self::OrbWalker),
            "Repulsor" => return Some(Self::Repulsor),
            "Spiker" => return Some(Self::Spiker),
            "Spire Growth" => return Some(Self::SpireGrowth),
            "Transient" => return Some(Self::Transient),
            "Writhing Mass" => return Some(Self::WrithingMass),
            "Giant Head" => return Some(Self::GiantHead),
            "Nemesis" => return Some(Self::Nemesis),
            "Reptomancer" => return Some(Self::Reptomancer),
            "Awakened One" => return Some(Self::AwakenedOne),
            "Deca" => return Some(Self::Deca),
            "Donu" => return Some(Self::Donu),
            "Time Eater" => return Some(Self::TimeEater),
            "Corrupt Heart" => return Some(Self::CorruptHeart),
            "Spire Shield" => return Some(Self::SpireShield),
            "Spire Spear" => return Some(Self::SpireSpear),
            "Dagger" => return Some(Self::Dagger),
            _ => {}
        }

        // Java ID fallback (for searching Java source / CommunicationMod raw data)
        match s {
            "AcidSlime_L" => Some(Self::AcidSlime_L),
            "AcidSlime_M" => Some(Self::AcidSlime_M),
            "AcidSlime_S" => Some(Self::AcidSlime_S),
            "GremlinFat" => Some(Self::FatGremlin),
            "FungiBeast" => Some(Self::FungiBeast),
            "FuzzyLouseDefensive" => Some(Self::GreenLouse),
            "FuzzyLouseNormal" => Some(Self::RedLouse),
            "JawWorm" => Some(Self::JawWorm),
            "GremlinWarrior" => Some(Self::MadGremlin),
            "GremlinThief" => Some(Self::SneakyGremlin),
            "GremlinTsundere" => Some(Self::ShieldGremlin),
            "SpikeSlime_L" => Some(Self::SpikeSlime_L),
            "SpikeSlime_M" => Some(Self::SpikeSlime_M),
            "SpikeSlime_S" => Some(Self::SpikeSlime_S),
            "SlaverBlue" => Some(Self::BlueSlaver),
            "SlaverRed" => Some(Self::RedSlaver),
            "GremlinNob" => Some(Self::GremlinNob),
            "TheGuardian" => Some(Self::Guardian),
            "SlimeBoss" => Some(Self::SlimeBoss),
            "BanditBear" => Some(Self::Bear),
            "BookOfStabbing" => Some(Self::BookOfStabbing),
            "BronzeAutomaton" => Some(Self::BronzeAutomaton),
            "BronzeOrb" => Some(Self::BronzeOrb),
            "Healer" => Some(Self::Mystic),
            "BanditChild" => Some(Self::Pointy),
            "BanditLeader" => Some(Self::Romeo),
            "SphericGuardian" => Some(Self::SphericGuardian),
            "SlaverBoss" => Some(Self::Taskmaster),
            "TorchHead" => Some(Self::TorchHead),
            "GremlinLeader" => Some(Self::GremlinLeader),
            "Champ" => Some(Self::Champ),
            "TheCollector" => Some(Self::Collector),
            "Maw" => Some(Self::Maw),
            "Serpent" => Some(Self::SpireGrowth),
            "WrithingMass" => Some(Self::WrithingMass),
            "GiantHead" => Some(Self::GiantHead),
            "AwakenedOne" => Some(Self::AwakenedOne),
            "TimeEater" => Some(Self::TimeEater),
            "CorruptHeart" => Some(Self::CorruptHeart),
            "SpireShield" => Some(Self::SpireShield),
            "SpireSpear" => Some(Self::SpireSpear),
            "SnakePlant" => Some(Self::SnakePlant),
            // Already matched as display name above:
            // Cultist, Byrd, Centurion, Chosen, Mugger, Darkling, Exploder,
            // Repulsor, Spiker, Transient, Deca, Donu, Nemesis, Reptomancer, etc.
            _ => None,
        }
    }
}

impl fmt::Display for MonsterId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.display_name())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_display_name_roundtrip() {
        // Every display name should resolve back to the same MonsterId
        let all = [
            MonsterId::AcidSlime_L, MonsterId::JawWorm, MonsterId::Cultist,
            MonsterId::ShieldGremlin, MonsterId::GreenLouse, MonsterId::Guardian,
            MonsterId::Champ, MonsterId::SpireGrowth, MonsterId::Mystic,
            MonsterId::CorruptHeart, MonsterId::WrithingMass,
        ];
        for id in &all {
            let name = id.display_name();
            let resolved = MonsterId::from_str_fuzzy(name);
            assert_eq!(resolved, Some(*id), "Display name '{}' didn't resolve back to {:?}", name, id);
        }
    }

    #[test]
    fn test_java_id_resolution() {
        // Java IDs that differ from display names
        assert_eq!(MonsterId::from_str_fuzzy("GremlinTsundere"), Some(MonsterId::ShieldGremlin));
        assert_eq!(MonsterId::from_str_fuzzy("Serpent"), Some(MonsterId::SpireGrowth));
        assert_eq!(MonsterId::from_str_fuzzy("Healer"), Some(MonsterId::Mystic));
        assert_eq!(MonsterId::from_str_fuzzy("FuzzyLouseDefensive"), Some(MonsterId::GreenLouse));
        assert_eq!(MonsterId::from_str_fuzzy("SlaverBoss"), Some(MonsterId::Taskmaster));
        assert_eq!(MonsterId::from_str_fuzzy("TheGuardian"), Some(MonsterId::Guardian));
    }

    #[test]
    fn test_unknown_returns_none() {
        assert_eq!(MonsterId::from_str_fuzzy("NonExistentMonster"), None);
    }
}
