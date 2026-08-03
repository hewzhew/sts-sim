use crate::ai::analysis::card_semantics::{
    card_definition_with_upgrades, CombatEvent, Mechanic, TriggeredEffect,
};
use crate::ai::card_analysis_v1::card_analysis_profile_v1;
use crate::ai::card_semantics_v1::{
    card_mechanics_profile_v1, card_reward_facts_v1, CardRewardPickDependencyV1,
};
use crate::content::cards::{CardId, CardType};
use crate::runtime::combat::CombatCard;
use crate::state::rewards::RewardCard;
use crate::state::run::RunState;

use super::types::{
    StrategyCapabilityCoverageV1 as Coverage, StrategyCapabilityEvidenceV1 as Capability,
    StrategyCapabilityInputKindV1 as InputKind, StrategyCapabilityInputV1 as Input,
    StrategyCapabilityKindV1 as Kind, StrategyDeckFactsV1, StrategyThreatCoverageGapV1,
    StrategyThreatCoverageLedgerV1, StrategyThreatProfileV1, StrategyThreatTagV1 as Tag,
};

#[derive(Default)]
struct CapabilityFacts {
    single_target_damage: i32,
    best_single_hit: i32,
    aoe_sources: usize,
    block_total: i32,
    weak_sources: usize,
    strength_down_sources: usize,
    scaling_sources: usize,
    scaling_payoffs: usize,
    draw_sources: usize,
    energy_sources: usize,
    exhaust_generators: usize,
    broad_exhaust_generators: usize,
    exhaust_payoffs: usize,
    exhaust_block_payoffs: usize,
    debuff_sources: usize,
    dense_card_sources: usize,
    retaliation_safe_sources: usize,
}

pub fn threat_coverage_from_run_state_v1(
    run_state: &RunState,
    threats: &StrategyThreatProfileV1,
) -> StrategyThreatCoverageLedgerV1 {
    ledger_from_capability_facts(capability_facts_from_run_state(run_state), threats)
}

fn capability_facts_from_run_state(run_state: &RunState) -> CapabilityFacts {
    let mut facts = CapabilityFacts::default();
    for card in &run_state.master_deck {
        let card = RewardCard::new(card.id, card.upgrades);
        let observed = card_reward_facts_v1(&card);
        if observed.card_type == CardType::Attack {
            facts.single_target_damage = facts
                .single_target_damage
                .saturating_add(observed.damage.total_damage);
            facts.best_single_hit = facts.best_single_hit.max(observed.damage.damage_per_hit);
        }
        facts.aoe_sources += usize::from(observed.is_aoe && observed.damage.total_damage > 0);
        facts.block_total = facts.block_total.saturating_add(observed.block);
        facts.weak_sources += usize::from(observed.weak > 0);
        facts.strength_down_sources += usize::from(observed.enemy_strength_down > 0);
        facts.scaling_sources += usize::from(
            observed.strength_gain > 0
                && !card_mechanics_profile_v1(observed.card).temporary_strength_burst,
        );
        facts.scaling_payoffs += usize::from(
            observed
                .pick_dependencies
                .contains(&CardRewardPickDependencyV1::StrengthScaling),
        );
        let mechanics = card_mechanics_profile_v1(observed.card);
        facts.draw_sources +=
            usize::from(observed.draw_cards > i32::from(mechanics.hand_topdeck_selection));
        facts.energy_sources += usize::from(observed.energy_gain > 0);
        facts.exhaust_generators += usize::from(observed.exhausts_other_cards);
        facts.broad_exhaust_generators += usize::from(
            card_analysis_profile_v1(observed.card, card.upgrades)
                .is_block_plan_broad_exhaust_source,
        );
        facts.exhaust_payoffs += usize::from(
            observed
                .pick_dependencies
                .contains(&CardRewardPickDependencyV1::ExhaustPackage),
        );
        facts.exhaust_block_payoffs += usize::from(
            card_definition_with_upgrades(observed.card, card.upgrades)
                .event_handlers
                .iter()
                .any(|handler| {
                    handler.on == CombatEvent::CardExhausted
                        && handler.effect == TriggeredEffect::Provide(Mechanic::Block)
                }),
        );
        facts.debuff_sources += usize::from(observed.weak > 0)
            + usize::from(observed.vulnerable > 0)
            + usize::from(observed.enemy_strength_down > 0);
        facts.dense_card_sources += usize::from(
            observed.cost >= 2
                && (observed.damage.total_damage >= 15
                    || observed.block >= 12
                    || observed.enemy_strength_down > 0),
        );
        facts.retaliation_safe_sources += usize::from(
            (observed.card_type == CardType::Attack
                && !observed.is_aoe
                && observed.damage.hit_count == 1
                && observed.damage.total_damage >= 12)
                || matches!(
                    observed.card,
                    crate::content::cards::CardId::FlameBarrier
                        | crate::content::cards::CardId::Combust
                        | crate::content::cards::CardId::Juggernaut
                        | crate::content::cards::CardId::FireBreathing
                ),
        );
    }
    facts
}

