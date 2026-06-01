use super::*;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AddCardToDeckActionState {
    pub card_to_obtain: CardRef,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BetterDiscardPileToHandActionState {
    pub number_of_cards: i32,
    pub optional: bool,
    pub new_cost: i32,
    pub set_cost: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BetterDrawPileToHandActionState {
    pub number_of_cards: i32,
    pub optional: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiscardToHandActionState {
    pub card_ref: CardRef,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DrawCardActionState {
    pub shuffle_check: bool,
    pub clear_draw_history: bool,
    pub follow_up_action: Option<Box<ActionState>>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DrawPileToHandActionState {
    pub type_to_check: CardType,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiscardActionState {
    pub is_random: bool,
    pub end_turn: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiscardSpecificCardActionState {
    pub target_card: CardRef,
    pub group_zone_ref: Option<ZoneRef>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EmptyDeckShuffleActionState {
    pub shuffled: bool,
    pub vfx_done: bool,
    pub count: i32,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExhaustActionState {
    pub is_random: bool,
    pub any_number: bool,
    pub can_pick_zero: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExhaustToHandActionState {
    pub card_ref: CardRef,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExhaustSpecificCardActionState {
    pub target_card: CardRef,
    pub group_zone_ref: ZoneRef,
    pub starting_duration_bits: F32Bits,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MakeTempCardInDiscardActionState {
    pub card_to_make: CardRef,
    pub num_cards: i32,
    pub same_uuid: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MakeTempCardInDiscardAndDeckActionState {
    pub card_to_make: CardRef,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MakeTempCardInDrawPileActionState {
    pub card_to_make: CardRef,
    pub random_spot: bool,
    pub to_bottom: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MakeTempCardInHandActionState {
    pub card_to_make: CardRef,
    pub same_uuid: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct NewQueueCardActionState {
    pub card_ref: Option<CardRef>,
    pub random_target: bool,
    pub immediate_card: bool,
    pub autoplay_card: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlayTopCardActionState {
    pub exhaust_cards: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PutOnBottomOfDeckActionState {
    pub is_random: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PutOnDeckActionState {
    pub is_random: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct QueueCardActionState {
    pub card_ref: Option<CardRef>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReduceCostActionState {
    pub target_uuid: Option<String>,
    pub card_ref: Option<CardRef>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReduceCostForTurnActionState {
    pub target_card: CardRef,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResetFlagsActionState {
    pub card_ref: CardRef,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SetDontTriggerActionState {
    pub card_ref: CardRef,
    pub trigger: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ShowCardActionState {
    pub card_ref: CardRef,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ShowCardAndPoofActionState {
    pub card_ref: CardRef,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TransformCardInHandActionState {
    pub replacement_card: CardRef,
    pub hand_index: i32,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct UnlimboActionState {
    pub card_ref: CardRef,
    pub exhaust: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct UpdateCardDescriptionActionState {
    pub target_card: CardRef,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct UseCardActionState {
    pub target_card: CardRef,
    pub card_target: Option<CombatantRef>,
    pub exhaust_card: bool,
    pub return_to_hand: bool,
    pub rebound_card: bool,
}
