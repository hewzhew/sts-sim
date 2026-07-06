use sts_simulator::content::cards::java_id;
use sts_simulator::eval::combat_case::CombatCase;
use sts_simulator::runtime::combat::CombatCard;

use super::super::key_card_counterfactual::{move_key_card, KeyCardCounterfactualPlacement};
use super::super::key_card_lifecycle::KeyCardReason;
use super::types::{RootActionRoleDuelBasis, RootActionRoleDuelKeyCard, RootActionRoleDuelVariant};

pub(super) enum PreparedRootActionRoleDuelVariant {
    Ready(PreparedRootActionRoleDuelCase),
    Skipped(RootActionRoleDuelVariant),
}

pub(super) struct PreparedRootActionRoleDuelCase {
    pub(super) case: CombatCase,
    pub(super) basis: RootActionRoleDuelBasis,
}

pub(super) fn prepare_duel_variant_case(
    original_case: &CombatCase,
    key_card: Option<(&CombatCard, KeyCardReason)>,
) -> PreparedRootActionRoleDuelVariant {
    let mut case = original_case.clone();
    let basis = match key_card {
        Some((card, reason)) => {
            let placement = KeyCardCounterfactualPlacement::OpeningHand;
            if move_key_card(&mut case.position.combat, card.uuid, placement).is_none() {
                return PreparedRootActionRoleDuelVariant::Skipped(skipped_variant(
                    card,
                    reason,
                    placement,
                    "key_card_not_in_active_combat_zones",
                ));
            }
            key_card_basis(card, reason, placement)
        }
        None => RootActionRoleDuelBasis {
            label: "original_root".to_string(),
            moved_key_card: None,
        },
    };
    PreparedRootActionRoleDuelVariant::Ready(PreparedRootActionRoleDuelCase { case, basis })
}

fn key_card_basis(
    card: &CombatCard,
    reason: KeyCardReason,
    placement: KeyCardCounterfactualPlacement,
) -> RootActionRoleDuelBasis {
    RootActionRoleDuelBasis {
        label: format!("key_card_opening_hand:{}#{}", java_id(card.id), card.uuid),
        moved_key_card: Some(RootActionRoleDuelKeyCard {
            card: format!("{}+{}", java_id(card.id), card.upgrades),
            uuid: card.uuid,
            reason: reason.label(),
            placement: placement.label(),
        }),
    }
}

fn skipped_variant(
    card: &CombatCard,
    reason: KeyCardReason,
    placement: KeyCardCounterfactualPlacement,
    skipped_reason: &'static str,
) -> RootActionRoleDuelVariant {
    RootActionRoleDuelVariant {
        basis: key_card_basis(card, reason, placement),
        skipped_reason: Some(skipped_reason),
        microscope: None,
        duels: Vec::new(),
    }
}