pub fn threat_coverage_from_deck_facts_v1(
    deck: &StrategyDeckFactsV1,
    threats: &StrategyThreatProfileV1,
) -> StrategyThreatCoverageLedgerV1 {
    let facts = CapabilityFacts {
        single_target_damage: deck.total_attack_damage,
        block_total: deck.total_block,
        weak_sources: deck.weak_sources as usize,
        scaling_sources: deck.strength_sources as usize,
        scaling_payoffs: deck.strength_payoffs as usize,
        draw_sources: deck.draw_sources as usize,
        energy_sources: deck.energy_sources as usize,
        exhaust_generators: deck.exhaust_generators as usize,
        exhaust_payoffs: deck.exhaust_payoffs as usize,
        debuff_sources: (deck.weak_sources + deck.vulnerable_sources) as usize,
        ..CapabilityFacts::default()
    };
    ledger_from_capability_facts(facts, threats)
}

pub fn threat_coverage_after_card_v1(
    run_state: &RunState,
    threats: &StrategyThreatProfileV1,
    card: CardId,
    upgrades: u8,
) -> StrategyThreatCoverageLedgerV1 {
    let mut trial = run_state.clone();
    let uuid = trial
        .master_deck
        .iter()
        .map(|card| card.uuid)
        .max()
        .unwrap_or(0)
        .saturating_add(1);
    let mut candidate = CombatCard::new(card, uuid);
    candidate.upgrades = upgrades;
    trial.master_deck.push(candidate);
    threat_coverage_from_run_state_v1(&trial, threats)
}

pub fn threat_relevant_capability_improvements_v1(
    threats: &StrategyThreatProfileV1,
    before: &StrategyThreatCoverageLedgerV1,
    after: &StrategyThreatCoverageLedgerV1,
) -> Vec<Kind> {
    let mut improvements = Vec::new();
    for source in &threats.sources {
        for kind in required_capabilities(source.tag) {
            let before_coverage = before
                .capability(kind)
                .map(|capability| capability.coverage)
                .unwrap_or(Coverage::Unknown);
            let after_coverage = after
                .capability(kind)
                .map(|capability| capability.coverage)
                .unwrap_or(Coverage::Unknown);
            if after_coverage > before_coverage && !improvements.contains(&kind) {
                improvements.push(kind);
            }
        }
    }
    improvements
}

