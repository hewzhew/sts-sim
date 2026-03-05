"""Generate the complete monsters_verified.json with all Act 1 monsters.
Uses data verified manually from Java source code."""
import json

data = {"_meta": {
    "schema": "v5",
    "rules": [
        "R1: All values uniform type: damage/amount is int or {min,max}, never array",
        "R2: Move keys are Java byte IDs as strings. name field is human-readable only",
        "R3: HP is always {min, max} object",
        "R4: Effect uses id (not power), amount is Optional (omit if N/A), target is self|player",
        "R5: Cards use id/amount/destination - symmetric with effects",
        "R6: No metadata inside moves/effects/cards. Root _notes only (underscore = skip on deserialize)",
        "R7: Ascension overrides are cumulative: asc4 inherits base, asc9 inherits asc4, etc",
        "R8: Rust types: DynamicValue = Fixed(i32) | Range{min,max}; amount = Option<DynamicValue>",
        "R9: Each entry has id field = Rust MonsterId enum variant name (= top-level key)"
    ]
}}

def R(lo, hi):
    """Range object."""
    return {"min": lo, "max": hi}

def eff(id, amount=None, target="self"):
    """Effect entry."""
    e = {"id": id, "target": target}
    if amount is not None:
        e["amount"] = amount
    return e

def card(id, amount, dest):
    """Card entry."""
    return {"id": id, "amount": amount, "destination": dest}

# ====================== ACT 1 — NORMAL ======================

data["AcidSlime_L"] = {
    "id": "AcidSlime_L", "name": "Acid Slime (L)", "java_id": "AcidSlime_L",
    "type": "normal", "act": 1,
    "hp": R(65, 69),
    "moves": {
        "1": {"name": "Corrosive Spit", "damage": 11,
              "cards": [card("Slimed", 2, "discard")]},
        "2": {"name": "Tackle", "damage": 16},
        "3": {"name": "Split"},
        "4": {"name": "Lick", "effects": [eff("Weak", 2, "player")]}
    },
    "pre_battle": [eff("Split", target="self")],
    "ascension": {
        "2": {"moves": {"1": {"damage": 12}, "2": {"damage": 18}}},
        "7": {"hp": R(68, 72)}
    }
}

data["AcidSlime_M"] = {
    "id": "AcidSlime_M", "name": "Acid Slime (M)", "java_id": "AcidSlime_M",
    "type": "normal", "act": 1,
    "hp": R(28, 32),
    "moves": {
        "1": {"name": "Corrosive Spit", "damage": 7,
              "cards": [card("Slimed", 1, "discard")]},
        "2": {"name": "Tackle", "damage": 10},
        "4": {"name": "Lick", "effects": [eff("Weak", 1, "player")]}
    },
    "ascension": {
        "2": {"moves": {"1": {"damage": 8}, "2": {"damage": 12}}},
        "7": {"hp": R(29, 34)}
    }
}

data["AcidSlime_S"] = {
    "id": "AcidSlime_S", "name": "Acid Slime (S)", "java_id": "AcidSlime_S",
    "type": "normal", "act": 1,
    "hp": R(8, 12),
    "moves": {
        "1": {"name": "Tackle", "damage": 3},
        "2": {"name": "Lick", "effects": [eff("Weak", 1, "player")]}
    },
    "ascension": {
        "2": {"moves": {"1": {"damage": 4}}},
        "7": {"hp": R(9, 13)}
    }
}

data["Cultist"] = {
    "id": "Cultist", "name": "Cultist", "java_id": "Cultist",
    "type": "normal", "act": 1,
    "hp": R(48, 54),
    "moves": {
        "1": {"name": "Dark Strike", "damage": 6},
        "3": {"name": "Incantation", "effects": [eff("Ritual", 3, "self")]}
    },
    "ascension": {
        "2": {"moves": {"3": {"effects": [eff("Ritual", 4, "self")]}}},
        "7": {"hp": R(50, 56)},
        "17": {"moves": {"3": {"effects": [eff("Ritual", 5, "self")]}}}
    }
}

data["FatGremlin"] = {
    "id": "FatGremlin", "name": "Fat Gremlin", "java_id": "GremlinFat",
    "type": "normal", "act": 1,
    "hp": R(13, 17),
    "moves": {
        "2": {"name": "Smear", "damage": 4,
              "effects": [eff("Weak", 1, "player")]}
    },
    "ascension": {
        "2": {"moves": {"2": {"damage": 5}}},
        "7": {"hp": R(14, 18)},
        "17": {"moves": {"2": {"damage": 5,
               "effects": [eff("Weak", 1, "player"), eff("Frail", 1, "player")]}}}
    }
}

data["FungiBeast"] = {
    "id": "FungiBeast", "name": "Fungi Beast", "java_id": "FungiBeast",
    "type": "normal", "act": 1,
    "hp": R(22, 28),
    "moves": {
        "1": {"name": "Bite", "damage": 6},
        "2": {"name": "Grow", "effects": [eff("Strength", 3, "self")]}
    },
    "pre_battle": [eff("SporeCloud", 2, "self")],
    "ascension": {
        "2": {"moves": {"2": {"effects": [eff("Strength", 4, "self")]}}},
        "7": {"hp": R(24, 28)},
        "17": {"moves": {"2": {"effects": [eff("Strength", 5, "self")]}}}
    },
    "_notes": "Asc17 grow = strAmt+1 in takeTurn. Base strAmt=3, asc2 strAmt=4."
}

data["GreenLouse"] = {
    "id": "GreenLouse", "name": "Green Louse", "java_id": "FuzzyLouseDefensive",
    "type": "normal", "act": 1,
    "hp": R(11, 17),
    "moves": {
        "3": {"name": "Bite", "damage": R(5, 7)},
        "4": {"name": "Spit Web", "effects": [eff("Weak", 2, "player")]}
    },
    "pre_battle": [eff("CurlUp", R(3, 7), "self")],
    "ascension": {
        "2": {"moves": {"3": {"damage": R(6, 8)}}},
        "7": {"hp": R(12, 18), "pre_battle": [eff("CurlUp", R(4, 8), "self")]},
        "17": {"pre_battle": [eff("CurlUp", R(9, 12), "self")]}
    }
}

data["JawWorm"] = {
    "id": "JawWorm", "name": "Jaw Worm", "java_id": "JawWorm",
    "type": "normal", "act": 1,
    "hp": R(40, 44),
    "moves": {
        "1": {"name": "Chomp", "damage": 11},
        "3": {"name": "Thrash", "damage": 7, "block": 5},
        "2": {"name": "Bellow", "block": 6,
              "effects": [eff("Strength", 3, "self")]}
    },
    "ascension": {
        "2": {"moves": {"1": {"damage": 12},
              "2": {"effects": [eff("Strength", 4, "self")]}}},
        "7": {"hp": R(42, 46)},
        "17": {"moves": {"2": {"block": 9,
               "effects": [eff("Strength", 5, "self")]}}}
    }
}

