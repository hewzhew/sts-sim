use crate::content::cards::{get_card_definition, CardId, CardType};
use crate::content::monsters::EnemyId;
use crate::runtime::combat::CombatCard;
use crate::state::core::PendingChoice;
use crate::state::events::EventEffect;
use crate::state::map::node::RoomType;
use crate::state::rewards::RewardItem;
use crate::state::run::RunState;

use super::DecisionCandidate;

pub(crate) fn candidate(
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

pub(crate) fn event_effect_summary(effects: &[EventEffect]) -> Option<String> {
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
