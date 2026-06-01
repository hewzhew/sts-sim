use super::*;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BurnIncreaseActionState {
    pub got_burned: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GainEnergyActionState {
    pub energy_gain: i32,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ObtainPotionActionState {
    pub potion_ref: PotionRef,
}
