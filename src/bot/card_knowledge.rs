use crate::bot::evaluator::DeckProfile;
use crate::bot::noncombat_families::ShopNeedProfile;
use crate::content::cards::CardId;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct NoncombatCardTags {
    pub high_value_tactical: bool,
    pub strength_capstone: bool,
    pub strength_enabler: bool,
    pub strength_payoff: bool,
    pub exhaust_core: bool,
    pub exhaust_payoff: bool,
    pub block_core: bool,
    pub block_payoff: bool,
    pub damage_gap_patch: bool,
    pub block_gap_patch: bool,
    pub control_gap_patch: bool,
    pub searing_anchor: bool,
    pub searing_support: bool,
}

pub(crate) fn noncombat_card_tags(card_id: CardId) -> NoncombatCardTags {
    use CardId::*;

    let high_value_tactical = matches!(
        card_id,
        Apotheosis
            | Panacea
            | Blind
            | DarkShackles
            | Trip
            | GoodInstincts
            | Finesse
            | FlashOfSteel
            | MasterOfStrategy
            | Corruption
            | FeelNoPain
            | DarkEmbrace
            | Shockwave
    );
    let strength_capstone = matches!(card_id, LimitBreak);
    let strength_enabler = matches!(card_id, Inflame | SpotWeakness | DemonForm);
    let strength_payoff = matches!(card_id, HeavyBlade | SwordBoomerang | Pummel | Whirlwind);
    let exhaust_core = matches!(card_id, Corruption | FeelNoPain | DarkEmbrace);
    let exhaust_payoff =
        matches!(card_id, SecondWind | BurningPact | SeverSoul | FiendFire);
    let block_core = matches!(card_id, Barricade | Entrench);
    let block_payoff = matches!(card_id, BodySlam | FlameBarrier | Impervious);
    let damage_gap_patch =
        matches!(card_id, Hemokinesis | Carnage | Pummel | Whirlwind | SearingBlow | Immolate | Uppercut);
    let block_gap_patch =
        matches!(card_id, ShrugItOff | FlameBarrier | GhostlyArmor | Impervious | PowerThrough | Disarm);
    let control_gap_patch = matches!(card_id, Disarm | Shockwave | Uppercut | Clothesline);
    let searing_anchor = matches!(card_id, SearingBlow);
    let searing_support = matches!(
        card_id,
        Armaments | Offering | BattleTrance | Headbutt | SeeingRed | ShrugItOff | DoubleTap
    );

    NoncombatCardTags {
        high_value_tactical,
        strength_capstone,
        strength_enabler,
        strength_payoff,
        exhaust_core,
        exhaust_payoff,
        block_core,
        block_payoff,
        damage_gap_patch,
        block_gap_patch,
        control_gap_patch,
        searing_anchor,
        searing_support,
    }
}

pub(crate) fn is_high_value_tactical_card(card_id: CardId) -> bool {
    noncombat_card_tags(card_id).high_value_tactical
}

pub(crate) fn reward_shell_bonus(card_id: CardId, profile: &DeckProfile) -> i32 {
    let tags = noncombat_card_tags(card_id);

    if tags.strength_capstone && profile.strength_enablers >= 1 {
        return 18;
    }
    if tags.strength_enabler && profile.strength_payoffs >= 2 {
        return 12;
    }
    if tags.strength_payoff && profile.strength_enablers >= 2 {
        return 8;
    }
    if tags.exhaust_core && (profile.exhaust_outlets >= 1 || profile.exhaust_fodder >= 1) {
        return 16;
    }
    if card_id == CardId::DarkEmbrace
        && (profile.exhaust_engines >= 1
            || (profile.exhaust_outlets >= 1 && profile.draw_sources >= 1))
    {
        return 14;
    }
    if tags.exhaust_payoff && profile.exhaust_engines >= 2 {
        return 10;
    }
    if card_id == CardId::BurningPact
        && (profile.exhaust_engines >= 1
            || (profile.exhaust_outlets >= 1 && profile.exhaust_fodder >= 1))
    {
        return 14;
    }
    if card_id == CardId::Offering && (profile.exhaust_engines >= 1 || profile.draw_sources >= 2)
    {
        return 10;
    }
    if card_id == CardId::Armaments
        && (profile.power_scalers >= 1
            || profile.block_core >= 2
            || (profile.exhaust_engines >= 1 && profile.draw_sources >= 1))
    {
        return 10;
    }
    if tags.block_core && profile.block_core >= 3 {
        return 16;
    }
    if tags.block_payoff && profile.block_payoffs >= 1 {
        return 10;
    }

    0
}

