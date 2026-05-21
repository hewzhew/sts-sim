use super::*;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApplyPoisonOnRandomMonsterActionState {
    pub starting_duration_bits: F32Bits,
    pub power_to_apply: Option<PowerRef>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApplyPowerActionState {
    pub power_to_apply: PowerRef,
    pub starting_duration_bits: F32Bits,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApplyPowerToRandomEnemyActionState {
    pub power_to_apply: PowerRef,
    pub is_fast: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AttackDamageRandomEnemyActionState {
    pub card_ref: CardRef,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DamageActionState {
    pub gold_amount: i32,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DamageAllEnemiesActionState {
    pub damage: Vec<i32>,
    pub base_damage: i32,
    pub first_frame: bool,
    pub utilize_base_damage: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DamageRandomEnemyActionState {}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModifyBlockActionState {
    pub target_uuid: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PummelDamageActionState {}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReApplyPowersActionState {
    pub card_ref: CardRef,
    pub monster_ref: MonsterRef,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReducePowerActionState {
    pub power_id: Option<String>,
    pub power_ref: Option<PowerRef>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RemoveSpecificPowerActionState {
    pub power_id: Option<String>,
    pub power_ref: Option<PowerRef>,
}
