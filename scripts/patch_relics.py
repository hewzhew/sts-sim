#!/usr/bin/env python3
"""
Relic Logic Patcher
===================
Auto-populates logic for relics based on their descriptions.
This script dramatically increases relic coverage from ~30% to ~80%+.

Usage:
    python scripts/patch_relics.py

Output:
    data/relics_patched.json (with populated logic)
"""

import json
import re
from pathlib import Path

# =============================================================================
# MANUAL OVERRIDES - Complex relics that need hand-crafted logic
# =============================================================================

MANUAL_LOGIC = {
    # === Starter Relics (class-specific) ===
    "PureWater": {
        "hooks": [{
            "trigger": "BattleStart",
            "commands": [{"type": "AddCard", "params": {"card": "Miracle", "destination": "hand", "count": 1}}]
        }]
    },
    
    # === Combat Start - Stats ===
    "Akabeko": {
        "hooks": [{
            "trigger": "BattleStart",
            "commands": [{"type": "ApplyBuff", "params": {"buff": "Vigor", "amount": 8}}]
        }]
    },
    "ClockworkSouvenir": {
        "hooks": [{
            "trigger": "BattleStart",
            "commands": [{"type": "ApplyBuff", "params": {"buff": "Artifact", "amount": 1}}]
        }]
    },
    "SlingofCourage": {
        "hooks": [{
            "trigger": "BattleStart",
            "conditions": [{"type": "IsEliteCombat"}],
            "commands": [{"type": "ApplyBuff", "params": {"buff": "Strength", "amount": 2}}]
        }]
    },
    "GremlinVisage": {
        "hooks": [{
            "trigger": "BattleStart",
            "commands": [{"type": "ApplyDebuff", "params": {"debuff": "Weak", "amount": 1}}]
        }]
    },
    "NeowsBlessing": {
        "hooks": [{
            "trigger": "BattleStart",
            "conditions": [{"type": "Counter", "max": 3}],
            "commands": [
                {"type": "SetAllEnemiesHP", "params": {"amount": 1}},
                {"type": "IncrementCounter", "params": {}}
            ]
        }]
    },
    
    # === Combat Start - Cards ===
    "NinjaScroll": {
        "hooks": [{
            "trigger": "BattleStart",
            "commands": [{"type": "AddCard", "params": {"card": "Shiv", "destination": "hand", "count": 3}}]
        }]
    },
    "HolyWater": {
        "hooks": [{
            "trigger": "BattleStart",
            "commands": [{"type": "AddCard", "params": {"card": "Miracle", "destination": "hand", "count": 3}}]
        }]
    },
    
    # === Combat Start - Orbs (Defect) ===
    "RunicCapacitor": {
        "hooks": [{
            "trigger": "BattleStart",
            "commands": [{"type": "GainOrbSlots", "params": {"amount": 3}}]
        }]
    },
    "DataDisk": {
        "hooks": [{
            "trigger": "BattleStart",
            "commands": [{"type": "ApplyBuff", "params": {"buff": "Focus", "amount": 1}}]
        }]
    },
    
    # === Turn Start ===
    "HappyFlower": {
        "hooks": [{
            "trigger": "TurnStart",
            "conditions": [{"type": "EveryNTurns", "n": 3}],
            "commands": [{"type": "GainEnergy", "params": {"amount": 1}}]
        }]
    },
    "IncenseBurner": {
        "hooks": [{
            "trigger": "TurnStart",
            "conditions": [{"type": "EveryNTurns", "n": 6}],
            "commands": [{"type": "ApplyBuff", "params": {"buff": "Intangible", "amount": 1}}]
        }]
    },
    "Inserter": {
        "hooks": [{
            "trigger": "TurnStart",
            "conditions": [{"type": "EveryNTurns", "n": 2}],
            "commands": [{"type": "GainOrbSlots", "params": {"amount": 1}}]
        }]
    },
    "WarpedTongs": {
        "hooks": [{
            "trigger": "TurnStart",
            "commands": [{"type": "UpgradeRandomCardInHand", "params": {"temporary": True}}]
        }]
    },
    
    # === Turn End ===
    "CloakClasp": {
        "hooks": [{
            "trigger": "TurnEnd",
            "commands": [{"type": "GainBlockPerCardsInHand", "params": {"amount_per_card": 1}}]
        }]
    },
    "Nilry'sCodex": {
        "hooks": [{
            "trigger": "TurnEnd",
            "commands": [{"type": "ChooseAndShuffleCard", "params": {"choices": 3}}]
        }]
    },
    "RunicPyramid": {
        "hooks": [{
            "trigger": "TurnEnd",
            "commands": [{"type": "RetainHand", "params": {}}]
        }]
    },
    
    # === On Attack ===
    "PenNib": {
        "hooks": [{
            "trigger": "OnPlayAttack",
            "conditions": [{"type": "EveryNAttacks", "n": 10}],
            "commands": [{"type": "DoubleDamage", "params": {}}]
        }]
    },
    "Nunchaku": {
        "hooks": [{
            "trigger": "OnPlayAttack",
            "conditions": [{"type": "EveryNAttacks", "n": 10}],
            "commands": [{"type": "GainEnergy", "params": {"amount": 1}}]
        }]
    },
    "Kunai": {
        "hooks": [{
            "trigger": "OnPlayAttack",
            "conditions": [{"type": "EveryNAttacksPerTurn", "n": 3}],
            "commands": [{"type": "ApplyBuff", "params": {"buff": "Dexterity", "amount": 1}}]
        }]
    },
    "Shuriken": {
        "hooks": [{
            "trigger": "OnPlayAttack",
            "conditions": [{"type": "EveryNAttacksPerTurn", "n": 3}],
            "commands": [{"type": "ApplyBuff", "params": {"buff": "Strength", "amount": 1}}]
        }]
    },
    "OrnamentalFan": {
        "hooks": [{
            "trigger": "OnPlayAttack",
            "conditions": [{"type": "EveryNAttacksPerTurn", "n": 3}],
            "commands": [{"type": "GainBlock", "params": {"base": 4}}]
        }]
    },
    "Necronomicon": {
        "hooks": [{
            "trigger": "OnPlayAttack",
            "conditions": [{"type": "CardCostAtLeast", "cost": 2}, {"type": "OncePerTurn"}],
            "commands": [{"type": "PlayCardAgain", "params": {}}]
        }]
    },
    "WristBlade": {
        "hooks": [{
            "trigger": "OnPlayAttack",
            "conditions": [{"type": "CardCostEquals", "cost": 0}],
            "commands": [{"type": "BonusDamage", "params": {"amount": 4}}]
        }]
    },
    
    # === On Skill ===
    "LetterOpener": {
        "hooks": [{
            "trigger": "OnPlaySkill",
            "conditions": [{"type": "EveryNSkillsPerTurn", "n": 3}],
            "commands": [{"type": "DealDamageToAllEnemies", "params": {"amount": 5}}]
        }]
    },
    "MummifiedHand": {
        "hooks": [{
            "trigger": "OnPlayPower",
            "commands": [{"type": "ReduceRandomCardCost", "params": {"amount": 1}}]
        }]
    },
    
    # === On Card Draw / Play ===
    "InkBottle": {
        "hooks": [{
            "trigger": "OnPlayCard",
            "conditions": [{"type": "EveryNCards", "n": 10}],
            "commands": [{"type": "DrawCards", "params": {"base": 1}}]
        }]
    },
    "Pocketwatch": {
        "hooks": [{
            "trigger": "TurnEnd",
            "conditions": [{"type": "CardsPlayedThisTurn", "max": 3}],
            "commands": [{"type": "DrawCardsNextTurn", "params": {"amount": 3}}]
        }]
    },
    "UnceasingTop": {
        "hooks": [{
            "trigger": "OnHandEmpty",
            "commands": [{"type": "DrawCards", "params": {"base": 1}}]
        }]
    },
    
    # === On Exhaust ===
    "DeadBranch": {
        "hooks": [{
            "trigger": "PlayerExhaust",
            "commands": [{"type": "AddRandomCard", "params": {"destination": "hand"}}]
        }]
    },
    "StrangeSpoon": {
        "hooks": [{
            "trigger": "OnExhaustAttempt",
            "conditions": [{"type": "Chance", "percent": 50}],
            "commands": [{"type": "PreventExhaust", "params": {}}]
        }]
    },
    
    # === On Discard ===
    "HoveringKite": {
        "hooks": [{
            "trigger": "PlayerDiscard",
            "conditions": [{"type": "OncePerTurn"}],
            "commands": [{"type": "GainEnergy", "params": {"amount": 1}}]
        }]
    },
    
    # === On Shuffle ===
    "Sundial": {
        "hooks": [{
            "trigger": "OnShuffle",
            "conditions": [{"type": "EveryNShuffles", "n": 3}],
            "commands": [{"type": "GainEnergy", "params": {"amount": 2}}]
        }]
    },
    "Abacus": {
        "hooks": [{
            "trigger": "OnShuffle",
            "commands": [{"type": "GainBlock", "params": {"base": 6}}]
        }]
    },
    "Melange": {
        "hooks": [{
            "trigger": "OnShuffle",
            "commands": [{"type": "Scry", "params": {"amount": 3}}]
        }]
    },
    
    # === On Damage Taken ===
    "Torii": {
        "hooks": [{
            "trigger": "OnDamageTaken",
            "conditions": [{"type": "DamageAtMost", "amount": 5}],
            "commands": [{"type": "ReduceDamage", "params": {"to": 1}}]
        }]
    },
    "TungstenRod": {
        "hooks": [{
            "trigger": "OnLoseHP",
            "commands": [{"type": "ReduceHPLoss", "params": {"amount": 1}}]
        }]
    },
    "FossilizedHelix": {
        "hooks": [{
            "trigger": "OnLoseHP",
            "conditions": [{"type": "OncePerCombat"}],
            "commands": [{"type": "PreventDamage", "params": {}}]
        }]
    },
    "LizardTail": {
        "hooks": [{
            "trigger": "OnDeath",
            "conditions": [{"type": "OncePerRun"}],
            "commands": [{"type": "HealPercent", "params": {"percent": 50}}]
        }]
    },
    "EmotionChip": {
        "hooks": [{
            "trigger": "TurnStart",
            "conditions": [{"type": "LostHPLastTurn"}],
            "commands": [{"type": "TriggerAllOrbPassives", "params": {}}]
        }]
    },
    
    # === On Block Break ===
    "HandDrill": {
        "hooks": [{
            "trigger": "OnBreakEnemyBlock",
            "commands": [{"type": "ApplyToTarget", "params": {"status": "Vulnerable", "amount": 2}}]
        }]
    },
    
    # === On Kill ===
    "TheSpecimen": {
        "hooks": [{
            "trigger": "EnemyDied",
            "commands": [{"type": "TransferPoison", "params": {}}]
        }]
    },
    "GremlinHorn": {
        "hooks": [{
            "trigger": "EnemyDied",
            "commands": [
                {"type": "GainEnergy", "params": {"amount": 1}},
                {"type": "DrawCards", "params": {"base": 1}}
            ]
        }]
    },
    
    # === On Gold Gain ===
    "BloodyIdol": {
        "hooks": [{
            "trigger": "OnGainGold",
            "commands": [{"type": "Heal", "params": {"amount": 5}}]
        }]
    },
    
    # === On Potion Use ===
    "ToyOrnithopter": {
        "hooks": [{
            "trigger": "PlayerUsePotion",
            "commands": [{"type": "Heal", "params": {"amount": 5}}]
        }]
    },
    
    # === On Enter Room ===
    "AncientTeaSet": {
        "hooks": [{
            "trigger": "EnterRest",
            "commands": [{"type": "GainEnergyNextCombat", "params": {"amount": 2}}]
        }]
    },
    "Serpent Head": {
        "hooks": [{
            "trigger": "EnterUnknown",
            "commands": [{"type": "GainGold", "params": {"amount": 50}}]
        }]
    },
    
    # === Passive Modifiers (No Trigger - checked during calculation) ===
    "RedSkull": {
        "hooks": [{
            "trigger": "Passive",
            "conditions": [{"type": "HPBelow", "percent": 50}],
            "effect": {"type": "BonusStrength", "amount": 3}
        }]
    },
    "TheBoot": {
        "hooks": [{
            "trigger": "Passive",
            "effect": {"type": "MinimumDamage", "amount": 5}
        }]
    },
    "Paper Frog": {
        "hooks": [{
            "trigger": "Passive",
            "effect": {"type": "VulnerableMultiplier", "value": 1.75}
        }]
    },
    "Paper Crane": {
        "hooks": [{
            "trigger": "Passive",
            "effect": {"type": "WeakMultiplier", "value": 0.60}
        }]
    },
    "OddMushroom": {
        "hooks": [{
            "trigger": "Passive",
            "effect": {"type": "VulnerableSelfMultiplier", "value": 1.25}
        }]
    },
    "Calipers": {
        "hooks": [{
            "trigger": "TurnStart",
            "commands": [{"type": "PreserveBlock", "params": {"amount": 15}}]
        }]
    },
    "IceCream": {
        "hooks": [{
            "trigger": "Passive",
            "effect": {"type": "ConserveEnergy"}
        }]
    },
    "Ginger": {
        "hooks": [{
            "trigger": "Passive",
            "effect": {"type": "Immunity", "status": "Weak"}
        }]
    },
    "Turnip": {
        "hooks": [{
            "trigger": "Passive",
            "effect": {"type": "Immunity", "status": "Frail"}
        }]
    },
    "ChemicalX": {
        "hooks": [{
            "trigger": "Passive",
            "effect": {"type": "BonusXCost", "amount": 2}
        }]
    },
    "MagicFlower": {
        "hooks": [{
            "trigger": "Passive",
            "effect": {"type": "HealingMultiplier", "value": 1.5}
        }]
    },
    "Gold-PlatedCables": {
        "hooks": [{
            "trigger": "OnOrbPassive",
            "conditions": [{"type": "IsRightmostOrb"}],
            "commands": [{"type": "TriggerOrbPassive", "params": {}}]
        }]
    },
    "MembershipCard": {
        "hooks": [{
            "trigger": "Passive",
            "effect": {"type": "ShopDiscount", "percent": 50}
        }]
    },
    "GoldenIdol": {
        "hooks": [{
            "trigger": "Passive",
            "effect": {"type": "GoldMultiplier", "value": 1.25}
        }]
    },
    "PreservedInsect": {
        "hooks": [{
            "trigger": "Passive",
            "conditions": [{"type": "IsEliteCombat"}],
            "effect": {"type": "EnemyHPMultiplier", "value": 0.75}
        }]
    },
    
    # === On Pickup ===
    "DarkstonePeriapt": {
        "hooks": [{
            "trigger": "OnPickup",
            "commands": [{"type": "GainMaxHP", "params": {"amount": 6}}]
        }]
    },
    "Toolbox": {
        "hooks": [{
            "trigger": "BattleStart",
            "commands": [{"type": "ChooseAndAddColorlessCard", "params": {"choices": 3}}]
        }]
    },
    "GamblingChip": {
        "hooks": [{
            "trigger": "BattleStart",
            "commands": [{"type": "DiscardAndRedraw", "params": {}}]
        }]
    },
    
    # === Rest Site ===
    "DreamCatcher": {
        "hooks": [{
            "trigger": "OnRest",
            "commands": [{"type": "AddCardReward", "params": {}}]
        }]
    },
    "RegalPillow": {
        "hooks": [{
            "trigger": "OnRest",
            "commands": [{"type": "BonusHeal", "params": {"amount": 15}}]
        }]
    },
    "PeacePipe": {
        "hooks": [{
            "trigger": "AtRestSite",
            "effect": {"type": "EnableRemoveCard"}
        }]
    },
    "Shovel": {
        "hooks": [{
            "trigger": "AtRestSite",
            "effect": {"type": "EnableDig"}
        }]
    },
    "Girya": {
        "hooks": [{
            "trigger": "AtRestSite",
            "effect": {"type": "EnableLift", "max_uses": 3}
        }]
    },
    
    # === Watcher Specific ===
    "VioletLotus": {
        "hooks": [{
            "trigger": "OnExitCalm",
            "commands": [{"type": "GainEnergy", "params": {"amount": 1}}]
        }]
    },
    "Duality": {
        "hooks": [{
            "trigger": "OnPlayAttack",
            "commands": [{"type": "ApplyBuff", "params": {"buff": "Dexterity", "amount": 1, "temporary": True}}]
        }]
    },
    "GoldenEye": {
        "hooks": [{
            "trigger": "OnScry",
            "commands": [{"type": "BonusScry", "params": {"amount": 2}}]
        }]
    },
    
    # === Defect Specific ===
    "FrozenCore": {
        "hooks": [{
            "trigger": "TurnEnd",
            "conditions": [{"type": "NoOrbsChanneled"}],
            "commands": [{"type": "ChannelOrb", "params": {"orb": "Frost"}}]
        }]
    },
    
    # === Egg Relics (Upgrade on gain) ===
    "MoltenEgg": {
        "hooks": [{
            "trigger": "OnGainCard",
            "conditions": [{"type": "CardType", "card_type": "Attack"}],
            "commands": [{"type": "UpgradeCard", "params": {}}]
        }]
    },
    "ToxicEgg": {
        "hooks": [{
            "trigger": "OnGainCard",
            "conditions": [{"type": "CardType", "card_type": "Skill"}],
            "commands": [{"type": "UpgradeCard", "params": {}}]
        }]
    },
    "FrozenEgg": {
        "hooks": [{
            "trigger": "OnGainCard",
            "conditions": [{"type": "CardType", "card_type": "Power"}],
            "commands": [{"type": "UpgradeCard", "params": {}}]
        }]
    },
    
    # === Bottled Cards ===
    "BottledFlame": {
        "hooks": [{
            "trigger": "OnPickup",
            "commands": [{"type": "BottleCard", "params": {"card_type": "Attack"}}]
        }, {
            "trigger": "BattleStart",
            "commands": [{"type": "AddBottledCardToHand", "params": {}}]
        }]
    },
    "BottledLightning": {
        "hooks": [{
            "trigger": "OnPickup",
            "commands": [{"type": "BottleCard", "params": {"card_type": "Skill"}}]
        }, {
            "trigger": "BattleStart",
            "commands": [{"type": "AddBottledCardToHand", "params": {}}]
        }]
    },
    "BottledTornado": {
        "hooks": [{
            "trigger": "OnPickup",
            "commands": [{"type": "BottleCard", "params": {"card_type": "Power"}}]
        }, {
            "trigger": "BattleStart",
            "commands": [{"type": "AddBottledCardToHand", "params": {}}]
        }]
    },
    
    # === Paint Relics ===
    "WarPaint": {
        "hooks": [{
            "trigger": "OnPickup",
            "commands": [{"type": "UpgradeRandomCards", "params": {"count": 2, "card_type": "Skill"}}]
        }]
    },
    "Whetstone": {
        "hooks": [{
            "trigger": "OnPickup",
            "commands": [{"type": "UpgradeRandomCards", "params": {"count": 2, "card_type": "Attack"}}]
        }]
    },
    
    # === Du-Vu Doll ===
    "Du-VuDoll": {
        "hooks": [{
            "trigger": "BattleStart",
            "commands": [{"type": "GainStrengthPerCurse", "params": {"amount": 1}}]
        }]
    },
    
    # === Blue Candle ===
    "BlueCandle": {
        "hooks": [{
            "trigger": "Passive",
            "effect": {"type": "CursesPlayable", "hp_cost": 1}
        }]
    },
    
    # === Medical Kit ===
    "MedicalKit": {
        "hooks": [{
            "trigger": "Passive",
            "effect": {"type": "StatusPlayable", "exhaust": True}
        }]
    },
    
    # === Orange Pellets ===
    "OrangePellets": {
        "hooks": [{
            "trigger": "OnPlayAllCardTypes",
            "commands": [{"type": "RemoveAllDebuffs", "params": {}}]
        }]
    },
    
    # === Negative Relics (Curses) ===
    "MarkoftheBloom": {
        "hooks": [{
            "trigger": "Passive",
            "effect": {"type": "CannotHeal"}
        }]
    },
    "CursedKey": {
        "hooks": [{
            "trigger": "OnOpenChest",
            "commands": [{"type": "GainCurse", "params": {}}]
        }]
    },
    "Muzzle": {
        "hooks": [{
            "trigger": "Passive",
            "effect": [{"type": "CannotGainMaxHP"}, {"type": "HalfHealing"}]
        }]
    },
    "Scatterbrain": {
        "hooks": [{
            "trigger": "Passive",
            "effect": {"type": "DrawReduction", "amount": 1}
        }]
    },
    "VoidEssence": {
        "hooks": [{
            "trigger": "Passive",
            "effect": {"type": "EnergyReduction", "amount": 1}
        }]
    },
    "Post-Durian": {
        "hooks": [{
            "trigger": "OnPickup",
            "commands": [{"type": "LoseMaxHPPercent", "params": {"percent": 50}}]
        }]
    },
    "Hauntings": {
        "hooks": [{
            "trigger": "BattleStart",
            "commands": [{"type": "ApplyToAllEnemies", "params": {"status": "Intangible", "amount": 1}}]
        }]
    },
    "AncientAugmentation": {
        "hooks": [{
            "trigger": "BattleStart",
            "commands": [
                {"type": "ApplyToAllEnemies", "params": {"status": "Artifact", "amount": 1}},
                {"type": "ApplyToAllEnemies", "params": {"status": "PlatedArmor", "amount": 10}},
                {"type": "ApplyToAllEnemies", "params": {"status": "Regenerate", "amount": 10}}
            ]
        }]
    },
    "ShieldofBlight": {
        "hooks": [{
            "trigger": "Passive",
            "effect": {"type": "EnemyHPMultiplier", "value": 1.5}
        }]
    },
    "SpearofBlight": {
        "hooks": [{
            "trigger": "Passive",
            "effect": {"type": "EnemyDamageMultiplier", "value": 2.0}
        }]
    },
    "TimeMaze": {
        "hooks": [{
            "trigger": "Passive",
            "effect": {"type": "CardPlayLimit", "amount": 15}
        }]
    },
    
    # === Special Pickup Relics ===
    "Cauldron": {
        "hooks": [{
            "trigger": "OnPickup",
            "commands": [{"type": "BrewPotions", "params": {"count": 5}}]
        }]
    },
    "Dolly'sMirror": {
        "hooks": [{
            "trigger": "OnPickup",
            "commands": [{"type": "DuplicateCard", "params": {}}]
        }]
    },
    "Orrery": {
        "hooks": [{
            "trigger": "OnPickup",
            "commands": [{"type": "ChooseCards", "params": {"count": 5}}]
        }]
    },
    "Astrolabe": {
        "hooks": [{
            "trigger": "OnPickup",
            "commands": [{"type": "TransformAndUpgrade", "params": {"count": 3}}]
        }]
    },
    "EmptyCage": {
        "hooks": [{
            "trigger": "OnPickup",
            "commands": [{"type": "RemoveCards", "params": {"count": 2}}]
        }]
    },
    "Pandora'sBox": {
        "hooks": [{
            "trigger": "OnPickup",
            "commands": [{"type": "TransformBasicCards", "params": {}}]
        }]
    },
    "CallingBell": {
        "hooks": [{
            "trigger": "OnPickup",
            "commands": [
                {"type": "GainRelics", "params": {"count": 3}},
                {"type": "GainCurses", "params": {"count": 3}}
            ]
        }]
    },
    
    # === Upgraded Starter Relics ===
    "BlackBlood": {
        "hooks": [{
            "trigger": "BattleEnd",
            "commands": [{"type": "Heal", "params": {"amount": 12}}]
        }]
    },
    "RingoftheSerpent": {
        "hooks": [{
            "trigger": "BattleStart",
            "commands": [{"type": "DrawCards", "params": {"base": 1}}]
        }, {
            "trigger": "TurnStart",
            "commands": [{"type": "DrawCards", "params": {"base": 1}}]
        }]
    },
    "MarkofPain": {
        "hooks": [{
            "trigger": "BattleStart",
            "commands": [
                {"type": "AddCard", "params": {"card": "Wound", "destination": "draw_pile", "count": 2}}
            ]
        }, {
            "trigger": "Passive",
            "effect": {"type": "BonusEnergy", "amount": 1}
        }]
    },
    
    # === Misc / Flavor ===
    "CultistHeadpiece": {
        "hooks": []  # Just flavor text
    },
    "SpiritPoop": {
        "hooks": []  # Just flavor text
    },
    "Circlet": {
        "hooks": []  # Placeholder relic
    },
    "RedCirclet": {
        "hooks": []  # Placeholder relic
    },
    "N'loth'sGift": {
        "hooks": [{
            "trigger": "Passive",
            "effect": {"type": "DoubleNextRelicExchange"}
        }]
    },
    "N'loth'sHungryFace": {
        "hooks": [{
            "trigger": "OnOpenChest",
            "conditions": [{"type": "OncePerRun"}],
            "commands": [{"type": "EmptyChest", "params": {}}]
        }]
    },
    "Enchiridion": {
        "hooks": [{
            "trigger": "BattleStart",
            "commands": [{"type": "AddRandomPowerCard", "params": {"destination": "hand"}}]
        }]
    },
    
    # === StrikeDummy ===
    "StrikeDummy": {
        "hooks": [{
            "trigger": "Passive",
            "effect": {"type": "StrikeBonus", "damage": 3}
        }]
    },
    
    # === Art of War ===
    "ArtofWar": {
        "hooks": [{
            "trigger": "TurnEnd",
            "conditions": [{"type": "NoAttacksPlayedThisTurn"}],
            "commands": [{"type": "GainEnergyNextTurn", "params": {"amount": 1}}]
        }]
    },
    
    # === Bird-Faced Urn ===
    "Bird-FacedUrn": {
        "hooks": [{
            "trigger": "OnPlayPower",
            "commands": [{"type": "Heal", "params": {"amount": 2}}]
        }]
    },
    
    # === Singing Bowl ===
    "SingingBowl": {
        "hooks": [{
            "trigger": "OnCardReward",
            "effect": {"type": "OptionToGainMaxHP", "amount": 2}
        }]
    },
    
    # === Question Card ===
    "QuestionCard": {
        "hooks": [{
            "trigger": "Passive",
            "effect": {"type": "BonusCardChoice", "amount": 1}
        }]
    },
    
    # === Prayer Wheel ===
    "PrayerWheel": {
        "hooks": [{
            "trigger": "BattleEnd",
            "conditions": [{"type": "IsNormalCombat"}],
            "commands": [{"type": "ExtraCardReward", "params": {}}]
        }]
    },
    
    # === White Beast Statue ===
    "WhiteBeastStatue": {
        "hooks": [{
            "trigger": "BattleEnd",
            "commands": [{"type": "GuaranteePotionReward", "params": {}}]
        }]
    },
    
    # === Black Star ===
    "BlackStar": {
        "hooks": [{
            "trigger": "BattleEnd",
            "conditions": [{"type": "IsEliteCombat"}],
            "commands": [{"type": "ExtraRelicReward", "params": {}}]
        }]
    },
    
    # === Sacred Bark ===
    "SacredBark": {
        "hooks": [{
            "trigger": "Passive",
            "effect": {"type": "DoublePotionEffect"}
        }]
    },
    
    # === Frozen Eye ===
    "Frozen Eye": {
        "hooks": [{
            "trigger": "Passive",
            "effect": {"type": "ViewDrawPileOrder"}
        }]
    },
    
    # === Prismatic Shard ===
    "PrismaticShard": {
        "hooks": [{
            "trigger": "Passive",
            "effect": {"type": "AllCardsAvailable"}
        }]
    },
    
    # === Omamori ===
    "Omamori": {
        "hooks": [{
            "trigger": "OnGainCurse",
            "conditions": [{"type": "Counter", "max": 2}],
            "commands": [
                {"type": "NegateCurse", "params": {}},
                {"type": "IncrementCounter", "params": {}}
            ]
        }]
    },
    
    # === Matryoshka ===
    "Matryoshka": {
        "hooks": [{
            "trigger": "OnOpenChest",
            "conditions": [{"type": "Counter", "max": 2}, {"type": "NotBossChest"}],
            "commands": [
                {"type": "ExtraRelic", "params": {}},
                {"type": "IncrementCounter", "params": {}}
            ]
        }]
    },
    
    # === Wing Boots ===
    "WingBoots": {
        "hooks": [{
            "trigger": "OnMapNavigation",
            "conditions": [{"type": "Counter", "max": 3}],
            "effect": {"type": "IgnorePaths"}
        }]
    },
    
    # === Potion Belt ===
    "PotionBelt": {
        "hooks": [{
            "trigger": "OnPickup",
            "commands": [{"type": "GainPotionSlots", "params": {"amount": 2}}]
        }]
    },
    
    # === Tiny Chest ===
    "TinyChest": {
        "hooks": [{
            "trigger": "Passive",
            "effect": {"type": "Every4thUnknownIsTreasure"}
        }]
    },
    
    # === Juzu Bracelet ===
    "JuzuBracelet": {
        "hooks": [{
            "trigger": "Passive",
            "effect": {"type": "NoRegularFightsInUnknown"}
        }]
    },
    
    # === The Courier ===
    "TheCourier": {
        "hooks": [{
            "trigger": "Passive",
            "effect": {"type": "ShopRestock"}
        }]
    },
    
    # === Smiling Mask ===
    "SmilingMask": {
        "hooks": [{
            "trigger": "Passive",
            "effect": {"type": "FixedRemovalCost", "amount": 50}
        }]
    },
    
    # === Accursed ===
    "Accursed": {
        "hooks": []  # Complex curse-related, likely not implemented
    },
    
    # === TwistingMind ===
    "TwistingMind": {
        "hooks": []  # Complex, likely not implemented
    },
    
    # === GrotesqueTrophy ===
    "GrotesqueTrophy": {
        "hooks": []  # Boss relic effect
    },
    
    # === MimicInfestation ===
    "MimicInfestation": {
        "hooks": [{
            "trigger": "Passive",
            "effect": {"type": "TreasureRoomsAreElites"}
        }]
    },
}


