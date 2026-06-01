use super::*;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChooseOneColorlessActionState {
    pub retrieve_card: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConditionalDrawActionState {
    pub restricted_type: CardType,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CodexActionState {
    pub retrieve_card: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiscoveryActionState {
    pub retrieve_card: bool,
    pub return_colorless: bool,
    pub card_type: Option<CardType>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ForeignInfluenceActionState {
    pub retrieve_card: bool,
    pub upgraded: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScryActionState {
    pub starting_duration_bits: F32Bits,
}
