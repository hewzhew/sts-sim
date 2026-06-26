use crate::ai::analysis::card_semantics::{
    card_definition, CardDefinition, DeckMechanicContext, EventHandler, PayoffRequirement,
    PlayEffect, TriggeredEffect,
};
use crate::ai::strategy::package_state::{assess_package_state, PackageStateReport};
use crate::ai::strategy::package_transition::{
    assess_package_transition, PackageStateChange, PackageTransitionReport,
};
use crate::content::cards::CardId;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RewardSemanticProbeReport {
    pub deck_package: PackageStateReport,
    pub candidates: Vec<RewardCandidateSemanticProbe>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RewardCandidateSemanticProbe {
    pub card: CardId,
    pub transition: PackageTransitionReport,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RewardSemanticExplanationV1 {
    pub deck_package: PackageStateReport,
    pub candidates: Vec<RewardCandidateSemanticExplanationV1>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RewardCandidateSemanticExplanationV1 {
    pub card: CardId,
    pub package_changes: Vec<String>,
    pub closes: Vec<String>,
    pub opens: Vec<String>,
    pub provides: Vec<String>,
    pub damage_uses: Vec<String>,
    pub emits: Vec<String>,
    pub rules: Vec<String>,
    pub handlers: Vec<String>,
    pub burdens: Vec<String>,
    pub duplicates: Vec<String>,
    pub new_mechanics: Vec<String>,
    pub new_streams: Vec<String>,
    pub new_rules: Vec<String>,
}

pub fn assess_reward_semantics(
    deck: &[CardDefinition],
    candidates: &[CardDefinition],
) -> RewardSemanticProbeReport {
    let deck_context = DeckMechanicContext::from_definitions(deck);
    RewardSemanticProbeReport {
        deck_package: assess_package_state(&deck_context),
        candidates: candidates
            .iter()
            .cloned()
            .map(|candidate| RewardCandidateSemanticProbe {
                card: candidate.card,
                transition: assess_package_transition(deck, candidate),
            })
            .collect(),
    }
}

pub fn assess_reward_semantics_from_cards(
    deck: &[CardId],
    candidates: &[CardId],
) -> RewardSemanticProbeReport {
    let deck_definitions = deck
        .iter()
        .copied()
        .map(card_definition)
        .collect::<Vec<_>>();
    let candidate_definitions = candidates
        .iter()
        .copied()
        .map(card_definition)
        .collect::<Vec<_>>();
    assess_reward_semantics(&deck_definitions, &candidate_definitions)
}

pub fn explain_reward_semantics_v1(
    report: &RewardSemanticProbeReport,
) -> RewardSemanticExplanationV1 {
    RewardSemanticExplanationV1 {
        deck_package: report.deck_package.clone(),
        candidates: report
            .candidates
            .iter()
            .map(explain_reward_candidate_semantics_v1)
            .collect(),
    }
}

fn explain_reward_candidate_semantics_v1(
    probe: &RewardCandidateSemanticProbe,
) -> RewardCandidateSemanticExplanationV1 {
    let transition = &probe.transition;
    let play_effects = explain_play_effects(&transition.candidate_play_effects);
    RewardCandidateSemanticExplanationV1 {
        card: probe.card,
        package_changes: transition
            .package_changes
            .iter()
            .map(package_change_label)
            .collect(),
        closes: transition
            .newly_closed_requirements
            .iter()
            .map(payoff_requirement_label)
            .collect(),
        opens: transition
            .newly_open_requirements
            .iter()
            .map(payoff_requirement_label)
            .collect(),
        provides: play_effects.provides,
        damage_uses: play_effects.damage_uses,
        emits: play_effects.emits,
        rules: transition
            .candidate_installed_rules
            .iter()
            .map(debug_label)
            .collect(),
        handlers: transition
            .candidate_event_handlers
            .iter()
            .map(event_handler_label)
            .collect(),
        burdens: transition
            .candidate_burdens
            .iter()
            .map(debug_label)
            .collect(),
        duplicates: transition
            .candidate_duplicate_behaviors
            .iter()
            .map(debug_label)
            .collect(),
        new_mechanics: transition.new_mechanics.iter().map(debug_label).collect(),
        new_streams: transition
            .new_event_streams
            .iter()
            .map(debug_label)
            .collect(),
        new_rules: transition
            .new_installed_rules
            .iter()
            .map(debug_label)
            .collect(),
    }
}

#[derive(Default)]
struct PlayEffectExplanation {
    provides: Vec<String>,
    damage_uses: Vec<String>,
    emits: Vec<String>,
}

fn explain_play_effects(effects: &[PlayEffect]) -> PlayEffectExplanation {
    let mut explanation = PlayEffectExplanation::default();
    for effect in effects {
        match effect {
            PlayEffect::Provide(mechanic) => {
                explanation.provides.push(debug_label(mechanic));
            }
            PlayEffect::DamageUses(mechanic) => {
                explanation.damage_uses.push(debug_label(mechanic));
            }
            PlayEffect::EmitEvent(event) => {
                explanation.emits.push(debug_label(event));
            }
            PlayEffect::AddCombatDeckClutter => {
                explanation.emits.push("CombatDeckClutter".to_string());
            }
            PlayEffect::PlayTopCardAndExhaust => {
                explanation
                    .emits
                    .push("TopCardPlayedAndExhausted".to_string());
            }
        }
    }
    explanation
}

fn package_change_label(change: &PackageStateChange) -> String {
    format!("{:?}:{:?}->{:?}", change.package, change.from, change.to)
}

fn payoff_requirement_label(requirement: &PayoffRequirement) -> String {
    match requirement {
        PayoffRequirement::WantsMechanic(mechanic) => {
            format!("needs:{}", debug_label(mechanic))
        }
        PayoffRequirement::WantsEventStream(event) => {
            format!("needs_event:{}", debug_label(event))
        }
    }
}

fn event_handler_label(handler: &EventHandler) -> String {
    format!(
        "on:{}->{}",
        debug_label(&handler.on),
        triggered_effect_label(&handler.effect)
    )
}

fn triggered_effect_label(effect: &TriggeredEffect) -> String {
    match effect {
        TriggeredEffect::Provide(mechanic) => format!("Provide({})", debug_label(mechanic)),
        TriggeredEffect::LoseHpFromCard => "LoseHpFromCard".to_string(),
        TriggeredEffect::DealAllDamage => "DealAllDamage".to_string(),
    }
}

fn debug_label<T: std::fmt::Debug>(item: &T) -> String {
    format!("{item:?}")
}
