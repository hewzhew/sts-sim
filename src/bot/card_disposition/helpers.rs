use crate::content::cards::{self, CardId, CardType};
use crate::runtime::combat::{CombatCard, CombatState, Intent};

pub(super) fn total_incoming_damage(combat: &CombatState) -> i32 {
    combat
        .entities
        .monsters
        .iter()
        .filter(|monster| !monster.is_dying && !monster.is_escaped && !monster.half_dead)
        .map(|monster| monster.intent_dmg * intent_hits(&monster.current_intent))
        .sum()
}

pub(super) fn intent_hits(intent: &Intent) -> i32 {
    match intent {
        Intent::Attack { hits, .. }
        | Intent::AttackBuff { hits, .. }
        | Intent::AttackDebuff { hits, .. }
        | Intent::AttackDefend { hits, .. } => (*hits as i32).max(1),
        _ => 0,
    }
}

pub(super) fn monster_is_attacking(monster: &crate::runtime::combat::MonsterEntity) -> bool {
    !monster.is_dying
        && !monster.is_escaped
        && !monster.half_dead
        && matches!(
            monster.current_intent,
            Intent::Attack { .. }
                | Intent::AttackBuff { .. }
                | Intent::AttackDebuff { .. }
                | Intent::AttackDefend { .. }
        )
}

pub(super) fn effective_block(
    card: &CombatCard,
    def: &crate::content::cards::CardDefinition,
    combat: &CombatState,
) -> i32 {
    let base = if card.base_block_mut > 0 {
        card.base_block_mut
    } else {
        def.base_block
    };
    (base + combat.get_power(0, crate::runtime::combat::PowerId::Dexterity)).max(0)
}

pub(super) fn effective_damage(
    card: &CombatCard,
    def: &crate::content::cards::CardDefinition,
) -> i32 {
    if card.base_damage_mut > 0 {
        card.base_damage_mut
    } else if let Some(override_damage) = card.base_damage_override {
        override_damage
    } else {
        def.base_damage
    }
}

pub(super) fn is_exhaust_outlet_card(card_id: CardId) -> bool {
    use CardId::*;
    matches!(
        card_id,
        SecondWind | BurningPact | FiendFire | SeverSoul | TrueGrit | Exhume
    )
}

pub(super) fn is_exhaust_engine_card(card_id: CardId) -> bool {
    use CardId::*;
    matches!(card_id, Corruption | FeelNoPain | DarkEmbrace | Evolve)
}

pub(super) fn is_block_core_card(card_id: CardId) -> bool {
    use CardId::*;
    matches!(
        card_id,
        Defend
            | DefendG
            | ShrugItOff
            | GhostlyArmor
            | FlameBarrier
            | Impervious
            | PowerThrough
            | Entrench
            | BodySlam
            | Barricade
    )
}

pub(super) fn is_draw_core_card(card_id: CardId) -> bool {
    use CardId::*;
    matches!(
        card_id,
        Offering
            | BattleTrance
            | BurningPact
            | ShrugItOff
            | PommelStrike
            | Warcry
            | DarkEmbrace
            | Evolve
    )
}

pub(super) fn is_setup_power_card(card_id: CardId) -> bool {
    use CardId::*;
    matches!(
        card_id,
        Corruption
            | FeelNoPain
            | DarkEmbrace
            | Barricade
            | DemonForm
            | Inflame
            | Metallicize
            | Evolve
            | Berserk
            | Rupture
    )
}

pub(super) fn is_keeper_priority_card(card_id: CardId) -> bool {
    use CardId::*;
    matches!(
        card_id,
        Bash | Offering
            | BattleTrance
            | Shockwave
            | Disarm
            | Apotheosis
            | Corruption
            | FeelNoPain
            | DarkEmbrace
            | Barricade
            | DemonForm
            | Impervious
    )
}

pub(super) fn is_status_or_curse_card(card_id: CardId) -> bool {
    matches!(
        cards::get_card_definition(card_id).card_type,
        CardType::Curse | CardType::Status
    )
}
