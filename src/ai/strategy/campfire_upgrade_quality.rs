use crate::ai::analysis::card_semantics::{card_definition, Mechanic, PlayEffect};
use crate::content::cards::{
    get_card_definition, upgrade_card_once_java, upgraded_base_cost_override,
};
use crate::content::cards::{CardId, CardTag};
use crate::runtime::combat::CombatCard;

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum CampfireUpgradeTier {
    Avoid,
    Low,
    Useful,
    Strong,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CampfireUpgradeHint {
    CostDown,
    ControlledExhaust,
    DamageDelta(i32),
    BlockDelta(i32),
    MagicDelta(i32),
    DebuffDuration,
    DrawOrEnergyAmount,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CampfireUpgradeCaution {
    StarterStrikeOrDefend,
    StarterUnique,
    ThinPayoffSupport,
    RedundantDebuffDuration,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CampfireUpgradeQuality {
    pub deck_index: usize,
    pub card: CardId,
    pub tier: CampfireUpgradeTier,
    pub hints: Vec<CampfireUpgradeHint>,
    pub cautions: Vec<CampfireUpgradeCaution>,
}

pub fn rank_campfire_upgrades(deck: &[CombatCard]) -> Vec<CampfireUpgradeQuality> {
    let mut targets: Vec<_> = deck
        .iter()
        .enumerate()
        .filter(|(_, card)| crate::state::core::master_deck_card_can_upgrade(card))
        .map(|(idx, card)| assess_upgrade(deck, idx, card))
        .collect();
    targets.sort_by(|a, b| {
        rank_key(b)
            .cmp(&rank_key(a))
            .then_with(|| a.deck_index.cmp(&b.deck_index))
    });
    targets
}

pub fn should_rest_before_smith(current_hp: i32, max_hp: i32) -> bool {
    max_hp > 0 && current_hp < max_hp && current_hp * 100 <= max_hp * 45
}

impl CampfireUpgradeQuality {
    pub fn hints_label(&self) -> String {
        list_or_dash(self.hints.iter().map(hint_label))
    }

    pub fn cautions_label(&self) -> String {
        list_or_dash(self.cautions.iter().map(caution_label))
    }

    pub fn compact_label(&self) -> String {
        let card = get_card_definition(self.card).name;
        let caution = self.cautions_label();
        let suffix = if caution == "-" {
            String::new()
        } else {
            format!(" risk:{caution}")
        };
        format!("{card}[{:?}:{}{}]", self.tier, self.hints_label(), suffix)
    }
}

fn assess_upgrade(
    deck: &[CombatCard],
    deck_index: usize,
    card: &CombatCard,
) -> CampfireUpgradeQuality {
    let def = get_card_definition(card.id);
    let mut upgraded = card.clone();
    upgrade_card_once_java(&mut upgraded);

    let before_cost = upgraded_base_cost_override(card).unwrap_or(def.cost);
    let after_cost = upgraded_base_cost_override(&upgraded).unwrap_or(def.cost);
    let damage_delta = def.upgrade_damage.max(0) * attack_hits(card.id).max(1);
    let block_delta = def.upgrade_block.max(0);
    let magic_delta = def.upgrade_magic.max(0);
    let semantic = card_definition(card.id);
    let mut hints = Vec::new();
    let mut cautions = Vec::new();

    if before_cost > after_cost {
        hints.push(CampfireUpgradeHint::CostDown);
    }
    if card.id == CardId::TrueGrit {
        hints.push(CampfireUpgradeHint::ControlledExhaust);
    }
    if damage_delta > 0 {
        hints.push(CampfireUpgradeHint::DamageDelta(damage_delta));
    }
    if block_delta > 0 {
        hints.push(CampfireUpgradeHint::BlockDelta(block_delta));
    }
    if magic_delta > 0 {
        hints.push(CampfireUpgradeHint::MagicDelta(magic_delta));
        if provides_any(&semantic, &[Mechanic::Weak, Mechanic::Vulnerable]) {
            hints.push(CampfireUpgradeHint::DebuffDuration);
        }
        if provides_any(&semantic, &[Mechanic::CardDraw, Mechanic::Energy]) {
            hints.push(CampfireUpgradeHint::DrawOrEnergyAmount);
        }
    }

    if def
        .tags
        .iter()
        .any(|tag| matches!(tag, CardTag::StarterStrike | CardTag::StarterDefend))
    {
        cautions.push(CampfireUpgradeCaution::StarterStrikeOrDefend);
    } else if matches!(card.id, CardId::Bash) {
        cautions.push(CampfireUpgradeCaution::StarterUnique);
    }
    if semantic
        .play_effects
        .contains(&PlayEffect::DamageUses(Mechanic::Block))
        && block_support_units(deck) < 2
    {
        cautions.push(CampfireUpgradeCaution::ThinPayoffSupport);
    }
    if provides_any(&semantic, &[Mechanic::Weak, Mechanic::Vulnerable])
        && matching_debuff_sources(deck, card.id) >= 2
    {
        cautions.push(CampfireUpgradeCaution::RedundantDebuffDuration);
    }

    let score = score_hints(&hints) - score_cautions(&cautions);
    let tier = if score >= 70 {
        CampfireUpgradeTier::Strong
    } else if score >= 28 {
        CampfireUpgradeTier::Useful
    } else if score > 0 {
        CampfireUpgradeTier::Low
    } else {
        CampfireUpgradeTier::Avoid
    };

    CampfireUpgradeQuality {
        deck_index,
        card: card.id,
        tier,
        hints,
        cautions,
    }
}

fn rank_key(target: &CampfireUpgradeQuality) -> (CampfireUpgradeTier, i32) {
    (
        target.tier,
        score_hints(&target.hints) - score_cautions(&target.cautions),
    )
}

fn score_hints(hints: &[CampfireUpgradeHint]) -> i32 {
    hints
        .iter()
        .map(|hint| match *hint {
            CampfireUpgradeHint::CostDown => 55,
            CampfireUpgradeHint::ControlledExhaust => 45,
            CampfireUpgradeHint::DamageDelta(delta) => delta * 3,
            CampfireUpgradeHint::BlockDelta(delta) => delta * 5,
            CampfireUpgradeHint::MagicDelta(delta) => delta * 8,
            CampfireUpgradeHint::DebuffDuration => 8,
            CampfireUpgradeHint::DrawOrEnergyAmount => 28,
        })
        .sum()
}

fn score_cautions(cautions: &[CampfireUpgradeCaution]) -> i32 {
    cautions
        .iter()
        .map(|caution| match caution {
            CampfireUpgradeCaution::StarterStrikeOrDefend => 70,
            CampfireUpgradeCaution::StarterUnique => 32,
            CampfireUpgradeCaution::ThinPayoffSupport => 60,
            CampfireUpgradeCaution::RedundantDebuffDuration => 35,
        })
        .sum()
}

fn provides_any(
    definition: &crate::ai::analysis::card_semantics::CardDefinition,
    mechanics: &[Mechanic],
) -> bool {
    definition.play_effects.iter().any(
        |effect| matches!(effect, PlayEffect::Provide(mechanic) if mechanics.contains(mechanic)),
    )
}

fn attack_hits(card: CardId) -> i32 {
    match card {
        CardId::TwinStrike => 2,
        CardId::SwordBoomerang => 3,
        CardId::RiddleWithHoles => 5,
        _ => 1,
    }
}

fn block_support_units(deck: &[CombatCard]) -> u8 {
    deck.iter()
        .map(|card| match card.id {
            CardId::FlameBarrier | CardId::Impervious | CardId::PowerThrough => 2,
            CardId::ShrugItOff | CardId::TrueGrit | CardId::SecondWind | CardId::IronWave => 1,
            _ => 0,
        })
        .sum()
}

fn matching_debuff_sources(deck: &[CombatCard], card: CardId) -> usize {
    let wants_vuln = card_definition(card)
        .play_effects
        .contains(&PlayEffect::Provide(Mechanic::Vulnerable));
    let wants_weak = card_definition(card)
        .play_effects
        .contains(&PlayEffect::Provide(Mechanic::Weak));
    deck.iter()
        .filter(|entry| {
            let definition = card_definition(entry.id);
            (wants_vuln
                && definition
                    .play_effects
                    .contains(&PlayEffect::Provide(Mechanic::Vulnerable)))
                || (wants_weak
                    && definition
                        .play_effects
                        .contains(&PlayEffect::Provide(Mechanic::Weak)))
        })
        .count()
}

fn hint_label(hint: &CampfireUpgradeHint) -> String {
    match *hint {
        CampfireUpgradeHint::CostDown => "cost_down".to_string(),
        CampfireUpgradeHint::ControlledExhaust => "controlled_exhaust".to_string(),
        CampfireUpgradeHint::DamageDelta(delta) => format!("damage+{delta}"),
        CampfireUpgradeHint::BlockDelta(delta) => format!("block+{delta}"),
        CampfireUpgradeHint::MagicDelta(delta) => format!("magic+{delta}"),
        CampfireUpgradeHint::DebuffDuration => "debuff_duration".to_string(),
        CampfireUpgradeHint::DrawOrEnergyAmount => "draw_or_energy_amount".to_string(),
    }
}

fn caution_label(caution: &CampfireUpgradeCaution) -> &'static str {
    match caution {
        CampfireUpgradeCaution::StarterStrikeOrDefend => "starter_strike_or_defend",
        CampfireUpgradeCaution::StarterUnique => "starter_unique",
        CampfireUpgradeCaution::ThinPayoffSupport => "thin_payoff_support",
        CampfireUpgradeCaution::RedundantDebuffDuration => "redundant_debuff_duration",
    }
}

fn list_or_dash<T: ToString>(items: impl Iterator<Item = T>) -> String {
    let labels = items.map(|item| item.to_string()).collect::<Vec<_>>();
    if labels.is_empty() {
        "-".to_string()
    } else {
        labels.join(",")
    }
}