fn ledger_from_capability_facts(
    facts: CapabilityFacts,
    threats: &StrategyThreatProfileV1,
) -> StrategyThreatCoverageLedgerV1 {
    let mut ledger = StrategyThreatCoverageLedgerV1 {
        capabilities: vec![
            capability(
                Kind::SingleTargetFrontload,
                frontload_coverage(&facts),
                &facts,
                vec![format!(
                    "single_target_damage={} best_single_hit={}",
                    facts.single_target_damage, facts.best_single_hit
                )],
            ),
            capability(
                Kind::MultiTargetControl,
                count_coverage(facts.aoe_sources),
                &facts,
                vec![format!("aoe_sources={}", facts.aoe_sources)],
            ),
            capability(
                Kind::SustainedDefense,
                defense_coverage(&facts),
                &facts,
                vec![format!(
                    "block_total={} weak_sources={} strength_down_sources={} broad_exhaust_generators={} exhaust_block_payoffs={}",
                    facts.block_total,
                    facts.weak_sources,
                    facts.strength_down_sources,
                    facts.broad_exhaust_generators,
                    facts.exhaust_block_payoffs
                )],
            ),
            capability(
                Kind::LongFightScaling,
                long_fight_coverage(&facts),
                &facts,
                vec![format!(
                    "scaling_sources={} scaling_payoffs={} exhaust_generators={} exhaust_payoffs={}",
                    facts.scaling_sources,
                    facts.scaling_payoffs,
                    facts.exhaust_generators,
                    facts.exhaust_payoffs
                )],
            ),
            capability(
                Kind::DrawEnergyConsistency,
                consistency_coverage(&facts),
                &facts,
                vec![format!(
                    "draw_sources={} energy_sources={} exhaust_generators={}",
                    facts.draw_sources, facts.energy_sources, facts.exhaust_generators
                )],
            ),
            capability(
                Kind::PhaseControl,
                phase_coverage(&facts),
                &facts,
                vec![format!(
                    "best_single_hit={} draw_sources={} scaling_sources={}",
                    facts.best_single_hit, facts.draw_sources, facts.scaling_sources
                )],
            ),
            capability(
                Kind::DebuffResilience,
                artifact_strip_coverage(facts.debuff_sources),
                &facts,
                vec![format!(
                    "independent_debuff_applications={}",
                    facts.debuff_sources
                )],
            ),
            capability(
                Kind::CardPlayEfficiency,
                count_coverage(facts.dense_card_sources),
                &facts,
                vec![format!("dense_card_sources={}", facts.dense_card_sources)],
            ),
            capability(
                Kind::RetaliationSafeDamage,
                count_coverage(facts.retaliation_safe_sources),
                &facts,
                vec![format!(
                    "retaliation_safe_damage_sources={}",
                    facts.retaliation_safe_sources
                )],
            ),
            capability(
                Kind::TimedDamageRace,
                timed_race_coverage(&facts),
                &facts,
                vec![format!(
                    "single_target_damage={} draw_sources={} energy_sources={}",
                    facts.single_target_damage, facts.draw_sources, facts.energy_sources
                )],
            ),
        ],
        gaps: Vec::new(),
    };

    for source in &threats.sources {
        let required_capabilities = required_capabilities(source.tag);
        if required_capabilities.is_empty() {
            continue;
        }
        let uncovered = required_capabilities.iter().any(|kind| {
            ledger
                .capability(*kind)
                .map(|capability| capability.coverage.is_gap())
                .unwrap_or(false)
        });
        if uncovered {
            ledger.gaps.push(StrategyThreatCoverageGapV1 {
                tag: source.tag,
                source: source.source,
                subject: source.subject.clone(),
                required_capabilities,
                evidence: vec![source.evidence.clone()],
            });
        }
    }
    ledger.gaps.sort_by(|left, right| {
        (left.source, left.subject.as_str(), left.tag).cmp(&(
            right.source,
            right.subject.as_str(),
            right.tag,
        ))
    });
    ledger.gaps.dedup_by(|left, right| {
        left.source == right.source && left.subject == right.subject && left.tag == right.tag
    });
    ledger
}

fn capability(
    kind: Kind,
    coverage: Coverage,
    facts: &CapabilityFacts,
    evidence: Vec<String>,
) -> Capability {
    Capability {
        capability: kind,
        coverage,
        inputs: capability_inputs(kind, facts),
        evidence,
    }
}