pub(crate) fn shop_shell_bonus(card_id: CardId, profile: &DeckProfile) -> i32 {
    let tags = noncombat_card_tags(card_id);

    if tags.strength_capstone && profile.strength_enablers >= 1 {
        return 18;
    }
    if tags.strength_enabler && profile.strength_payoffs >= 2 {
        return 12;
    }
    if tags.strength_payoff && profile.strength_enablers >= 2 {
        return 8;
    }
    if tags.exhaust_core && (profile.exhaust_outlets >= 1 || profile.exhaust_fodder >= 1) {
        return 18;
    }
    if tags.exhaust_payoff && profile.exhaust_engines >= 2 {
        return 10;
    }
    if tags.block_core && profile.block_core >= 3 {
        return 16;
    }
    if tags.block_payoff && profile.block_payoffs >= 1 {
        return 10;
    }

    0
}

pub(crate) fn shop_need_bonus(
    card_id: CardId,
    profile: &DeckProfile,
    shop_need: &ShopNeedProfile,
    searing_plan: i32,
) -> i32 {
    let tags = noncombat_card_tags(card_id);
    let mut bonus = 0;

    if shop_need.damage_gap > 0 && tags.damage_gap_patch {
        bonus += match card_id {
            CardId::Hemokinesis => 24 + shop_need.damage_gap / 3,
            CardId::Carnage => 20 + shop_need.damage_gap / 4,
            CardId::Pummel | CardId::Whirlwind => 18 + shop_need.damage_gap / 5,
            CardId::SearingBlow => 20 + shop_need.damage_gap / 4,
            CardId::Immolate => 22 + shop_need.damage_gap / 4,
            CardId::Uppercut => 8 + shop_need.damage_gap / 6,
            _ => 0,
        };
    }

    if shop_need.block_gap > 0 && tags.block_gap_patch {
        bonus += match card_id {
            CardId::ShrugItOff => 14 + shop_need.block_gap / 3,
            CardId::FlameBarrier => 16 + shop_need.block_gap / 3,
            CardId::GhostlyArmor => 12 + shop_need.block_gap / 4,
            CardId::Impervious => 20 + shop_need.block_gap / 3,
            CardId::PowerThrough => 10 + shop_need.block_gap / 4,
            CardId::Disarm => 8 + shop_need.block_gap / 6,
            _ => 0,
        };
    }

    if shop_need.control_gap > 0 && tags.control_gap_patch {
        bonus += match card_id {
            CardId::Disarm => 16 + shop_need.control_gap / 3,
            CardId::Shockwave => 16 + shop_need.control_gap / 3,
            CardId::Uppercut => 12 + shop_need.control_gap / 4,
            CardId::Clothesline => 8 + shop_need.control_gap / 5,
            _ => 0,
        };
    }

    if searing_plan > 0 && (tags.searing_anchor || tags.searing_support) {
        bonus += match card_id {
            CardId::SearingBlow => 40 + profile.searing_blow_upgrades * 10,
            CardId::Armaments => 18,
            CardId::Offering => 18,
            CardId::BattleTrance | CardId::Headbutt | CardId::SeeingRed => 12,
            CardId::ShrugItOff => 8,
            CardId::DoubleTap => 10,
            _ => 0,
        };
    }

    bonus
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tactical_and_shell_tags_are_explicit() {
        let tags = noncombat_card_tags(CardId::Corruption);
        assert!(tags.high_value_tactical);
        assert!(tags.exhaust_core);
        assert!(!tags.control_gap_patch);
    }

    #[test]
    fn shop_need_bonus_uses_gap_patch_and_searing_support_tags() {
        let profile = DeckProfile::default();
        let shop_need = ShopNeedProfile {
            damage_gap: 24,
            block_gap: 0,
            control_gap: 0,
            upgrade_hunger: 0,
            purge_hunger: 0,
            shell_incomplete: false,
        };

        assert!(shop_need_bonus(CardId::Hemokinesis, &profile, &shop_need, 0) > 0);
        assert_eq!(shop_need_bonus(CardId::DarkEmbrace, &profile, &shop_need, 0), 0);
    }
}
