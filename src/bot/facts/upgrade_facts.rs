use crate::content::cards::{self, CardId};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct UpgradeFacts {
    pub changes_cost: bool,
    pub improves_frontload: bool,
    pub improves_block_efficiency: bool,
    pub improves_draw_consistency: bool,
    pub improves_target_control: bool,
    pub improves_scaling: bool,
    pub improves_exhaust_control: bool,
    pub extends_debuff_duration: bool,
    pub repeatable_upgrade: bool,
}

pub(crate) fn upgrade_facts(card_id: CardId) -> UpgradeFacts {
    use CardId::*;

    let def = cards::get_card_definition(card_id);
    UpgradeFacts {
        changes_cost: matches!(
            card_id,
            Armaments | BattleTrance | Berserk | BloodForBlood | LimitBreak | SeeingRed | Shockwave
        ),
        improves_frontload: def.upgrade_damage > 0
            || matches!(
                card_id,
                Bash | BattleTrance
                    | Carnage
                    | Offering
                    | PommelStrike
                    | Pummel
                    | SearingBlow
                    | Shockwave
                    | Uppercut
            ),
        improves_block_efficiency: def.upgrade_block > 0
            || matches!(
                card_id,
                BodySlam
                    | FlameBarrier
                    | GhostlyArmor
                    | Impervious
                    | ShrugItOff
                    | SecondWind
                    | TrueGrit
            ),
        improves_draw_consistency: matches!(
            card_id,
            BattleTrance | BurningPact | DarkEmbrace | Offering | PommelStrike | ShrugItOff
        ),
        improves_target_control: matches!(
            card_id,
            Bash | Disarm | Headbutt | Shockwave | SpotWeakness | TrueGrit | Uppercut
        ),
        improves_scaling: def.upgrade_magic > 0
            || matches!(
                card_id,
                Armaments
                    | Corruption
                    | DarkEmbrace
                    | DemonForm
                    | FeelNoPain
                    | Inflame
                    | LimitBreak
                    | Rupture
                    | SearingBlow
            ),
        improves_exhaust_control: matches!(
            card_id,
            BurningPact | Corruption | DarkEmbrace | FeelNoPain | SecondWind | TrueGrit
        ),
        extends_debuff_duration: matches!(card_id, Bash | Disarm | Shockwave | Uppercut),
        repeatable_upgrade: matches!(card_id, SearingBlow),
    }
}

pub(crate) fn dominant_upgrade_semantic_key(card_id: CardId) -> &'static str {
    let facts = upgrade_facts(card_id);
    if facts.repeatable_upgrade {
        "repeatable_upgrade"
    } else if facts.changes_cost {
        "upgrade_changes_cost"
    } else if facts.extends_debuff_duration || facts.improves_target_control {
        "upgrade_target_control"
    } else if facts.improves_exhaust_control {
        "upgrade_exhaust_control"
    } else if facts.improves_draw_consistency {
        "upgrade_draw_consistency"
    } else if facts.improves_scaling {
        "upgrade_scaling"
    } else if facts.improves_block_efficiency {
        "upgrade_block_efficiency"
    } else if facts.improves_frontload {
        "upgrade_frontload"
    } else {
        "upgrade_numeric_improvement"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn upgrade_facts_capture_semantic_changes() {
        let battle_trance = upgrade_facts(CardId::BattleTrance);
        assert!(battle_trance.changes_cost);
        assert!(battle_trance.improves_draw_consistency);

        let shockwave = upgrade_facts(CardId::Shockwave);
        assert!(shockwave.changes_cost);
        assert!(shockwave.extends_debuff_duration);
        assert!(shockwave.improves_target_control);

        let searing = upgrade_facts(CardId::SearingBlow);
        assert!(searing.repeatable_upgrade);
        assert!(searing.improves_scaling);
    }
}
