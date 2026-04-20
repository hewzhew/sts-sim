use crate::content::potions::Potion;
use crate::runtime::combat::{CombatCard, CombatState};
use crate::state::core::ClientInput;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SearchEquivalenceMode {
    Off,
    Safe,
    Experimental,
}

impl SearchEquivalenceMode {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Off => "off",
            Self::Safe => "safe",
            Self::Experimental => "experimental",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SearchEquivalenceKind {
    Exact,
    Heuristic,
}

impl SearchEquivalenceKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Exact => "exact",
            Self::Heuristic => "heuristic",
        }
    }
}

pub(crate) fn default_equivalence_mode() -> SearchEquivalenceMode {
    match std::env::var("STS_SEARCH_EQUIVALENCE_MODE")
        .unwrap_or_else(|_| "safe".to_string())
        .to_ascii_lowercase()
        .as_str()
    {
        "off" => SearchEquivalenceMode::Off,
        "experimental" | "exp" => SearchEquivalenceMode::Experimental,
        _ => SearchEquivalenceMode::Safe,
    }
}

#[derive(Clone, Debug)]
pub(crate) struct InputCluster {
    pub(crate) representative: ClientInput,
    pub(crate) collapsed_inputs: Vec<ClientInput>,
    pub(crate) kind: Option<SearchEquivalenceKind>,
}

pub(crate) fn reduce_equivalent_inputs(
    combat: &CombatState,
    inputs: Vec<ClientInput>,
    mode: SearchEquivalenceMode,
) -> Vec<InputCluster> {
    if matches!(mode, SearchEquivalenceMode::Off) {
        return inputs
            .into_iter()
            .map(|input| InputCluster {
                representative: input,
                collapsed_inputs: Vec::new(),
                kind: None,
            })
            .collect();
    }

    let mut clusters: Vec<InputCluster> = Vec::new();
    'outer: for input in inputs {
        for cluster in &mut clusters {
            if same_or_equivalent_client_input(combat, &cluster.representative, &input) {
                cluster.collapsed_inputs.push(input);
                cluster.kind = Some(SearchEquivalenceKind::Exact);
                continue 'outer;
            }
        }
        clusters.push(InputCluster {
            representative: input,
            collapsed_inputs: Vec::new(),
            kind: None,
        });
    }
    clusters
}

pub(crate) fn same_or_equivalent_client_input(
    combat: &CombatState,
    left: &ClientInput,
    right: &ClientInput,
) -> bool {
    match (left, right) {
        (
            ClientInput::PlayCard {
                card_index: left_card,
                target: left_target,
            },
            ClientInput::PlayCard {
                card_index: right_card,
                target: right_target,
            },
        ) => {
            if left_target != right_target {
                return false;
            }
            match (
                combat.zones.hand.get(*left_card),
                combat.zones.hand.get(*right_card),
            ) {
                (Some(left_card), Some(right_card)) => {
                    left_card == right_card || same_card_play_signature(left_card, right_card)
                }
                _ => left_card == right_card,
            }
        }
        (
            ClientInput::UsePotion {
                potion_index: left_potion,
                target: left_target,
            },
            ClientInput::UsePotion {
                potion_index: right_potion,
                target: right_target,
            },
        ) => {
            if left_target != right_target {
                return false;
            }
            match (
                combat.entities.potions.get(*left_potion),
                combat.entities.potions.get(*right_potion),
            ) {
                (Some(Some(left_potion)), Some(Some(right_potion))) => {
                    left_potion == right_potion
                        || same_potion_use_signature(left_potion, right_potion)
                }
                _ => left_potion == right_potion,
            }
        }
        (ClientInput::EndTurn, ClientInput::EndTurn)
        | (ClientInput::Proceed, ClientInput::Proceed)
        | (ClientInput::Cancel, ClientInput::Cancel) => true,
        _ => left == right,
    }
}

fn same_card_play_signature(left: &CombatCard, right: &CombatCard) -> bool {
    left.id == right.id
        && left.upgrades == right.upgrades
        && left.misc_value == right.misc_value
        && left.base_damage_override == right.base_damage_override
        && left.cost_modifier == right.cost_modifier
        && left.cost_for_turn == right.cost_for_turn
        && left.base_damage_mut == right.base_damage_mut
        && left.base_block_mut == right.base_block_mut
        && left.base_magic_num_mut == right.base_magic_num_mut
        && left.multi_damage == right.multi_damage
        && left.exhaust_override == right.exhaust_override
        && left.retain_override == right.retain_override
        && left.free_to_play_once == right.free_to_play_once
        && left.energy_on_use == right.energy_on_use
}

fn same_potion_use_signature(left: &Potion, right: &Potion) -> bool {
    left.id == right.id
}

#[cfg(test)]
mod tests {
    use super::reduce_equivalent_inputs;
    use super::SearchEquivalenceMode;
    use crate::content::cards::CardId;
    use crate::runtime::combat::CombatCard;
    use crate::state::core::ClientInput;
    use crate::test_support::blank_test_combat;

    #[test]
    fn reduce_equivalent_inputs_collapses_identical_card_plays() {
        let mut combat = blank_test_combat();
        combat.zones.hand.push(CombatCard::new(CardId::Defend, 1));
        combat.zones.hand.push(CombatCard::new(CardId::Defend, 2));

        let clusters = reduce_equivalent_inputs(
            &combat,
            vec![
                ClientInput::PlayCard {
                    card_index: 0,
                    target: None,
                },
                ClientInput::PlayCard {
                    card_index: 1,
                    target: None,
                },
            ],
            SearchEquivalenceMode::Safe,
        );

        assert_eq!(clusters.len(), 1);
        assert_eq!(clusters[0].collapsed_inputs.len(), 1);
    }
}
