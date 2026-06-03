use serde::{Deserialize, Serialize};

use crate::content::cards::java_id;
use crate::content::monsters::EnemyId;
use crate::content::relics::RelicId;
use crate::runtime::combat::{CombatCard, CombatState, Intent};

pub const OBSERVATION_BOUNDARY_SCHEMA_NAME: &str = "CombatPublicObservationV1";
pub const OBSERVATION_BOUNDARY_SCHEMA_VERSION: u32 = 1;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum InformationAccessV1 {
    Public,
    PrivilegedSimulator,
    DebugRaw,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ObservationEvidenceKindV1 {
    VisibleExact,
    PublicOrderedCollection,
    PublicUnorderedCollection,
    Hidden,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum HiddenInformationReasonV1 {
    RunicDome,
    IntentNotVisible,
    DrawPileOrderHidden,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CombatPublicObservationV1 {
    pub schema_name: String,
    pub schema_version: u32,
    pub information_access: InformationAccessV1,
    pub player: CombatPublicPlayerV1,
    pub hand: Vec<CombatPublicCardV1>,
    pub piles: CombatPublicPilesV1,
    pub potions: Vec<Option<CombatPublicPotionV1>>,
    pub monsters: Vec<CombatPublicMonsterV1>,
    pub hidden_reasons: Vec<HiddenInformationReasonV1>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CombatPublicPlayerV1 {
    pub player_class: String,
    pub ascension_level: u8,
    pub hp: i32,
    pub max_hp: i32,
    pub block: i32,
    pub energy: u8,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CombatPublicCardV1 {
    pub card_id: String,
    pub upgrades: u8,
    pub cost_for_turn: i32,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CombatPublicPilesV1 {
    pub draw: CombatPublicCardPileV1,
    pub discard_count: usize,
    pub exhaust_count: usize,
    pub limbo_count: usize,
    pub queued_cards_count: usize,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CombatPublicCardPileV1 {
    pub count: usize,
    pub evidence: ObservationEvidenceKindV1,
    pub hidden_reason: Option<HiddenInformationReasonV1>,
    pub cards: Vec<CombatPublicCardV1>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CombatPublicPotionV1 {
    pub potion_id: String,
    pub can_use: bool,
    pub can_discard: bool,
    pub requires_target: bool,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CombatPublicMonsterV1 {
    pub slot: u8,
    pub enemy_id: String,
    pub hp: i32,
    pub max_hp: i32,
    pub block: i32,
    pub alive: bool,
    pub escaped: bool,
    pub dying: bool,
    pub half_dead: bool,
    pub intent: CombatPublicIntentV1,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CombatPublicIntentV1 {
    pub evidence: ObservationEvidenceKindV1,
    pub intent: Option<String>,
    pub preview_damage_per_hit: Option<i32>,
    pub hidden_reason: Option<HiddenInformationReasonV1>,
}

pub fn combat_public_observation_v1(combat: &CombatState) -> CombatPublicObservationV1 {
    let runic_dome = combat.entities.player.has_relic(RelicId::RunicDome);
    let draw_order_visible = combat.entities.player.has_relic(RelicId::FrozenEye);
    let mut hidden_reasons = Vec::new();
    if runic_dome {
        hidden_reasons.push(HiddenInformationReasonV1::RunicDome);
    }
    if !draw_order_visible && !combat.zones.draw_pile.is_empty() {
        hidden_reasons.push(HiddenInformationReasonV1::DrawPileOrderHidden);
    }

    CombatPublicObservationV1 {
        schema_name: OBSERVATION_BOUNDARY_SCHEMA_NAME.to_string(),
        schema_version: OBSERVATION_BOUNDARY_SCHEMA_VERSION,
        information_access: InformationAccessV1::Public,
        player: CombatPublicPlayerV1 {
            player_class: combat.meta.player_class.clone(),
            ascension_level: combat.meta.ascension_level,
            hp: combat.entities.player.current_hp,
            max_hp: combat.entities.player.max_hp,
            block: combat.entities.player.block,
            energy: combat.turn.energy,
        },
        hand: combat.zones.hand.iter().map(public_card).collect(),
        piles: CombatPublicPilesV1 {
            draw: public_draw_pile(combat, draw_order_visible),
            discard_count: combat.zones.discard_pile.len(),
            exhaust_count: combat.zones.exhaust_pile.len(),
            limbo_count: combat.zones.limbo.len(),
            queued_cards_count: combat.zones.queued_cards.len(),
        },
        potions: combat
            .entities
            .potions
            .iter()
            .map(|slot| {
                slot.as_ref().map(|potion| CombatPublicPotionV1 {
                    potion_id: format!("{:?}", potion.id),
                    can_use: potion.can_use,
                    can_discard: potion.can_discard,
                    requires_target: potion.requires_target,
                })
            })
            .collect(),
        monsters: combat
            .entities
            .monsters
            .iter()
            .map(|monster| CombatPublicMonsterV1 {
                slot: monster.slot,
                enemy_id: EnemyId::from_id(monster.monster_type)
                    .map(|enemy| format!("{enemy:?}"))
                    .unwrap_or_else(|| format!("monster_type:{}", monster.monster_type)),
                hp: monster.current_hp,
                max_hp: monster.max_hp,
                block: monster.block,
                alive: monster.is_alive_for_action(),
                escaped: monster.is_escaped,
                dying: monster.is_dying,
                half_dead: monster.half_dead,
                intent: public_intent(combat, monster.id, runic_dome),
            })
            .collect(),
        hidden_reasons,
    }
}

fn public_draw_pile(combat: &CombatState, draw_order_visible: bool) -> CombatPublicCardPileV1 {
    let mut cards = combat
        .zones
        .draw_pile
        .iter()
        .map(public_card)
        .collect::<Vec<_>>();
    if !draw_order_visible {
        cards.sort_by(|a, b| {
            a.card_id
                .cmp(&b.card_id)
                .then(a.upgrades.cmp(&b.upgrades))
                .then(a.cost_for_turn.cmp(&b.cost_for_turn))
        });
    }
    CombatPublicCardPileV1 {
        count: combat.zones.draw_pile.len(),
        evidence: if draw_order_visible {
            ObservationEvidenceKindV1::PublicOrderedCollection
        } else {
            ObservationEvidenceKindV1::PublicUnorderedCollection
        },
        hidden_reason: (!draw_order_visible && !combat.zones.draw_pile.is_empty())
            .then_some(HiddenInformationReasonV1::DrawPileOrderHidden),
        cards,
    }
}

fn public_intent(
    combat: &CombatState,
    monster_id: usize,
    runic_dome: bool,
) -> CombatPublicIntentV1 {
    if runic_dome {
        return CombatPublicIntentV1 {
            evidence: ObservationEvidenceKindV1::Hidden,
            intent: None,
            preview_damage_per_hit: None,
            hidden_reason: Some(HiddenInformationReasonV1::RunicDome),
        };
    }
    let observation = combat
        .runtime
        .monster_protocol
        .get(&monster_id)
        .map(|protocol| &protocol.observation);
    let Some(observation) = observation else {
        return hidden_intent(HiddenInformationReasonV1::IntentNotVisible);
    };
    if observation.visible_intent == Intent::Unknown {
        return hidden_intent(HiddenInformationReasonV1::IntentNotVisible);
    }
    CombatPublicIntentV1 {
        evidence: ObservationEvidenceKindV1::VisibleExact,
        intent: Some(format!("{:?}", observation.visible_intent)),
        preview_damage_per_hit: (observation.preview_damage_per_hit > 0)
            .then_some(observation.preview_damage_per_hit),
        hidden_reason: None,
    }
}

fn hidden_intent(reason: HiddenInformationReasonV1) -> CombatPublicIntentV1 {
    CombatPublicIntentV1 {
        evidence: ObservationEvidenceKindV1::Hidden,
        intent: None,
        preview_damage_per_hit: None,
        hidden_reason: Some(reason),
    }
}

fn public_card(card: &CombatCard) -> CombatPublicCardV1 {
    CombatPublicCardV1 {
        card_id: java_id(card.id).to_string(),
        upgrades: card.upgrades,
        cost_for_turn: card.cost_for_turn_java(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::cards::CardId;
    use crate::content::monsters::EnemyId;
    use crate::content::relics::RelicState;
    use crate::runtime::combat::CombatCard;

    #[test]
    fn runic_dome_public_observation_hides_intent_and_damage_preview() {
        let mut combat = crate::test_support::blank_test_combat();
        let mut monster = crate::test_support::test_monster(EnemyId::JawWorm);
        monster.id = 7;
        monster.slot = 0;
        combat.entities.monsters.push(monster);
        combat.set_monster_protocol_visible_intent(
            7,
            Intent::Attack {
                damage: 11,
                hits: 1,
            },
        );
        combat
            .entities
            .player
            .add_relic(RelicState::new(RelicId::RunicDome));

        let observation = combat_public_observation_v1(&combat);

        assert!(observation
            .hidden_reasons
            .contains(&HiddenInformationReasonV1::RunicDome));
        assert_eq!(
            observation.monsters[0].intent.hidden_reason,
            Some(HiddenInformationReasonV1::RunicDome)
        );
        assert_eq!(observation.monsters[0].intent.intent, None);
        assert_eq!(observation.monsters[0].intent.preview_damage_per_hit, None);
    }

    #[test]
    fn draw_pile_order_is_hidden_without_frozen_eye_but_card_set_is_public() {
        let mut combat = crate::test_support::blank_test_combat();
        combat.zones.draw_pile = vec![
            CombatCard::new(CardId::Bash, 1),
            CombatCard::new(CardId::Strike, 2),
            CombatCard::new(CardId::Defend, 3),
        ];

        let observation = combat_public_observation_v1(&combat);

        assert_eq!(
            observation.piles.draw.evidence,
            ObservationEvidenceKindV1::PublicUnorderedCollection
        );
        assert_eq!(
            observation.piles.draw.hidden_reason,
            Some(HiddenInformationReasonV1::DrawPileOrderHidden)
        );
        assert_eq!(
            observation
                .piles
                .draw
                .cards
                .iter()
                .map(|card| card.card_id.as_str())
                .collect::<Vec<_>>(),
            vec!["Bash", "Defend_R", "Strike_R"]
        );
    }

    #[test]
    fn frozen_eye_public_observation_keeps_draw_pile_order() {
        let mut combat = crate::test_support::blank_test_combat();
        combat
            .entities
            .player
            .add_relic(RelicState::new(RelicId::FrozenEye));
        combat.zones.draw_pile = vec![
            CombatCard::new(CardId::Bash, 1),
            CombatCard::new(CardId::Strike, 2),
            CombatCard::new(CardId::Defend, 3),
        ];

        let observation = combat_public_observation_v1(&combat);

        assert_eq!(
            observation.piles.draw.evidence,
            ObservationEvidenceKindV1::PublicOrderedCollection
        );
        assert_eq!(observation.piles.draw.hidden_reason, None);
        assert_eq!(
            observation
                .piles
                .draw
                .cards
                .iter()
                .map(|card| card.card_id.as_str())
                .collect::<Vec<_>>(),
            vec!["Bash", "Strike_R", "Defend_R"]
        );
    }
}
