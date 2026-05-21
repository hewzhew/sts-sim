use super::*;

#[derive(Clone, Debug, Default, PartialEq)]
pub enum PowerPayload {
    #[default]
    None,
    Card(CombatCard),
}

#[derive(Clone, Debug, PartialEq)]
pub struct Power {
    pub power_type: PowerId,
    pub instance_id: Option<u32>,
    pub amount: i32,
    pub extra_data: i32,
    pub payload: PowerPayload,
    pub just_applied: bool,
}