data["Looter"] = {
    "id": "Looter", "name": "Looter", "java_id": "Looter",
    "type": "normal", "act": 1,
    "hp": R(44, 48),
    "moves": {
        "1": {"name": "Mug", "damage": 10},
        "2": {"name": "Smoke Bomb", "block": 6},
        "3": {"name": "Escape"},
        "4": {"name": "Lunge", "damage": 12}
    },
    "pre_battle": [eff("Thievery", 15, "self")],
    "ascension": {
        "2": {"moves": {"1": {"damage": 11}, "4": {"damage": 14}}},
        "7": {"hp": R(46, 50)},
        "17": {"pre_battle": [eff("Thievery", 20, "self")]}
    },
    "_notes": "Thievery steals gold on unblocked attack damage."
}

data["MadGremlin"] = {
    "id": "MadGremlin", "name": "Mad Gremlin", "java_id": "GremlinWarrior",
    "type": "normal", "act": 1,
    "hp": R(20, 24),
    "moves": {
        "1": {"name": "Scratch", "damage": 4}
    },
    "pre_battle": [eff("Angry", 1, "self")],
    "ascension": {
        "2": {"moves": {"1": {"damage": 5}}},
        "7": {"hp": R(21, 25)},
        "17": {"pre_battle": [eff("Angry", 2, "self")]}
    }
}

data["RedLouse"] = {
    "id": "RedLouse", "name": "Red Louse", "java_id": "FuzzyLouseNormal",
    "type": "normal", "act": 1,
    "hp": R(10, 15),
    "moves": {
        "3": {"name": "Bite", "damage": R(5, 7)},
        "4": {"name": "Grow", "effects": [eff("Strength", 3, "self")]}
    },
    "pre_battle": [eff("CurlUp", R(3, 7), "self")],
    "ascension": {
        "2": {"moves": {"3": {"damage": R(6, 8)}}},
        "7": {"hp": R(11, 16), "pre_battle": [eff("CurlUp", R(4, 8), "self")]},
        "17": {"moves": {"4": {"effects": [eff("Strength", 4, "self")]}},
               "pre_battle": [eff("CurlUp", R(9, 12), "self")]}
    }
}

data["SneakyGremlin"] = {
    "id": "SneakyGremlin", "name": "Sneaky Gremlin", "java_id": "GremlinThief",
    "type": "normal", "act": 1,
    "hp": R(10, 14),
    "moves": {
        "1": {"name": "Puncture", "damage": 9}
    },
    "ascension": {
        "2": {"moves": {"1": {"damage": 10}}},
        "7": {"hp": R(11, 15)}
    }
}

data["ShieldGremlin"] = {
    "id": "ShieldGremlin", "name": "Shield Gremlin", "java_id": "GremlinTsundere",
    "type": "normal", "act": 1,
    "hp": R(12, 15),
    "moves": {
        "1": {"name": "Protect", "block": 7},
        "2": {"name": "Shield Bash", "damage": 6}
    },
    "ascension": {
        "2": {"moves": {"2": {"damage": 8}}},
        "7": {"hp": R(13, 17), "moves": {"1": {"block": 8}}},
        "17": {"moves": {"1": {"block": 11}}}
    },
    "_notes": "Protect gives block to a random ally. Falls back to Shield Bash when alone."
}

data["GremlinWizard"] = {
    "id": "GremlinWizard", "name": "Gremlin Wizard", "java_id": "GremlinWizard",
    "type": "normal", "act": 1,
    "hp": R(21, 25),
    "moves": {
        "2": {"name": "Charging"},
        "1": {"name": "Ultimate Blast", "damage": 25}
    },
    "ascension": {
        "2": {"moves": {"1": {"damage": 30}}},
        "7": {"hp": R(22, 26)}
    },
    "_notes": "Charges 2 turns then fires. Asc17: no charge after firing, attacks every turn."
}

data["SpikeSlime_L"] = {
    "id": "SpikeSlime_L", "name": "Spike Slime (L)", "java_id": "SpikeSlime_L",
    "type": "normal", "act": 1,
    "hp": R(64, 70),
    "moves": {
        "1": {"name": "Flame Tackle", "damage": 16,
              "cards": [card("Slimed", 2, "discard")]},
        "3": {"name": "Split"},
        "4": {"name": "Lick", "effects": [eff("Frail", 2, "player")]}
    },
    "pre_battle": [eff("Split", target="self")],
    "ascension": {
        "2": {"moves": {"1": {"damage": 18}}},
        "7": {"hp": R(67, 73)},
        "17": {"moves": {"4": {"effects": [eff("Frail", 3, "player")]}}}
    }
}

data["SpikeSlime_M"] = {
    "id": "SpikeSlime_M", "name": "Spike Slime (M)", "java_id": "SpikeSlime_M",
    "type": "normal", "act": 1,
    "hp": R(28, 32),
    "moves": {
        "1": {"name": "Flame Tackle", "damage": 8,
              "cards": [card("Slimed", 1, "discard")]},
        "4": {"name": "Lick", "effects": [eff("Frail", 1, "player")]}
    },
    "ascension": {
        "2": {"moves": {"1": {"damage": 10}}},
        "7": {"hp": R(29, 34)}
    }
}

data["SpikeSlime_S"] = {
    "id": "SpikeSlime_S", "name": "Spike Slime (S)", "java_id": "SpikeSlime_S",
    "type": "normal", "act": 1,
    "hp": R(10, 14),
    "moves": {
        "1": {"name": "Tackle", "damage": 5}
    },
    "ascension": {
        "2": {"moves": {"1": {"damage": 6}}},
        "7": {"hp": R(11, 15)}
    }
}

data["BlueSlaver"] = {
    "id": "BlueSlaver", "name": "Blue Slaver", "java_id": "SlaverBlue",
    "type": "normal", "act": 1,
    "hp": R(46, 50),
    "moves": {
        "1": {"name": "Stab", "damage": 12},
        "4": {"name": "Rake", "damage": 7,
              "effects": [eff("Weak", 1, "player")]}
    },
    "ascension": {
        "2": {"moves": {"1": {"damage": 13}, "4": {"damage": 8}}},
        "7": {"hp": R(48, 52)},
        "17": {"moves": {"4": {"damage": 8,
               "effects": [eff("Weak", 2, "player")]}}}
    }
}