fn capability_inputs(kind: Kind, facts: &CapabilityFacts) -> Vec<Input> {
    let count = |input, value: usize| Input {
        input,
        value: i32::try_from(value).unwrap_or(i32::MAX),
    };
    let scalar = |input, value| Input { input, value };

    match kind {
        Kind::SingleTargetFrontload => vec![
            scalar(InputKind::SingleTargetDamage, facts.single_target_damage),
            scalar(InputKind::BestSingleHit, facts.best_single_hit),
        ],
        Kind::MultiTargetControl => vec![count(InputKind::AoeSources, facts.aoe_sources)],
        Kind::SustainedDefense => vec![
            scalar(InputKind::BlockTotal, facts.block_total),
            count(InputKind::WeakSources, facts.weak_sources),
            count(InputKind::StrengthDownSources, facts.strength_down_sources),
            count(InputKind::ExhaustGenerators, facts.exhaust_generators),
            count(
                InputKind::BroadExhaustGenerators,
                facts.broad_exhaust_generators,
            ),
            count(InputKind::ExhaustBlockPayoffs, facts.exhaust_block_payoffs),
        ],
        Kind::LongFightScaling => vec![
            count(InputKind::ScalingSources, facts.scaling_sources),
            count(InputKind::ScalingPayoffs, facts.scaling_payoffs),
            count(InputKind::ExhaustGenerators, facts.exhaust_generators),
            count(InputKind::ExhaustPayoffs, facts.exhaust_payoffs),
        ],
        Kind::DrawEnergyConsistency => vec![
            count(InputKind::DrawSources, facts.draw_sources),
            count(InputKind::EnergySources, facts.energy_sources),
            count(InputKind::ExhaustGenerators, facts.exhaust_generators),
        ],
        Kind::PhaseControl => vec![
            scalar(InputKind::BestSingleHit, facts.best_single_hit),
            count(InputKind::DrawSources, facts.draw_sources),
            count(InputKind::ScalingSources, facts.scaling_sources),
        ],
        Kind::DebuffResilience => vec![count(InputKind::DebuffSources, facts.debuff_sources)],
        Kind::CardPlayEfficiency => {
            vec![count(InputKind::DenseCardSources, facts.dense_card_sources)]
        }
        Kind::RetaliationSafeDamage => vec![count(
            InputKind::RetaliationSafeSources,
            facts.retaliation_safe_sources,
        )],
        Kind::TimedDamageRace => vec![
            scalar(InputKind::SingleTargetDamage, facts.single_target_damage),
            scalar(InputKind::BestSingleHit, facts.best_single_hit),
            count(InputKind::DrawSources, facts.draw_sources),
            count(InputKind::EnergySources, facts.energy_sources),
            count(InputKind::ExhaustGenerators, facts.exhaust_generators),
        ],
    }
}

fn count_coverage(count: usize) -> Coverage {
    match count {
        0 => Coverage::Missing,
        1 => Coverage::Supported,
        _ => Coverage::Strong,
    }
}

fn frontload_coverage(facts: &CapabilityFacts) -> Coverage {
    if facts.best_single_hit >= 20 || facts.single_target_damage >= 80 {
        Coverage::Strong
    } else if facts.best_single_hit >= 12 || facts.single_target_damage >= 50 {
        Coverage::Supported
    } else if facts.single_target_damage > 0 {
        Coverage::Thin
    } else {
        Coverage::Missing
    }
}

fn defense_coverage(facts: &CapabilityFacts) -> Coverage {
    let supported_exhaust_block_engine =
        facts.exhaust_generators > 0 && facts.exhaust_block_payoffs > 0;
    let broad_exhaust_block_engine =
        facts.broad_exhaust_generators > 0 && facts.exhaust_block_payoffs > 0;
    if facts.strength_down_sources > 0 && facts.block_total >= 25
        || broad_exhaust_block_engine && facts.block_total >= 25
    {
        Coverage::Strong
    } else if facts.strength_down_sources > 0
        || facts.weak_sources > 0
        || facts.block_total >= 30
        || supported_exhaust_block_engine
    {
        Coverage::Supported
    } else if facts.block_total > 0 {
        Coverage::Thin
    } else {
        Coverage::Missing
    }
}

