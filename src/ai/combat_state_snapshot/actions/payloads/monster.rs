use super::*;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReviveMonsterActionState {}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RollMoveActionState {
    pub monster_ref: MonsterRef,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SetMoveActionState {
    pub monster_ref: MonsterRef,
    pub next_move: i8,
    pub next_intent: IntentKind,
    pub next_damage: i32,
    pub next_name: Option<String>,
    pub multiplier: i32,
    pub is_multiplier: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpawnMonsterActionState {
    pub used: bool,
    pub monster_ref: MonsterRef,
    pub minion: bool,
    pub target_slot: i32,
    pub use_smart_positioning: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SuicideActionState {
    pub monster_ref: MonsterRef,
    pub relic_trigger: bool,
}