data["RedSlaver"] = {
    "id": "RedSlaver", "name": "Red Slaver", "java_id": "SlaverRed",
    "type": "normal", "act": 1,
    "hp": R(46, 50),
    "moves": {
        "1": {"name": "Stab", "damage": 13},
        "2": {"name": "Entangle", "effects": [eff("Entangle", target="player")]},
        "3": {"name": "Scrape", "damage": 8,
              "effects": [eff("Vulnerable", 1, "player")]}
    },
    "ascension": {
        "2": {"moves": {"1": {"damage": 14}, "3": {"damage": 9}}},
        "7": {"hp": R(48, 52)},
        "17": {"moves": {"3": {"damage": 9,
               "effects": [eff("Vulnerable", 2, "player")]}}}
    },
    "_notes": "Entangle used at most once. No amount needed."
}

# ====================== ACT 1 — ELITE ======================

data["GremlinNob"] = {
    "id": "GremlinNob", "name": "Gremlin Nob", "java_id": "GremlinNob",
    "type": "elite", "act": 1,
    "hp": R(82, 86),
    "moves": {
        "3": {"name": "Bellow", "effects": [eff("Enrage", 2, "self")]},
        "1": {"name": "Rush", "damage": 14},
        "2": {"name": "Skull Bash", "damage": 6,
              "effects": [eff("Vulnerable", 2, "player")]}
    },
    "ascension": {
        "3": {"moves": {"3": {"effects": [eff("Enrage", 3, "self")]},
              "1": {"damage": 16}, "2": {"damage": 8}}},
        "8": {"hp": R(85, 90)},
        "18": {"moves": {"3": {"effects": [eff("Enrage", 4, "self")]}}}
    },
    "_notes": "Enrage = gain Str each time player plays a Skill."
}

data["Lagavulin"] = {
    "id": "Lagavulin", "name": "Lagavulin", "java_id": "Lagavulin",
    "type": "elite", "act": 1,
    "hp": R(109, 111),
    "moves": {
        "3": {"name": "Attack", "damage": 18},
        "1": {"name": "Siphon Soul",
              "effects": [eff("Dexterity", -1, "player"), eff("Strength", -1, "player")]}
    },
    "pre_battle": [eff("Metallicize", 8, "self")],
    "ascension": {
        "3": {"moves": {"3": {"damage": 20},
              "1": {"effects": [eff("Dexterity", -2, "player"), eff("Strength", -2, "player")]}}},
        "8": {"hp": R(112, 115)},
        "18": {"pre_battle": [eff("Metallicize", 10, "self")]}
    },
    "_notes": "Starts asleep with Metallicize. Wakes after 3 turns or on damage."
}

data["Sentry"] = {
    "id": "Sentry", "name": "Sentry", "java_id": "Sentry",
    "type": "elite", "act": 1,
    "hp": R(38, 42),
    "moves": {
        "3": {"name": "Bolt", "cards": [card("Dazed", 2, "discard")]},
        "4": {"name": "Beam", "damage": 9}
    },
    "pre_battle": [eff("Artifact", 1, "self")],
    "ascension": {
        "3": {"moves": {"4": {"damage": 10}}},
        "8": {"hp": R(39, 45)},
        "18": {"moves": {"3": {"cards": [card("Dazed", 3, "discard")]}}}
    }
}

# ====================== ACT 1 — BOSS ======================

data["Guardian"] = {
    "id": "Guardian", "name": "The Guardian", "java_id": "TheGuardian",
    "type": "boss", "act": 1,
    "hp": R(240, 240),
    "moves": {
        "5": {"name": "Whirlwind", "damage": 5, "hits": 4},
        "2": {"name": "Fierce Bash", "damage": 32},
        "7": {"name": "Vent Steam",
              "effects": [eff("Weak", 2, "player"), eff("Vulnerable", 2, "player")]},
        "6": {"name": "Charge Up"},
        "3": {"name": "Roll Attack", "damage": 9},
        "4": {"name": "Twin Slam", "damage": 8, "hits": 2},
        "1": {"name": "Close Up"}
    },
    "pre_battle": [eff("ModeShift", 10, "self")],
    "ascension": {
        "4": {"moves": {"2": {"damage": 36}, "4": {"damage": 9}}},
        "9": {"hp": R(250, 250)},
        "19": {"pre_battle": [eff("ModeShift", 15, "self")],
               "moves": {"7": {"effects": [eff("Weak", 3, "player"), eff("Vulnerable", 3, "player")]}}}
    },
    "_notes": "Has offensive/defensive mode cycle. Sharp Hide 3 (asc19: 4) applied in Close Up."
}

data["Hexaghost"] = {
    "id": "Hexaghost", "name": "Hexaghost", "java_id": "Hexaghost",
    "type": "boss", "act": 1,
    "hp": R(250, 250),
    "moves": {
        "5": {"name": "Activate"},
        "1": {"name": "Divider", "damage": 0},
        "4": {"name": "Sear", "damage": 6,
              "cards": [card("Burn", 1, "discard")]},
        "2": {"name": "Tackle", "damage": 5, "hits": 2},
        "3": {"name": "Inflame", "effects": [eff("Strength", 2, "self")]},
        "6": {"name": "Inferno", "damage": 2, "hits": 6,
              "cards": [card("Burn", 3, "discard")]}
    },
    "ascension": {
        "4": {"moves": {"3": {"effects": [eff("Strength", 3, "self")]},
              "2": {"damage": 6}}},
        "9": {"hp": R(264, 264)},
        "19": {"moves": {"6": {"cards": [card("BurnPlus", 3, "discard")]}}}
    },
    "_notes": "Divider damage = (player_HP / 12 + 1). Inferno upgraded burns at asc19."
}

data["SlimeBoss"] = {
    "id": "SlimeBoss", "name": "Slime Boss", "java_id": "SlimeBoss",
    "type": "boss", "act": 1,
    "hp": R(140, 140),
    "moves": {
        "1": {"name": "Slam", "damage": 35},
        "2": {"name": "Preparing"},
        "3": {"name": "Split"},
        "4": {"name": "Goop Spray", "cards": [card("Slimed", 3, "discard")]}
    },
    "pre_battle": [eff("Split", target="self")],
    "ascension": {
        "4": {"moves": {"1": {"damage": 38}}},
        "9": {"hp": R(150, 150)},
        "19": {"moves": {"4": {"cards": [card("Slimed", 5, "discard")]}}}
    }
}

# ====================== ACT 2 — NORMAL ======================