fn long_fight_coverage(facts: &CapabilityFacts) -> Coverage {
    let direct_scaling_supported = facts.scaling_sources > 0;
    let direct_scaling_engine = facts.scaling_sources > 0 && facts.scaling_payoffs > 0;
    let exhaust_engine = facts.exhaust_generators > 0 && facts.exhaust_payoffs > 0;
    if facts.scaling_sources >= 2
        || direct_scaling_engine
        || (exhaust_engine && facts.exhaust_generators + facts.exhaust_payoffs >= 3)
    {
        Coverage::Strong
    } else if direct_scaling_supported || exhaust_engine {
        Coverage::Supported
    } else if facts.scaling_payoffs > 0 || facts.exhaust_generators > 0 || facts.exhaust_payoffs > 0
    {
        Coverage::Thin
    } else {
        Coverage::Missing
    }
}

fn artifact_strip_coverage(independent_applications: usize) -> Coverage {
    match independent_applications {
        0 => Coverage::Missing,
        1 => Coverage::Thin,
        2 => Coverage::Supported,
        _ => Coverage::Strong,
    }
}

fn consistency_coverage(facts: &CapabilityFacts) -> Coverage {
    if facts.draw_sources >= 2 && (facts.energy_sources > 0 || facts.exhaust_generators > 0) {
        Coverage::Strong
    } else if facts.draw_sources > 0 {
        Coverage::Supported
    } else if facts.energy_sources > 0 || facts.exhaust_generators > 0 {
        Coverage::Thin
    } else {
        Coverage::Missing
    }
}

fn phase_coverage(facts: &CapabilityFacts) -> Coverage {
    if facts.best_single_hit >= 20 && facts.draw_sources > 0 && facts.scaling_sources > 0 {
        Coverage::Strong
    } else if facts.best_single_hit >= 15 && facts.draw_sources > 0 {
        Coverage::Supported
    } else if facts.best_single_hit >= 12 || facts.scaling_sources > 0 {
        Coverage::Thin
    } else {
        Coverage::Missing
    }
}

fn timed_race_coverage(facts: &CapabilityFacts) -> Coverage {
    match (frontload_coverage(facts), consistency_coverage(facts)) {
        (Coverage::Strong, Coverage::Supported | Coverage::Strong) => Coverage::Strong,
        (Coverage::Supported | Coverage::Strong, _) => Coverage::Supported,
        (Coverage::Thin, _) => Coverage::Thin,
        _ => Coverage::Missing,
    }
}

fn required_capabilities(tag: Tag) -> Vec<Kind> {
    match tag {
        Tag::HighIncomingDamage
        | Tag::MultiHit
        | Tag::StrengthDebuffValuable
        | Tag::WeakValuable => {
            vec![Kind::SustainedDefense]
        }
        Tag::AoEValuable => vec![Kind::MultiTargetControl],
        Tag::ArtifactBlocksDebuff => vec![Kind::DebuffResilience],
        Tag::StatusFlood => vec![Kind::DrawEnergyConsistency],
        Tag::SplitThreshold | Tag::ModeShiftThreshold => vec![Kind::PhaseControl],
        Tag::SkillPunish => vec![Kind::SingleTargetFrontload],
        Tag::PowerPunish | Tag::LongFightScaling | Tag::SetupWindow => {
            vec![Kind::LongFightScaling]
        }
        Tag::CardPlayLimit => vec![Kind::CardPlayEfficiency],
        Tag::RetaliationPunish => vec![Kind::RetaliationSafeDamage],
        Tag::TimedDamageRace => vec![Kind::TimedDamageRace],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn repeated_weak_supports_defense_without_claiming_persistent_strength_down() {
        let weak_only = CapabilityFacts {
            block_total: 48,
            weak_sources: 2,
            ..CapabilityFacts::default()
        };
        assert_eq!(defense_coverage(&weak_only), Coverage::Supported);

        let with_strength_down = CapabilityFacts {
            strength_down_sources: 1,
            ..weak_only
        };
        assert_eq!(defense_coverage(&with_strength_down), Coverage::Strong);
    }
}