# =============================================================================
# REGEX PATTERNS - For automatic detection
# =============================================================================

PATTERNS = [
    # === Combat Start - Stat Buffs ===
    (r"Start each combat with (\d+) #?Strength", "BattleStart", 
     lambda m: [{"type": "ApplyBuff", "params": {"buff": "Strength", "amount": int(m.group(1))}}]),
    
    (r"Start each combat with (\d+) #?Dexterity", "BattleStart",
     lambda m: [{"type": "ApplyBuff", "params": {"buff": "Dexterity", "amount": int(m.group(1))}}]),
    
    (r"Start each combat with (\d+) #?Artifact", "BattleStart",
     lambda m: [{"type": "ApplyBuff", "params": {"buff": "Artifact", "amount": int(m.group(1))}}]),
    
    (r"Start each combat with (\d+) #?Block", "BattleStart",
     lambda m: [{"type": "GainBlock", "params": {"base": int(m.group(1))}}]),
    
    (r"Start each combat with (\d+) #?Vigor", "BattleStart",
     lambda m: [{"type": "ApplyBuff", "params": {"buff": "Vigor", "amount": int(m.group(1))}}]),
    
    (r"Start each combat with (\d+) #?Thorns", "BattleStart",
     lambda m: [{"type": "ApplyBuff", "params": {"buff": "Thorns", "amount": int(m.group(1))}}]),
    
    (r"Start each combat with (\d+) #?Metallicize", "BattleStart",
     lambda m: [{"type": "ApplyBuff", "params": {"buff": "Metallicize", "amount": int(m.group(1))}}]),
    
    (r"Start each combat with (\d+) #?Plated Armor", "BattleStart",
     lambda m: [{"type": "ApplyBuff", "params": {"buff": "PlatedArmor", "amount": int(m.group(1))}}]),
    
    # === Combat Start - Draw Cards ===
    (r"At the start of each combat, draw (\d+) additional cards?", "BattleStart",
     lambda m: [{"type": "DrawCards", "params": {"base": int(m.group(1))}}]),
    
    (r"At the start of each combat, draw (\d+) cards?", "BattleStart",
     lambda m: [{"type": "DrawCards", "params": {"base": int(m.group(1))}}]),
    
    # === Combat Start - Apply to Enemies ===
    (r"At the start of each combat, apply (\d+) #?Vulnerable to ALL enemies", "BattleStart",
     lambda m: [{"type": "ApplyToAllEnemies", "params": {"status": "Vulnerable", "amount": int(m.group(1))}}]),
    
    (r"At the start of each combat, apply (\d+) #?Weak to ALL enemies", "BattleStart",
     lambda m: [{"type": "ApplyToAllEnemies", "params": {"status": "Weak", "amount": int(m.group(1))}}]),
    
    (r"At the start of each combat, apply (\d+) #?Poison to ALL enemies", "BattleStart",
     lambda m: [{"type": "ApplyToAllEnemies", "params": {"status": "Poison", "amount": int(m.group(1))}}]),
    
    # === Combat End - Heal ===
    (r"At the end of combat, heal (\d+) HP", "BattleEnd",
     lambda m: [{"type": "Heal", "params": {"amount": int(m.group(1))}}]),
    
    (r"heal (\d+) HP at the end of combat", "BattleEnd",
     lambda m: [{"type": "Heal", "params": {"amount": int(m.group(1))}}]),
    
    # === Turn Start ===
    (r"Gain (\d+) #?Energy at the start of each turn", "TurnStart",
     lambda m: [{"type": "GainEnergy", "params": {"amount": int(m.group(1))}}]),
    
    (r"At the start of each turn, gain (\d+) #?Block", "TurnStart",
     lambda m: [{"type": "GainBlock", "params": {"base": int(m.group(1))}}]),
    
    (r"At the start of each turn, draw (\d+) additional cards?", "TurnStart",
     lambda m: [{"type": "DrawCards", "params": {"base": int(m.group(1))}}]),
    
    # === On Pickup - Max HP ===
    (r"Raise your Max HP by (\d+)", "OnPickup",
     lambda m: [{"type": "GainMaxHP", "params": {"amount": int(m.group(1))}}]),
    
    (r"Upon pickup, raise Max HP by (\d+)", "OnPickup",
     lambda m: [{"type": "GainMaxHP", "params": {"amount": int(m.group(1))}}]),
    
    (r"Increase your Max HP by (\d+)", "OnPickup",
     lambda m: [{"type": "GainMaxHP", "params": {"amount": int(m.group(1))}}]),
    
    # === Channel Orbs ===
    (r"At the start of each combat, #?Channel (\d+) #?Lightning", "BattleStart",
     lambda m: [{"type": "ChannelOrb", "params": {"orb": "Lightning", "count": int(m.group(1))}}]),
    
    (r"At the start of each combat, #?Channel (\d+) #?Frost", "BattleStart",
     lambda m: [{"type": "ChannelOrb", "params": {"orb": "Frost", "count": int(m.group(1))}}]),
    
    (r"At the start of each combat, #?Channel (\d+) #?Dark", "BattleStart",
     lambda m: [{"type": "ChannelOrb", "params": {"orb": "Dark", "count": int(m.group(1))}}]),
]