data["Bear"] = {
    "id": "Bear", "name": "Bear", "java_id": "BanditBear",
    "type": "normal", "act": 2,
    "hp": R(38, 42),
    "moves": {
        "1": {"name": "Maul", "damage": 18},
        "2": {"name": "Bear Hug", "effects": [eff("Dexterity", -2, "player")]},
        "3": {"name": "Lunge", "damage": 9, "block": 9}
    },
    "ascension": {
        "2": {"moves": {"1": {"damage": 20}, "3": {"damage": 10}}},
        "7": {"hp": R(40, 44)},
        "17": {"moves": {"2": {"effects": [eff("Dexterity", -4, "player")]}}}
    },
    "_notes": "Always starts with Bear Hug. Cycle: BearHug -> Lunge -> Maul -> Lunge -> Maul..."
}

data["BookOfStabbing"] = {
    "id": "BookOfStabbing", "name": "Book of Stabbing", "java_id": "BookOfStabbing",
    "type": "elite", "act": 2,
    "hp": R(160, 164),
    "moves": {
        "1": {"name": "Multi-Stab", "damage": 6, "hits": 2},
        "2": {"name": "Single Stab", "damage": 21}
    },
    "pre_battle": [eff("PainfulStabs", target="self")],
    "ascension": {
        "3": {"moves": {"1": {"damage": 7}, "2": {"damage": 24}}},
        "8": {"hp": R(168, 172)},
        "18": {"moves": {"1": {"hits": 3}}}
    },
    "_notes": "Multi-stab hits increase by 1 each use. PainfulStabs = shuffle Wound on attack."
}

data["BronzeOrb"] = {
    "id": "BronzeOrb", "name": "Bronze Orb", "java_id": "BronzeOrb",
    "type": "normal", "act": 2,
    "hp": R(52, 58),
    "moves": {
        "1": {"name": "Beam", "damage": 8},
        "2": {"name": "Support Beam", "damage": 8, "block": 0},
        "3": {"name": "Stasis"}
    },
    "ascension": {
        "9": {"hp": R(54, 60)}
    },
    "_notes": "Stasis steals a card. SupportBeam block = 2x # orbs alive. Spawned by BronzeAutomaton."
}

data["Byrd"] = {
    "id": "Byrd", "name": "Byrd", "java_id": "Byrd",
    "type": "normal", "act": 2,
    "hp": R(25, 31),
    "moves": {
        "1": {"name": "Peck", "damage": 1, "hits": 5},
        "2": {"name": "Fly"},
        "3": {"name": "Swoop", "damage": 12},
        "4": {"name": "Stunned"},
        "5": {"name": "Headbutt", "damage": 3},
        "6": {"name": "Caw", "effects": [eff("Strength", 1, "self")]}
    },
    "pre_battle": [eff("Flight", 3, "self")],
    "ascension": {
        "2": {"moves": {"1": {"hits": 6}, "3": {"damage": 14}}},
        "7": {"hp": R(26, 33)},
        "17": {"pre_battle": [eff("Flight", 4, "self")]}
    },
    "_notes": "Starts flying. Grounded = stunned, then headbutt -> fly again."
}

data["Centurion"] = {
    "id": "Centurion", "name": "Centurion", "java_id": "Centurion",
    "type": "normal", "act": 2,
    "hp": R(76, 80),
    "moves": {
        "1": {"name": "Slash", "damage": 12},
        "2": {"name": "Defend", "block": 15},
        "3": {"name": "Fury", "damage": 6, "hits": 3}
    },
    "ascension": {
        "2": {"moves": {"1": {"damage": 14}, "3": {"damage": 7}}},
        "7": {"hp": R(78, 83)},
        "17": {"moves": {"2": {"block": 20}}}
    },
    "_notes": "Defend gives block to random ally (usually Mystic). When alone, uses Fury instead."
}

data["Chosen"] = {
    "id": "Chosen", "name": "Chosen", "java_id": "Chosen",
    "type": "normal", "act": 2,
    "hp": R(95, 99),
    "moves": {
        "5": {"name": "Poke", "damage": 5, "hits": 2},
        "1": {"name": "Zap", "damage": 18},
        "2": {"name": "Drain", "effects": [eff("Weak", 3, "player"), eff("Strength", 3, "self")]},
        "3": {"name": "Debilitate", "damage": 10, "effects": [eff("Vulnerable", 2, "player")]},
        "4": {"name": "Hex", "effects": [eff("Hex", 1, "player")]}
    },
    "ascension": {
        "2": {"moves": {"5": {"damage": 6}, "1": {"damage": 21}, "3": {"damage": 12}}},
        "7": {"hp": R(98, 103)}
    },
    "_notes": "Always uses Hex first. Then Poke, then cycles attacks."
}

data["Mugger"] = {
    "id": "Mugger", "name": "Mugger", "java_id": "Mugger",
    "type": "normal", "act": 2,
    "hp": R(48, 52),
    "moves": {
        "1": {"name": "Mug", "damage": 10},
        "2": {"name": "Smoke Bomb", "block": 11},
        "3": {"name": "Escape"},
        "4": {"name": "Lunge", "damage": 16}
    },
    "pre_battle": [eff("Thievery", 15, "self")],
    "ascension": {
        "2": {"moves": {"1": {"damage": 11}, "4": {"damage": 18}}},
        "7": {"hp": R(50, 54)},
        "17": {"moves": {"2": {"block": 17}}, "pre_battle": [eff("Thievery", 20, "self")]}
    }
}

data["Mystic"] = {
    "id": "Mystic", "name": "Mystic", "java_id": "Healer",
    "type": "normal", "act": 2,
    "hp": R(48, 56),
    "moves": {
        "1": {"name": "Attack", "damage": 8, "effects": [eff("Frail", 2, "player")]},
        "2": {"name": "Heal"},
        "3": {"name": "Buff", "effects": [eff("Strength", 2, "self")]}
    },
    "ascension": {
        "2": {"moves": {"1": {"damage": 9}, "3": {"effects": [eff("Strength", 3, "self")]}}},
        "7": {"hp": R(50, 58)},
        "17": {"moves": {"3": {"effects": [eff("Strength", 4, "self")]}}}
    },
    "_notes": "Heal heals all allies for 16 (asc17: 20). Buff gives Str to all allies."
}

data["Pointy"] = {
    "id": "Pointy", "name": "Pointy", "java_id": "BanditChild",
    "type": "normal", "act": 2,
    "hp": R(30, 30),
    "moves": {
        "1": {"name": "Attack", "damage": 5, "hits": 2}
    },
    "ascension": {
        "2": {"moves": {"1": {"damage": 6}}},
        "7": {"hp": R(34, 34)}
    }
}

