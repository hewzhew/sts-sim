#![allow(unused_imports)]

use super::super::*;
use super::*;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub const TYPED_ACTION_PAYLOAD_SOURCE_CLASSES: &[&str] = &[
    "AddCardToDeckAction",
    "ApplyPoisonOnRandomMonsterAction",
    "ApplyPowerAction",
    "ApplyPowerToRandomEnemyAction",
    "AttackDamageRandomEnemyAction",
    "BetterDiscardPileToHandAction",
    "BetterDrawPileToHandAction",
    "BurnIncreaseAction",
    "ChooseOneColorless",
    "ConditionalDrawAction",
    "CodexAction",
    "DamageAction",
    "DamageAllEnemiesAction",
    "DamageRandomEnemyAction",
    "DiscardToHandAction",
    "DiscardAction",
    "DiscardSpecificCardAction",
    "DiscoveryAction",
    "DrawCardAction",
    "DrawPileToHandAction",
    "EmptyDeckShuffleAction",
    "ExhaustAction",
    "ExhaustSpecificCardAction",
    "ExhaustToHandAction",
    "ForeignInfluenceAction",
    "GainEnergyAction",
    "MakeTempCardInDiscardAction",
    "MakeTempCardInDiscardAndDeckAction",
    "MakeTempCardInDrawPileAction",
    "MakeTempCardInHandAction",
    "ModifyBlockAction",
    "NewQueueCardAction",
    "ObtainPotionAction",
    "PlayTopCardAction",
    "PutOnBottomOfDeckAction",
    "PutOnDeckAction",
    "PummelDamageAction",
    "QueueCardAction",
    "ReApplyPowersAction",
    "ReduceCostAction",
    "ReduceCostForTurnAction",
    "ReducePowerAction",
    "RemoveSpecificPowerAction",
    "ResetFlagsAction",
    "ReviveMonsterAction",
    "RollMoveAction",
    "ScryAction",
    "SetMoveAction",
    "SetDontTriggerAction",
    "ShowCardAction",
    "ShowCardAndPoofAction",
    "SpawnMonsterAction",
    "SuicideAction",
    "TransformCardInHandAction",
    "UnlimboAction",
    "UpdateCardDescriptionAction",
    "UseCardAction",
];

pub const NO_EXTRA_ACTION_PAYLOAD_SOURCE_CLASSES: &[&str] = &[
    "EscapeAction",
    "ExhaustAllEtherealAction",
    "GainBlockAction",
    "GainGoldAction",
    "HandCheckAction",
    "HealAction",
    "InstantKillAction",
    "LoseBlockAction",
    "LoseHPAction",
    "LosePercentHPAction",
    "MakeTempCardAtBottomOfDeckAction",
    "RemoveAllBlockAction",
];

/// Java action classes whose update methods only drive UI, VFX, audio, hover,
/// text, or pacing. The Rust AI simulator must consume/drop them instead of
/// exposing them as mechanical queued actions.
pub const RENDER_ONLY_ACTION_SOURCE_CLASSES: &[&str] = &[
    "HideHealthBarAction",
    "SFXAction",
    "ShakeScreenAction",
    "TextAboveCreatureAction",
    "TextCenteredAction",
    "UnhoverCardAction",
    "WaitAction",
];

/// Java VFX/UI classes that do not mutate AI-relevant mechanical state.
/// Rust must not implement them as simulator work; they are listed here so
/// source audits do not reintroduce UI carriers under mechanical names.
pub const RENDER_ONLY_UI_SOURCE_CLASSES: &[&str] = &["BattleStartEffect"];

/// Java VFX/UI classes whose constructors or updates mutate combat/run state.
/// Rust must extract their mechanical transition and must not implement their
/// rendering, timing, hitbox, sound, or coordinate behavior.
pub const MECHANICAL_HOSTED_IN_UI_SOURCE_CLASSES: &[&str] = &[
    "CampfireDigEffect",
    "CampfireLiftEffect",
    "CampfireRecallEffect",
    "CampfireSleepEffect",
    "CampfireSmithEffect",
    "CampfireTokeEffect",
    "FastCardObtainEffect",
    "NecronomicurseEffect",
    "ObtainKeyEffect",
    "ObtainPotionEffect",
    "PlayerTurnEffect",
    "ShowCardAndAddToDiscardEffect",
    "ShowCardAndAddToDrawPileEffect",
    "ShowCardAndAddToHandEffect",
    "ShowCardAndObtainEffect",
];

/// Java screen classes that host mechanical decision state. Rust must extract
/// their candidate lists, selected refs, constraints, and result refs without
/// inheriting Java UI widgets, hover state, scrolling, or layout behavior.
pub const SCREEN_HOSTED_DECISION_SOURCE_CLASSES: &[&str] = &[
    "CardRewardScreen",
    "GridCardSelectScreen",
    "HandCardSelectScreen",
];
