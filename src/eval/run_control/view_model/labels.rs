use crate::content::cards::{get_card_definition, CardId, CardType};
use crate::content::monsters::EnemyId;
use crate::runtime::combat::CombatCard;
use crate::state::core::PendingChoice;
use crate::state::map::node::RoomType;
use crate::state::rewards::RewardItem;
use crate::state::run::RunState;

use super::{CandidateAction, DecisionCandidate};

pub(crate) fn candidate(
    id: impl Into<String>,
    label: impl Into<String>,
    action: impl Into<CandidateAction>,
    note: Option<impl Into<String>>,
) -> DecisionCandidate {
    DecisionCandidate {
        id: id.into(),
        label: label.into(),
        action: action.into(),
        note: note.map(Into::into),
        resolution: None,
    }
}

pub(crate) fn unavailable_candidate(
    id: impl Into<String>,
    label: impl Into<String>,
    reason: impl Into<String>,
    note: Option<impl Into<String>>,
) -> DecisionCandidate {
    DecisionCandidate {
        id: id.into(),
        label: label.into(),
        action: CandidateAction::Unavailable {
            reason: reason.into(),
        },
        note: note.map(Into::into),
        resolution: None,
    }
}

pub(crate) fn reward_item_label(item: &RewardItem) -> String {
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

pub(crate) fn reward_card_label(id: CardId, upgrades: u8) -> String {
    let name = get_card_definition(id).name;
    if upgrades == 0 {
        name.to_string()
    } else {
        format!("{name}+{upgrades}")
    }
}

pub(crate) fn combat_card_label(card: &CombatCard) -> String {
    let name = get_card_definition(card.id).name;
    if card.upgrades == 0 {
        name.to_string()
    } else {
        format!("{name}+{}", card.upgrades)
    }
}

pub(crate) fn deck_summary(run_state: &RunState) -> String {
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

pub(crate) fn room_type_label(room_type: Option<RoomType>) -> &'static str {
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

pub(crate) fn boss_label(run_state: &RunState) -> String {
    run_state
        .boss_key
        .map(|boss| debug_words(&format!("{boss:?}")))
        .unwrap_or_else(|| "unknown".to_string())
}

pub(crate) fn clean_event_label(raw: &str) -> String {
    raw.trim()
        .strip_prefix('[')
        .and_then(|value| value.strip_suffix(']'))
        .unwrap_or_else(|| raw.trim())
        .to_string()
}

pub(crate) fn pending_choice_label(choice: &PendingChoice) -> &'static str {
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

pub(crate) fn shop_block_note(can_buy: bool, blocked_reason: Option<&str>) -> Option<String> {
    if can_buy {
        None
    } else {
        Some(format!(
            "locked: {}",
            blocked_reason.unwrap_or("cannot buy")
        ))
    }
}

pub(crate) fn monster_name(monster_type: usize) -> String {
    EnemyId::from_id(monster_type)
        .map(|enemy| enemy.get_name().to_string())
        .unwrap_or_else(|| format!("monster_type:{monster_type}"))
}

fn debug_words(raw: &str) -> String {
    let mut out = String::new();
    for (idx, ch) in raw.chars().enumerate() {
        if idx > 0 && ch.is_ascii_uppercase() {
            out.push(' ');
        }
        out.push(ch);
    }
    out
}