data["Romeo"] = {
    "id": "Romeo", "name": "Romeo", "java_id": "BanditLeader",
    "type": "normal", "act": 2,
    "hp": R(35, 39),
    "moves": {
        "1": {"name": "Cross Slash", "damage": 15},
        "2": {"name": "Mock"},
        "3": {"name": "Agonizing Slash", "damage": 10, "effects": [eff("Weak", 2, "player")]}
    },
    "ascension": {
        "2": {"moves": {"1": {"damage": 17}, "3": {"damage": 12}}},
        "7": {"hp": R(37, 41)},
        "17": {"moves": {"3": {"effects": [eff("Weak", 3, "player")]}}}
    },
    "_notes": "Always starts with Mock. Cycle: Mock -> AgonizingSlash -> CrossSlash."
}

data["ShelledParasite"] = {
    "id": "ShelledParasite", "name": "Shelled Parasite", "java_id": "Shelled Parasite",
    "type": "normal", "act": 2,
    "hp": R(68, 72),
    "moves": {
        "1": {"name": "Fell", "damage": 18, "effects": [eff("Frail", 2, "player")]},
        "2": {"name": "Double Strike", "damage": 6, "hits": 2},
        "3": {"name": "Suck", "damage": 10},
        "4": {"name": "Stunned"}
    },
    "pre_battle": [eff("PlatedArmor", 14, "self")],
    "ascension": {
        "2": {"moves": {"1": {"damage": 21}, "2": {"damage": 7}, "3": {"damage": 12}}},
        "7": {"hp": R(70, 75)},
        "17": {"pre_battle": [eff("PlatedArmor", 19, "self")]}
    },
    "_notes": "Suck heals self for damage dealt."
}

data["SnakePlant"] = {
    "id": "SnakePlant", "name": "Snake Plant", "java_id": "SnakePlant",
    "type": "normal", "act": 2,
    "hp": R(75, 79),
    "moves": {
        "1": {"name": "Chomp", "damage": 7, "hits": 3},
        "2": {"name": "Enfeebling Spores", "effects": [eff("Frail", 2, "player"), eff("Weak", 2, "player")]}
    },
    "pre_battle": [eff("Malleable", target="self")],
    "ascension": {
        "2": {"moves": {"1": {"damage": 8}}},
        "7": {"hp": R(78, 82)},
        "17": {"moves": {"2": {"effects": [eff("Frail", 3, "player"), eff("Weak", 3, "player")]}}}
    }
}

data["Snecko"] = {
    "id": "Snecko", "name": "Snecko", "java_id": "Snecko",
    "type": "normal", "act": 2,
    "hp": R(114, 120),
    "moves": {
        "1": {"name": "Perplexing Glare", "effects": [eff("Confused", target="player")]},
        "2": {"name": "Bite", "damage": 15},
        "3": {"name": "Tail Whip", "effects": [eff("Vulnerable", 2, "player"), eff("Weak", 2, "player")]}
    },
    "ascension": {
        "2": {"moves": {"2": {"damage": 18}}},
        "7": {"hp": R(120, 125)},
        "17": {"moves": {"3": {"effects": [eff("Vulnerable", 3, "player"), eff("Weak", 2, "player")]}}}
    },
    "_notes": "Always uses Glare first."
}

data["SphericGuardian"] = {
    "id": "SphericGuardian", "name": "Spheric Guardian", "java_id": "SphericGuardian",
    "type": "normal", "act": 2,
    "hp": R(20, 20),
    "moves": {
        "1": {"name": "Slam", "damage": 10, "hits": 2},
        "2": {"name": "Activate", "block": 25},
        "3": {"name": "Harden", "damage": 10, "block": 15},
        "4": {"name": "Attack+Debuff", "damage": 10, "effects": [eff("Frail", 5, "player")]}
    },
    "pre_battle": [eff("Artifact", 1, "self"), eff("Barricade", target="self")],
    "ascension": {
        "2": {"moves": {"1": {"damage": 11}, "3": {"damage": 11}, "4": {"damage": 11}}},
        "17": {"pre_battle": [eff("Artifact", 2, "self"), eff("Barricade", target="self")]}
    },
    "_notes": "Starts with Activate for block, then cycles attacks. Barricade = block persists."
}

data["Taskmaster"] = {
    "id": "Taskmaster", "name": "Taskmaster", "java_id": "SlaverBoss",
    "type": "elite", "act": 2,
    "hp": R(54, 60),
    "moves": {
        "2": {"name": "Scouring Whip", "damage": 7,
              "cards": [card("Wound", 1, "discard")]}
    },
    "ascension": {
        "3": {"moves": {"2": {"damage": 8}}},
        "8": {"hp": R(57, 64)},
        "18": {"moves": {"2": {"cards": [card("Wound", 2, "discard")]}}}
    },
    "_notes": "Always uses Scouring Whip. Spawns RedSlaver + BlueSlaver."
}

data["TorchHead"] = {
    "id": "TorchHead", "name": "Torch Head", "java_id": "TorchHead",
    "type": "normal", "act": 2,
    "hp": R(38, 40),
    "moves": {
        "1": {"name": "Tackle", "damage": 7}
    },
    "ascension": {
        "9": {"hp": R(40, 45)}
    },
    "_notes": "Minion of BronzeAutomaton. Single move only."
}

# ====================== ACT 2 — ELITE ======================

data["GremlinLeader"] = {
    "id": "GremlinLeader", "name": "Gremlin Leader", "java_id": "GremlinLeader",
    "type": "elite", "act": 2,
    "hp": R(140, 148),
    "moves": {
        "2": {"name": "Rally!", "effects": [eff("Strength", 3, "self")]},
        "3": {"name": "Encourage", "block": 6, "effects": [eff("Strength", 1, "self")]},
        "4": {"name": "Stab", "damage": 6, "hits": 3}
    },
    "pre_battle": [eff("Minion", target="self")],
    "ascension": {
        "3": {"moves": {"4": {"damage": 7}, "2": {"effects": [eff("Strength", 4, "self")]}}},
        "8": {"hp": R(145, 155)},
        "18": {"moves": {"3": {"block": 10, "effects": [eff("Strength", 2, "self")]}}}
    },
    "_notes": "Encourage gives block+Str to all gremlins. Rally spawns new gremlins."
}

# ====================== ACT 2 — BOSS ======================

