use crate::content::cards::{get_card_definition, CardId, CardType};
use crate::content::monsters::EnemyId;
use crate::runtime::combat::{CombatCard, CombatState};
use crate::state::core::{ClientInput, PendingChoice};
use crate::state::events::EventEffect;
use crate::state::map::node::RoomType;
use crate::state::rewards::RewardItem;
use crate::state::run::RunState;

use super::DecisionCandidate;

pub(super) fn candidate(
    id: impl Into<String>,
    label: impl Into<String>,
    command: impl Into<String>,
    note: Option<impl Into<String>>,
) -> DecisionCandidate {
    DecisionCandidate {
        id: id.into(),
        label: label.into(),
        command: command.into(),
        note: note.map(Into::into),
    }
}

pub(super) fn describe_combat_input(combat: &CombatState, input: &ClientInput) -> String {
    match input {
        ClientInput::PlayCard { card_index, target } => {
            let card = combat
                .zones
                .hand
                .get(*card_index)
                .map(combat_card_label)
                .unwrap_or_else(|| format!("hand[{card_index}]"));
            match target {
                Some(target) => {
                    format!(
                        "Play {card} h{card_index} -> {}",
                        monster_label(combat, *target)
                    )
                }
                None => format!("Play {card} h{card_index}"),
            }
        }
        ClientInput::UsePotion {
            potion_index,
            target,
        } => match target {
            Some(target) => format!(
                "Use potion p{potion_index} -> {}",
                monster_label(combat, *target)
            ),
            None => format!("Use potion p{potion_index}"),
        },
        ClientInput::DiscardPotion(slot) => format!("Discard potion p{slot}"),
        ClientInput::EndTurn => "End turn".to_string(),
        ClientInput::SubmitHandSelect(uuids) => format!("Submit hand selection {uuids:?}"),
        ClientInput::SubmitGridSelect(uuids) => format!("Submit grid selection {uuids:?}"),
        ClientInput::SubmitDiscoverChoice(idx) => format!("Choose generated card {idx}"),
        ClientInput::Cancel => "Cancel".to_string(),
        other => format!("{other:?}"),
    }
}

pub(super) fn event_effect_summary(effects: &[EventEffect]) -> Option<String> {
    if effects.is_empty() {
        return None;
    }
    let effects = effects
        .iter()
        .map(|effect| match effect {
            EventEffect::GainGold(amount) => format!("gain {amount} gold"),
            EventEffect::LoseGold(amount) => format!("lose {amount} gold"),
            EventEffect::LoseHp(amount) => format!("lose {amount} hp"),
            EventEffect::LoseMaxHp(amount) => format!("lose {amount} max hp"),
            EventEffect::Heal(amount) => format!("heal {amount}"),
            EventEffect::GainMaxHp(amount) => format!("gain {amount} max hp"),
            EventEffect::ObtainRelic { count, kind } => format!("obtain {count} {kind:?} relic"),
            EventEffect::ObtainPotion { count } => format!("obtain {count} potion"),
            EventEffect::ObtainCard { count, kind } => format!("obtain {count} {kind:?} card"),
            EventEffect::ObtainColorlessCard { count, kind } => {
                format!("obtain {count} {kind:?} colorless")
            }
            EventEffect::ObtainCurse { count, kind } => format!("obtain {count} {kind:?} curse"),
            EventEffect::RemoveCard { count, kind, .. } => format!("remove {count} {kind:?} card"),
            EventEffect::UpgradeCard { count } => format!("upgrade {count} card"),
            EventEffect::TransformCard { count } => format!("transform {count} card"),
            EventEffect::DuplicateCard { count } => format!("duplicate {count} card"),
            EventEffect::LoseRelic {
                specific: Some(relic),
                ..
            } => format!("lose relic {relic:?}"),
            EventEffect::LoseRelic { specific: None, .. } => "lose relic".to_string(),
            EventEffect::LoseStarterRelic {
                specific: Some(relic),
            } => format!("lose starter relic {relic:?}"),
            EventEffect::LoseStarterRelic { specific: None } => "lose starter relic".to_string(),
            EventEffect::StartCombat => "start combat".to_string(),
        })
        .collect::<Vec<_>>();
    Some(effects.join(", "))
}