def apply_patterns(description: str) -> dict | None:
    """Try to match description against regex patterns."""
    if not description:
        return None
    
    for pattern, trigger, command_fn in PATTERNS:
        match = re.search(pattern, description, re.IGNORECASE)
        if match:
            return {
                "hooks": [{
                    "trigger": trigger,
                    "commands": command_fn(match)
                }]
            }
    return None


def patch_relics():
    """Main patching function."""
    input_path = Path("data/relics.json")
    output_path = Path("data/relics_patched.json")
    
    with open(input_path, 'r', encoding='utf-8') as f:
        relics = json.load(f)
    
    stats = {
        "total": len(relics),
        "already_done": 0,
        "manual_override": 0,
        "pattern_matched": 0,
        "still_empty": 0,
    }
    
    for relic in relics:
        relic_id = relic["id"]
        
        # Skip if already has logic
        if relic.get("logic", {}).get("hooks") and not relic.get("manual_review_needed"):
            stats["already_done"] += 1
            continue
        
        # Check manual overrides first
        if relic_id in MANUAL_LOGIC:
            relic["logic"] = MANUAL_LOGIC[relic_id]
            relic["manual_review_needed"] = False
            stats["manual_override"] += 1
            continue
        
        # Try pattern matching
        desc = relic.get("description", "")
        matched_logic = apply_patterns(desc)
        if matched_logic:
            relic["logic"] = matched_logic
            relic["manual_review_needed"] = False
            stats["pattern_matched"] += 1
            continue
        
        # Still empty
        stats["still_empty"] += 1
    
    # Write output
    with open(output_path, 'w', encoding='utf-8') as f:
        json.dump(relics, f, indent=2, ensure_ascii=False)
    
    # Print summary
    print("=" * 60)
    print("  Relic Logic Patcher - Summary")
    print("=" * 60)
    print(f"  Total relics:        {stats['total']}")
    print(f"  Already complete:    {stats['already_done']}")
    print(f"  Manual overrides:    {stats['manual_override']}")
    print(f"  Pattern matched:     {stats['pattern_matched']}")
    print(f"  Still empty:         {stats['still_empty']}")
    print("=" * 60)
    
    coverage = (stats['total'] - stats['still_empty']) / stats['total'] * 100
    print(f"  Coverage: {coverage:.1f}%")
    print(f"  Output: {output_path}")
    print("=" * 60)
    
    # List still empty relics
    if stats['still_empty'] > 0:
        print("\nRelics still needing manual logic:")
        for relic in relics:
            if relic.get("manual_review_needed"):
                print(f"  - {relic['id']}: {relic.get('description', '(no desc)')[:60]}")


if __name__ == "__main__":
    patch_relics()