data["Champ"] = {
    "id": "Champ", "name": "The Champ", "java_id": "Champ",
    "type": "boss", "act": 2,
    "hp": R(420, 420),
    "moves": {
        "4": {"name": "Face Slap", "damage": 12, "effects": [eff("Frail", 2, "player")]},
        "1": {"name": "Heavy Slash", "damage": 16},
        "2": {"name": "Defensive Stance", "block": 15, "effects": [eff("Metallicize", 5, "self")]},
        "3": {"name": "Execute", "damage": 10, "hits": 2},
        "5": {"name": "Gloat", "effects": [eff("Strength", 2, "self")]},
        "6": {"name": "Taunt", "effects": [eff("Vulnerable", 2, "player"), eff("Weak", 2, "player")]},
        "7": {"name": "Anger", "effects": [eff("RemoveDebuffs", target="self"), eff("Strength", 6, "self")]}
    },
    "ascension": {
        "4": {"moves": {"4": {"damage": 14}, "1": {"damage": 18}, "3": {"damage": 12}}},
        "9": {"hp": R(440, 440)},
        "19": {"moves": {"2": {"effects": [eff("Metallicize", 7, "self")]},
               "5": {"effects": [eff("Strength", 3, "self")]},
               "6": {"effects": [eff("Vulnerable", 3, "player"), eff("Weak", 3, "player")]}}}
    },
    "_notes": "Anger Str = strAmt * 3. Also removes Shackled."
}

data["Collector"] = {
    "id": "Collector", "name": "The Collector", "java_id": "TheCollector",
    "type": "boss", "act": 2,
    "hp": R(282, 282),
    "moves": {
        "1": {"name": "Spawn"},
        "2": {"name": "Fireball", "damage": 18},
        "3": {"name": "Buff", "effects": [eff("Strength", 3, "self")]},
        "4": {"name": "Mega Debuff", "effects": [eff("Vulnerable", 3, "player"), eff("Weak", 3, "player"), eff("Frail", 3, "player")]},
        "5": {"name": "Revive"}
    },
    "pre_battle": [eff("Minion", target="self")],
    "ascension": {
        "4": {"moves": {"2": {"damage": 21}, "3": {"effects": [eff("Strength", 4, "self")]}}},
        "9": {"hp": R(300, 300)},
        "19": {"moves": {"4": {"effects": [eff("Vulnerable", 5, "player"), eff("Weak", 5, "player"), eff("Frail", 5, "player")]}}}
    },
    "_notes": "Spawns 2 torchheads. Revive if minions dead. Mega Debuff on first turn."
}

data["BronzeAutomaton"] = {
    "id": "BronzeAutomaton", "name": "Bronze Automaton", "java_id": "BronzeAutomaton",
    "type": "boss", "act": 2,
    "hp": R(300, 300),
    "moves": {
        "4": {"name": "Spawn Orbs"},
        "1": {"name": "Flail", "damage": 7, "hits": 2},
        "2": {"name": "Hyper Beam", "damage": 45},
        "3": {"name": "Stunned"},
        "5": {"name": "Boost", "block": 9, "effects": [eff("Strength", 3, "self")]}
    },
    "pre_battle": [eff("Artifact", 3, "self")],
    "ascension": {
        "4": {"moves": {"1": {"damage": 8}, "2": {"damage": 50}, "5": {"effects": [eff("Strength", 4, "self")]}}},
        "9": {"hp": R(320, 320)},
        "19": {"pre_battle": [eff("Artifact", 4, "self")]}
    },
    "_notes": "Cycles: Boost -> Flail -> Flail -> HyperBeam -> Stunned -> repeat."
}

# ====================== ACT 3 — NORMAL ======================

data["Darkling"] = {
    "id": "Darkling", "name": "Darkling", "java_id": "Darkling",
    "type": "normal", "act": 3,
    "hp": R(48, 56),
    "moves": {
        "1": {"name": "Chomp", "damage": 8, "hits": 2},
        "2": {"name": "Harden", "block": 12, "effects": [eff("Regrow", target="self")]},
        "3": {"name": "Nip", "damage": 7},
        "5": {"name": "Reincarnate"}
    },
    "pre_battle": [eff("Regrow", target="self")],
    "ascension": {
        "2": {"moves": {"1": {"damage": 9}, "3": {"damage": 8}}},
        "7": {"hp": R(50, 59)},
        "17": {"moves": {"2": {"block": 15}}}
    },
    "_notes": "Revives with half HP when killed unless all Darklings die simultaneously."
}

data["Exploder"] = {
    "id": "Exploder", "name": "Exploder", "java_id": "Exploder",
    "type": "normal", "act": 3,
    "hp": R(30, 30),
    "moves": {
        "1": {"name": "Slam", "damage": 9}
    },
    "pre_battle": [eff("Explosive", 30, "self")],
    "ascension": {
        "2": {"moves": {"1": {"damage": 11}}},
        "7": {"hp": R(30, 35)}
    },
    "_notes": "Explodes for 30 damage after 3 turns if not killed."
}

data["Maw"] = {
    "id": "Maw", "name": "The Maw", "java_id": "Maw",
    "type": "normal", "act": 3,
    "hp": R(300, 300),
    "moves": {
        "2": {"name": "Roar", "effects": [eff("Weak", 3, "player"), eff("Frail", 3, "player")]},
        "3": {"name": "Slam", "damage": 25},
        "4": {"name": "Drool", "effects": [eff("Strength", 3, "self")]},
        "5": {"name": "Nom Nom", "damage": 5}
    },
    "ascension": {
        "2": {"moves": {"3": {"damage": 30}, "5": {"damage": 7}}},
        "17": {"moves": {"4": {"effects": [eff("Strength", 5, "self")]},
               "2": {"effects": [eff("Weak", 5, "player"), eff("Frail", 5, "player")]}}}
    },
    "_notes": "Roar first. NomNom hits = turnCount/2 (int div, turnCount starts 1, increments each getMove)."
}

data["OrbWalker"] = {
    "id": "OrbWalker", "name": "Orb Walker", "java_id": "Orb Walker",
    "type": "normal", "act": 3,
    "hp": R(90, 96),
    "moves": {
        "1": {"name": "Laser", "damage": 10, "cards": [card("Burn", 1, "discard")]},
        "2": {"name": "Claw", "damage": 15}
    },
    "pre_battle": [eff("GenericStrengthUp", target="self")],
    "ascension": {
        "2": {"moves": {"1": {"damage": 11}, "2": {"damage": 16}}},
        "7": {"hp": R(92, 102)},
        "17": {"moves": {"1": {"cards": [card("Burn", 2, "discard")]}}}
    },
    "_notes": "GenericStrengthUp = gain 3 Str each turn."
}

data["Repulsor"] = {
    "id": "Repulsor", "name": "Repulsor", "java_id": "Repulsor",
    "type": "normal", "act": 3,
    "hp": R(29, 35),
    "moves": {
        "1": {"name": "Bash", "cards": [card("Dazed", 2, "discard")]},
        "2": {"name": "Attack", "damage": 11}
    },
    "ascension": {
        "2": {"moves": {"2": {"damage": 13}}},
        "7": {"hp": R(31, 38)}
    }
}