pub(super) fn reward_item_label(item: &RewardItem) -> String {
    match item {
        RewardItem::Gold { amount } => format!("{amount} gold"),
        RewardItem::StolenGold { amount } => format!("{amount} stolen gold"),
        RewardItem::Card { cards } => {
            let labels = cards
                .iter()
                .map(|card| reward_card_label(card.id, card.upgrades))
                .collect::<Vec<_>>()
                .join(", ");
            format!("Card reward [{labels}]")
        }
        RewardItem::Relic { relic_id } => format!("Relic {relic_id:?}"),
        RewardItem::Potion { potion_id } => format!("Potion {potion_id:?}"),
        RewardItem::EmeraldKey => "Emerald key".to_string(),
        RewardItem::SapphireKey => "Sapphire key".to_string(),
    }
}

pub(super) fn reward_card_label(id: CardId, upgrades: u8) -> String {
    let name = get_card_definition(id).name;
    if upgrades == 0 {
        name.to_string()
    } else {
        format!("{name}+{upgrades}")
    }
}

pub(super) fn combat_card_label(card: &CombatCard) -> String {
    let name = get_card_definition(card.id).name;
    if card.upgrades == 0 {
        name.to_string()
    } else {
        format!("{name}+{}", card.upgrades)
    }
}

pub(super) fn deck_summary(run_state: &RunState) -> String {
    let mut attacks = 0usize;
    let mut skills = 0usize;
    let mut powers = 0usize;
    let mut statuses = 0usize;
    let mut curses = 0usize;
    let mut total_cost = 0i32;
    for card in &run_state.master_deck {
        let def = get_card_definition(card.id);
        match def.card_type {
            CardType::Attack => attacks += 1,
            CardType::Skill => skills += 1,
            CardType::Power => powers += 1,
            CardType::Status => statuses += 1,
            CardType::Curse => curses += 1,
        }
        total_cost += def.cost.max(0) as i32;
    }
    let avg_cost = if run_state.master_deck.is_empty() {
        0.0
    } else {
        total_cost as f32 / run_state.master_deck.len() as f32
    };
    format!(
        "Deck: cards={} | attacks={} | skills={} | powers={} | statuses={} | curses={} | avg_cost={avg_cost:.2}",
        run_state.master_deck.len(),
        attacks,
        skills,
        powers,
        statuses,
        curses
    )
}

pub(super) fn room_type_label(room_type: Option<RoomType>) -> &'static str {
    match room_type {
        Some(RoomType::EventRoom) => "Event",
        Some(RoomType::MonsterRoom) => "Monster",
        Some(RoomType::MonsterRoomElite) => "Elite",
        Some(RoomType::MonsterRoomBoss) => "Boss",
        Some(RoomType::RestRoom) => "Rest",
        Some(RoomType::ShopRoom) => "Shop",
        Some(RoomType::TreasureRoom) => "Treasure",
        Some(RoomType::TrueVictoryRoom) => "Victory",
        None => "Unknown",
    }
}

pub(super) fn boss_label(run_state: &RunState) -> String {
    run_state
        .boss_key
        .map(|boss| format!("{boss:?}"))
        .unwrap_or_else(|| "unknown".to_string())
}

pub(super) fn clean_event_label(raw: &str) -> String {
    raw.trim()
        .strip_prefix('[')
        .and_then(|value| value.strip_suffix(']'))
        .unwrap_or_else(|| raw.trim())
        .to_string()
}

pub(super) fn pending_choice_label(choice: &PendingChoice) -> &'static str {
    match choice {
        PendingChoice::GridSelect { .. } => "grid select",
        PendingChoice::HandSelect { .. } => "hand select",
        PendingChoice::DiscoverySelect(_) => "discovery",
        PendingChoice::ScrySelect { .. } => "scry",
        PendingChoice::CardRewardSelect { .. } => "card reward",
        PendingChoice::ForeignInfluenceSelect { .. } => "foreign influence",
        PendingChoice::ChooseOneSelect { .. } => "choose one",
        PendingChoice::StanceChoice => "stance",
    }
}

pub(super) fn shop_block_note(can_buy: bool, blocked_reason: Option<&str>) -> Option<String> {
    if can_buy {
        None
    } else {
        Some(format!(
            "locked: {}",
            blocked_reason.unwrap_or("cannot buy")
        ))
    }
}

pub(super) fn monster_name(monster_type: usize) -> String {
    EnemyId::from_id(monster_type)
        .map(|enemy| enemy.get_name().to_string())
        .unwrap_or_else(|| format!("monster_type:{monster_type}"))
}

fn monster_label(combat: &CombatState, target_id: usize) -> String {
    combat
        .entities
        .monsters
        .iter()
        .find(|monster| monster.id == target_id)
        .map(|monster| {
            format!(
                "{} slot {}",
                monster_name(monster.monster_type),
                monster.slot
            )
        })
        .unwrap_or_else(|| format!("entity {target_id}"))
}