data["Spiker"] = {
    "id": "Spiker", "name": "Spiker", "java_id": "Spiker",
    "type": "normal", "act": 3,
    "hp": R(42, 56),
    "moves": {
        "1": {"name": "Cut", "damage": 5, "hits": 2},
        "2": {"name": "Spike", "effects": [eff("Thorns", 2, "self")]}
    },
    "pre_battle": [eff("Thorns", 3, "self")],
    "ascension": {
        "2": {"moves": {"1": {"damage": 6}}},
        "7": {"hp": R(44, 60)},
        "17": {"moves": {"2": {"effects": [eff("Thorns", 3, "self")]}}}
    }
}

data["SpireGrowth"] = {
    "id": "SpireGrowth", "name": "Spire Growth", "java_id": "Serpent",
    "type": "normal", "act": 3,
    "hp": R(170, 170),
    "moves": {
        "1": {"name": "Quick Tackle", "damage": 16},
        "2": {"name": "Constrict", "effects": [eff("Constricted", 10, "player")]},
        "3": {"name": "Smash", "damage": 22}
    },
    "ascension": {
        "2": {"moves": {"1": {"damage": 18}, "3": {"damage": 25}}},
        "7": {"hp": R(190, 190)},
        "17": {"moves": {"2": {"effects": [eff("Constricted", 12, "player")]}}}
    }
}

data["Transient"] = {
    "id": "Transient", "name": "Transient", "java_id": "Transient",
    "type": "normal", "act": 3,
    "hp": R(999, 999),
    "moves": {
        "1": {"name": "Attack", "damage": 30}
    },
    "pre_battle": [eff("Shifting", target="self"), eff("Fading", 5, "self")],
    "ascension": {
        "2": {"moves": {"1": {"damage": 40}}},
        "17": {"moves": {"1": {"damage": 50}}}
    },
    "_notes": "Fading = dies after 5 turns. Shifting = damage increases by 10 each turn."
}

data["WrithingMass"] = {
    "id": "WrithingMass", "name": "Writhing Mass", "java_id": "WrithingMass",
    "type": "normal", "act": 3,
    "hp": R(160, 160),
    "moves": {
        "0": {"name": "Strong Strike", "damage": 32},
        "1": {"name": "Multi-Strike", "damage": 9, "hits": 3},
        "2": {"name": "Flail", "damage": 16, "block": 16},
        "3": {"name": "Wither", "damage": 12, "effects": [eff("Weak", 2, "player"), eff("Vulnerable", 2, "player")]},
        "4": {"name": "Mega Debuff", "effects": [eff("Weak", 2, "player"), eff("Vulnerable", 2, "player"), eff("Frail", 2, "player")]}
    },
    "pre_battle": [eff("Reactive", target="self"), eff("Malleable", target="self")],
    "ascension": {
        "2": {"moves": {"0": {"damage": 38}, "1": {"hits": 4},
              "2": {"block": 18}, "3": {"damage": 15},
              "4": {"effects": [eff("Weak", 3, "player"), eff("Vulnerable", 3, "player"), eff("Frail", 3, "player")]}}},
        "7": {"hp": R(175, 175)}
    },
    "_notes": "Randomly picks move each turn. Reactive = applies Weak/Vulnerable on attack."
}

# ====================== ACT 3 — ELITE ======================

data["GiantHead"] = {
    "id": "GiantHead", "name": "Giant Head", "java_id": "GiantHead",
    "type": "elite", "act": 3,
    "hp": R(500, 500),
    "moves": {
        "1": {"name": "Glare", "effects": [eff("Weak", 1, "player")]},
        "2": {"name": "It Is Time", "damage": 40},
        "3": {"name": "Count"}
    },
    "pre_battle": [eff("Slow", target="self")],
    "ascension": {
        "3": {"moves": {"2": {"damage": 50}}},
        "8": {"hp": R(520, 520)},
        "18": {"pre_battle": [eff("Slow", target="self")],
               "moves": {"1": {"effects": [eff("Weak", 2, "player")]}}}
    },
    "_notes": "Count (move 3) increments internal counter. Attacks with 'It Is Time' after 5 turns."
}

data["Nemesis"] = {
    "id": "Nemesis", "name": "Nemesis", "java_id": "Nemesis",
    "type": "elite", "act": 3,
    "hp": R(185, 185),
    "moves": {
        "2": {"name": "Debuff", "damage": 6, "hits": 3, "effects": [eff("Burn", target="player")]},
        "3": {"name": "Scythe", "damage": 45},
        "4": {"name": "Attack", "damage": 7, "hits": 2, "cards": [card("Burn", 3, "discard")]}
    },
    "end_turn_effects": [eff("Intangible", 1, "self")],
    "ascension": {
        "3": {"moves": {"3": {"damage": 50}}},
        "8": {"hp": R(200, 200)},
        "18": {"moves": {"4": {"cards": [card("Burn", 5, "discard")]}}}
    },
    "_notes": "Intangible only applied if not already present."
}

data["Reptomancer"] = {
    "id": "Reptomancer", "name": "Reptomancer", "java_id": "Reptomancer",
    "type": "elite", "act": 3,
    "hp": R(180, 190),
    "moves": {
        "1": {"name": "Snake Strike", "damage": 13, "hits": 2, "effects": [eff("Weak", 1, "player")]},
        "2": {"name": "Summon"},
        "3": {"name": "Big Bite", "damage": 30}
    },
    "pre_battle": [eff("Minion", target="self")],
    "ascension": {
        "3": {"moves": {"1": {"damage": 16}, "3": {"damage": 34}}},
        "8": {"hp": R(190, 200)},
        "18": {"moves": {"3": {"damage": 34}}}
    },
    "_notes": "Summons SnakeDagger minions. Always summons first."
}

# ====================== ACT 3 — BOSS ======================

data["AwakenedOne"] = {
    "id": "AwakenedOne", "name": "Awakened One", "java_id": "AwakenedOne",
    "type": "boss", "act": 3,
    "hp": R(300, 300),
    "moves": {
        "1": {"name": "Slash", "damage": 20},
        "2": {"name": "Soul Strike", "damage": 6, "hits": 4},
        "3": {"name": "Rebirth"},
        "5": {"name": "Dark Echo", "damage": 40},
        "6": {"name": "Sludge", "damage": 18, "cards": [card("Void", 1, "discard")]},
        "8": {"name": "Tackle", "damage": 10, "hits": 3}
    },
    "pre_battle": [eff("Curiosity", target="self")],
    "ascension": {
        "9": {"hp": R(320, 320)}
    },
    "_notes": "Phase 1: Slash/SoulStrike + Curiosity. Phase 2 after Rebirth: DarkEcho/Sludge/Tackle. Rebirth at full HP."
}

data["Deca"] = {
    "id": "Deca", "name": "Deca", "java_id": "Deca",
    "type": "boss", "act": 3,
    "hp": R(250, 250),
    "moves": {
        "0": {"name": "Beam", "damage": 10, "hits": 2, "cards": [card("Dazed", 2, "discard")]},
        "2": {"name": "Square of Protection", "block": 16,
              "effects": [eff("PlatedArmor", 3, "self")]}
    },
    "pre_battle": [eff("Artifact", 2, "self")],
    "ascension": {
        "4": {"moves": {"0": {"damage": 12}}},
        "9": {"hp": R(265, 265)},
        "19": {"pre_battle": [eff("Artifact", 3, "self")]}
    }
}

data["Donu"] = {
    "id": "Donu", "name": "Donu", "java_id": "Donu",
    "type": "boss", "act": 3,
    "hp": R(250, 250),
    "moves": {
        "0": {"name": "Beam", "damage": 10, "hits": 2},
        "2": {"name": "Circle of Power", "effects": [eff("Strength", 3, "self")]}
    },
    "pre_battle": [eff("Artifact", 2, "self")],
    "ascension": {
        "4": {"moves": {"0": {"damage": 12}, "2": {"effects": [eff("Strength", 4, "self")]}}},
        "9": {"hp": R(265, 265)},
        "19": {"pre_battle": [eff("Artifact", 3, "self")]}
    }
}

data["TimeEater"] = {
    "id": "TimeEater", "name": "Time Eater", "java_id": "TimeEater",
    "type": "boss", "act": 3,
    "hp": R(456, 456),
    "moves": {
        "2": {"name": "Reverberate", "damage": 7, "hits": 3},
        "3": {"name": "Ripple", "damage": 8, "block": 2},
        "4": {"name": "Head Slam", "damage": 26,
              "effects": [eff("DrawReduction", 1, "player")],
              "cards": [card("Slimed", 2, "discard")]},
        "5": {"name": "Haste", "effects": [eff("RemoveDebuffs", target="self"), eff("Heal", target="self")]}
    },
    "pre_battle": [eff("TimeWarp", target="self")],
    "ascension": {
        "4": {"moves": {"2": {"damage": 8}, "3": {"damage": 9}, "4": {"damage": 32}}},
        "9": {"hp": R(480, 480)}
    },
    "_notes": "TimeWarp: after 12 cards played, ends player turn + gains Str. Haste at half HP."
}

# ====================== ACT 4 ======================

data["CorruptHeart"] = {
    "id": "CorruptHeart", "name": "Corrupt Heart", "java_id": "CorruptHeart",
    "type": "boss", "act": 4,
    "hp": R(750, 750),
    "moves": {
        "1": {"name": "Blood Shots", "damage": 2, "hits": 12},
        "2": {"name": "Echo", "damage": 40},
        "3": {"name": "Debilitate",
              "effects": [eff("Vulnerable", 2, "player"), eff("Weak", 2, "player"), eff("Frail", 2, "player")],
              "cards": [card("Dazed", 1, "draw_pile"), card("Slimed", 1, "draw_pile"),
                        card("Wound", 1, "draw_pile"), card("Burn", 1, "draw_pile"),
                        card("Void", 1, "draw_pile")]},
        "4": {"name": "Buff", "effects": [eff("Artifact", 2, "self"), eff("BeatOfDeath", 1, "self"),
              eff("PainfulStabs", target="self"), eff("Strength", 2, "self")]}
    },
    "pre_battle": [eff("Invincible", 200, "self"), eff("BeatOfDeath", 1, "self")],
    "ascension": {
        "4": {"moves": {"1": {"hits": 15}, "2": {"damage": 45}}},
        "9": {"hp": R(800, 800)},
        "19": {"pre_battle": [eff("Invincible", 200, "self"), eff("BeatOfDeath", 2, "self")]}
    },
    "_notes": "Move 4 also removes negative Str first. Cycle-based extra buffs (Artifact/BeatOfDeath/PainfulStabs/Str+10/Str+50) are AI behavior in Rust."
}

data["SpireShield"] = {
    "id": "SpireShield", "name": "Spire Shield", "java_id": "SpireShield",
    "type": "elite", "act": 4,
    "hp": R(110, 110),
    "moves": {
        "1": {"name": "Bash", "damage": 12},
        "2": {"name": "Fortify", "block": 30},
        "3": {"name": "Smash", "damage": 34}
    },
    "pre_battle": [eff("Artifact", 1, "self"), eff("Surrounded", target="self")],
    "ascension": {
        "3": {"moves": {"1": {"damage": 14}, "3": {"damage": 38}}},
        "8": {"hp": R(125, 125)},
        "18": {"pre_battle": [eff("Artifact", 2, "self"), eff("Surrounded", target="self")]}
    }
}

data["SpireSpear"] = {
    "id": "SpireSpear", "name": "Spire Spear", "java_id": "SpireSpear",
    "type": "elite", "act": 4,
    "hp": R(160, 160),
    "moves": {
        "1": {"name": "Burn Strike", "damage": 5, "hits": 2, "cards": [card("Burn", 2, "discard")]},
        "2": {"name": "Piercer", "damage": 10, "effects": [eff("Strength", 2, "self")]},
        "3": {"name": "Skewer", "damage": 10, "hits": 3}
    },
    "pre_battle": [eff("Artifact", 1, "self"), eff("Surrounded", target="self")],
    "ascension": {
        "3": {"moves": {"1": {"damage": 6}, "3": {"hits": 4}}},
        "8": {"hp": R(180, 180)},
        "18": {"pre_battle": [eff("Artifact", 2, "self"), eff("Surrounded", target="self")]}
    }
}

# ====================== MINION ======================

data["Dagger"] = {
    "id": "Dagger", "name": "Dagger", "java_id": "Dagger",
    "type": "minion", "act": 3,
    "hp": R(20, 25),
    "moves": {
        "1": {"name": "Stab", "damage": 9, "cards": [card("Wound", 1, "discard")]},
        "2": {"name": "Explode", "damage": 25}
    },
    "_notes": "Reptomancer minion. No ascension scaling. First move = Stab, then Explode (kills self)."
}

# Write output
with open("data/monsters_verified.json", "w", encoding="utf-8") as f:
    json.dump(data, f, indent=2, ensure_ascii=False)

print(f"Written {len(data) - 1} monsters (excluding _meta)")
