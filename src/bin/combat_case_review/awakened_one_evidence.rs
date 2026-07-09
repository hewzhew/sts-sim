use serde::Serialize;
use sts_simulator::content::cards::{self, CardId};
use sts_simulator::content::monsters::EnemyId;
use sts_simulator::eval::combat_case::CombatCase;
use sts_simulator::runtime::combat::{CombatCard, CombatState};

use super::counterfactual_hp::CounterfactualHpProbe;

#[derive(Serialize)]
pub(crate) struct AwakenedOneFailureEvidenceFrame {
    pub(super) schema: &'static str,
    pub(super) contract: &'static str,
    pub(super) boss: &'static str,
    pub(super) start: AwakenedOneStartEvidence,
    pub(super) claims: Vec<AwakenedOneEvidenceClaim>,
    pub(super) risk_tags: Vec<&'static str>,
    pub(super) conclusion: &'static str,
}

#[derive(Serialize)]
pub(super) struct AwakenedOneStartEvidence {
    pub(super) turn: u32,
    pub(super) player_hp: i32,
    pub(super) player_max_hp: i32,
    pub(super) deck_size: usize,
    pub(super) power_cards: Vec<String>,
    pub(super) cultists_alive: usize,
    pub(super) awakened_hp: Option<i32>,
    pub(super) awakened_max_hp: Option<i32>,
    pub(super) total_enemy_hp: i32,
}

#[derive(Serialize)]
pub(super) struct AwakenedOneEvidenceClaim {
    pub(super) claim: &'static str,
    pub(super) status: &'static str,
    pub(super) support: Vec<String>,
    pub(super) counterevidence: Vec<String>,
    pub(super) unknown: Vec<String>,
}

#[derive(Default)]
struct AwakenedOneDeckSignals {
    deck: Vec<CombatCard>,
    powers: Vec<CombatCard>,
    damage_scaling: Vec<CombatCard>,
    defensive_scaling_or_mitigation: Vec<CombatCard>,
    big_block: Vec<CombatCard>,
    aoe: Vec<CombatCard>,
    access: Vec<CombatCard>,
    curses: Vec<CombatCard>,
}

pub(super) fn awakened_one_failure_evidence(
    case: &CombatCase,
    hp_probe: Option<&CounterfactualHpProbe>,
) -> Option<AwakenedOneFailureEvidenceFrame> {
    if !is_awakened_one_case(&case.position.combat) {
        return None;
    }

    let signals = AwakenedOneDeckSignals::from_combat(&case.position.combat);
    let mut claims = vec![
        damage_scaling_claim(&signals),
        defensive_scaling_claim(&signals),
        cultist_deadline_claim(&signals),
        phase2_dark_echo_claim(&signals),
        power_penalty_claim(&signals),
        deck_clean_not_sufficient_claim(&signals),
    ];
    if let Some(probe) = hp_probe {
        claims.push(full_hp_probe_claim(probe));
    } else {
        claims.push(AwakenedOneEvidenceClaim {
            claim: "full_hp_counterfactual_probe",
            status: "unknown",
            support: vec![],
            counterevidence: vec![],
            unknown: vec!["counterfactual_hp_probe was not run".to_string()],
        });
    }

    let risk_tags = risk_tags(&claims);
    let conclusion = conclusion_from_risk_tags(&risk_tags);
    Some(AwakenedOneFailureEvidenceFrame {
        schema: "awakened_one_failure_evidence_frame_v0",
        contract: "review_only_boss_plan_claims_with_support_counterevidence_unknown_no_runner_policy_change",
        boss: "AwakenedOne",
        start: start_evidence(&case.position.combat, &signals),
        claims,
        conclusion,
        risk_tags,
    })
}

impl AwakenedOneDeckSignals {
    fn from_combat(combat: &CombatState) -> Self {
        let deck = if combat.meta.master_deck_snapshot.is_empty() {
            combat
                .zones
                .hand
                .iter()
                .chain(combat.zones.draw_pile.iter())
                .chain(combat.zones.discard_pile.iter())
                .chain(combat.zones.exhaust_pile.iter())
                .cloned()
                .collect()
        } else {
            combat.meta.master_deck_snapshot.clone()
        };
        let mut signals = Self {
            deck,
            ..Default::default()
        };
        for card in &signals.deck {
            if is_power(card.id) {
                signals.powers.push(card.clone());
            }
            if is_damage_scaling(card.id) {
                signals.damage_scaling.push(card.clone());
            }
            if is_defensive_scaling_or_mitigation(card.id) {
                signals.defensive_scaling_or_mitigation.push(card.clone());
            }
            if is_big_block(card.id) {
                signals.big_block.push(card.clone());
            }
            if is_aoe(card.id) {
                signals.aoe.push(card.clone());
            }
            if is_access(card.id) {
                signals.access.push(card.clone());
            }
            if is_curse(card.id) {
                signals.curses.push(card.clone());
            }
        }
        signals
    }
}

fn damage_scaling_claim(signals: &AwakenedOneDeckSignals) -> AwakenedOneEvidenceClaim {
    if signals.damage_scaling.is_empty() {
        AwakenedOneEvidenceClaim {
            claim: "damage_scaling_present",
            status: "unsupported",
            support: vec![],
            counterevidence: vec![
                "no Demon Form / Limit Break / strength scaling evidence in deck".to_string(),
            ],
            unknown: vec![],
        }
    } else {
        AwakenedOneEvidenceClaim {
            claim: "damage_scaling_present",
            status: if signals.damage_scaling.len() == 1
                && signals.damage_scaling[0].id == CardId::DemonForm
            {
                "single_slow_source"
            } else {
                "supported"
            },
            support: card_labels(&signals.damage_scaling),
            counterevidence: if signals.damage_scaling.len() == 1
                && signals.damage_scaling[0].id == CardId::DemonForm
            {
                vec!["Demon Form is slow and must be survived into value".to_string()]
            } else {
                vec![]
            },
            unknown: vec![],
        }
    }
}

fn defensive_scaling_claim(signals: &AwakenedOneDeckSignals) -> AwakenedOneEvidenceClaim {
    if signals.defensive_scaling_or_mitigation.is_empty() {
        AwakenedOneEvidenceClaim {
            claim: "defensive_scaling_or_mitigation_present",
            status: "unsupported",
            support: vec![],
            counterevidence: vec![
                "no Disarm / Shockwave / Impervious / Power Through / Feel No Pain / Second Wind / Barricade evidence".to_string(),
                format!(
                    "generic block cards do not establish boss-grade defensive scaling: {}",
                    card_labels(&filter_cards(signals, is_generic_block)).join(", ")
                ),
            ],
            unknown: vec![],
        }
    } else {
        AwakenedOneEvidenceClaim {
            claim: "defensive_scaling_or_mitigation_present",
            status: "supported",
            support: card_labels(&signals.defensive_scaling_or_mitigation),
            counterevidence: vec![],
            unknown: vec![],
        }
    }
}

fn cultist_deadline_claim(signals: &AwakenedOneDeckSignals) -> AwakenedOneEvidenceClaim {
    if signals.aoe.is_empty() {
        AwakenedOneEvidenceClaim {
            claim: "cultist_deadline_plan",
            status: "unsupported",
            support: vec![],
            counterevidence: vec!["no AOE evidence for early Cultist cleanup".to_string()],
            unknown: vec![
                "single-target sequencing may still kill Cultists but is not evidenced here"
                    .to_string(),
            ],
        }
    } else {
        AwakenedOneEvidenceClaim {
            claim: "cultist_deadline_plan",
            status: "weak_supported",
            support: card_labels(&signals.aoe),
            counterevidence: vec![
                "AOE presence does not prove Cultists are killed before scaling pressure"
                    .to_string(),
            ],
            unknown: vec!["actual Cultist death turns require line replay evidence".to_string()],
        }
    }
}

fn phase2_dark_echo_claim(signals: &AwakenedOneDeckSignals) -> AwakenedOneEvidenceClaim {
    if !signals.big_block.is_empty() || !signals.defensive_scaling_or_mitigation.is_empty() {
        AwakenedOneEvidenceClaim {
            claim: "phase2_dark_echo_plan",
            status: "supported",
            support: card_labels(
                &signals
                    .big_block
                    .iter()
                    .chain(signals.defensive_scaling_or_mitigation.iter())
                    .cloned()
                    .collect::<Vec<_>>(),
            ),
            counterevidence: vec![],
            unknown: vec![],
        }
    } else {
        AwakenedOneEvidenceClaim {
            claim: "phase2_dark_echo_plan",
            status: "unsupported",
            support: vec![],
            counterevidence: vec![
                "no obvious big block, mitigation, or block engine for the phase-2 Dark Echo turn"
                    .to_string(),
            ],
            unknown: vec![
                "search/replay evidence could still show a specific transition line".to_string(),
            ],
        }
    }
}

fn power_penalty_claim(signals: &AwakenedOneDeckSignals) -> AwakenedOneEvidenceClaim {
    if signals.powers.is_empty() {
        AwakenedOneEvidenceClaim {
            claim: "awakened_one_power_penalty_exposure",
            status: "not_present",
            support: vec![],
            counterevidence: vec!["no Power cards in deck".to_string()],
            unknown: vec![],
        }
    } else {
        AwakenedOneEvidenceClaim {
            claim: "awakened_one_power_penalty_exposure",
            status: "supported",
            support: card_labels(&signals.powers),
            counterevidence: power_counterevidence(signals),
            unknown: vec!["actual Power timing requires replay evidence".to_string()],
        }
    }
}

fn deck_clean_not_sufficient_claim(signals: &AwakenedOneDeckSignals) -> AwakenedOneEvidenceClaim {
    let mut support = Vec::new();
    if signals.deck.len() <= 18 {
        support.push(format!("small deck_size={}", signals.deck.len()));
    }
    if signals.curses.is_empty() {
        support.push("no curse burden".to_string());
    }
    AwakenedOneEvidenceClaim {
        claim: "clean_deck_does_not_imply_boss_plan_sufficient",
        status: if support.is_empty() {
            "unknown"
        } else {
            "supported"
        },
        support,
        counterevidence: vec![
            "deck cleanliness is separate from defensive scaling, mitigation, and phase-transition planning".to_string(),
        ],
        unknown: vec![],
    }
}

fn full_hp_probe_claim(probe: &CounterfactualHpProbe) -> AwakenedOneEvidenceClaim {
    let mut support = vec![
        format!("hp_probe_classification={}", probe.classification_label()),
        format!("original_hp={}/{}", probe.original_hp(), probe.max_hp()),
    ];
    if let Some(enemy_hp) = probe.full_hp_best_progress_enemy_hp() {
        support.push(format!("full_hp_best_enemy_hp_remaining={enemy_hp}"));
    }
    if let Some(turns) = probe.full_hp_best_progress_turns() {
        support.push(format!("full_hp_best_progress_turns={turns}"));
    }
    AwakenedOneEvidenceClaim {
        claim: "full_hp_counterfactual_probe",
        status: if probe.full_hp_complete_win() == Some(false) && !probe.any_complete_win() {
            "supports_not_low_hp_only"
        } else if probe.any_complete_win() {
            "counterfactual_win_found"
        } else {
            "unknown"
        },
        support,
        counterevidence: vec![],
        unknown: vec!["review lanes are not a proof of unwinnability".to_string()],
    }
}

fn risk_tags(claims: &[AwakenedOneEvidenceClaim]) -> Vec<&'static str> {
    let mut tags = Vec::new();
    for claim in claims {
        match (claim.claim, claim.status) {
            ("damage_scaling_present", "single_slow_source") => {
                tags.push("single_slow_damage_scaling_source")
            }
            ("defensive_scaling_or_mitigation_present", "unsupported") => {
                tags.push("missing_defensive_scaling_or_mitigation")
            }
            ("cultist_deadline_plan", "unsupported" | "weak_supported") => {
                tags.push("cultist_cleanup_deadline_uncertain")
            }
            ("phase2_dark_echo_plan", "unsupported") => tags.push("phase2_dark_echo_plan_missing"),
            ("awakened_one_power_penalty_exposure", "supported") => {
                tags.push("awakened_one_power_penalty_exposure")
            }
            ("full_hp_counterfactual_probe", "supports_not_low_hp_only") => {
                tags.push("full_hp_no_win_found");
            }
            _ => {}
        }
    }
    tags.sort();
    tags.dedup();
    tags
}

fn conclusion_from_risk_tags(risk_tags: &[&'static str]) -> &'static str {
    if risk_tags.iter().any(|tag| *tag == "full_hp_no_win_found")
        && risk_tags
            .iter()
            .any(|tag| *tag == "missing_defensive_scaling_or_mitigation")
    {
        "likely_boss_plan_insufficient_not_low_hp_only"
    } else if risk_tags
        .iter()
        .any(|tag| *tag == "missing_defensive_scaling_or_mitigation")
    {
        "boss_plan_thin_with_missing_survival_plan"
    } else {
        "awakened_one_boss_plan_needs_review"
    }
}

fn start_evidence(
    combat: &CombatState,
    signals: &AwakenedOneDeckSignals,
) -> AwakenedOneStartEvidence {
    let awakened = combat
        .entities
        .monsters
        .iter()
        .find(|monster| EnemyId::from_id(monster.monster_type) == Some(EnemyId::AwakenedOne));
    AwakenedOneStartEvidence {
        turn: combat.turn.turn_count,
        player_hp: combat.entities.player.current_hp,
        player_max_hp: combat.entities.player.max_hp,
        deck_size: signals.deck.len(),
        power_cards: card_labels(&signals.powers),
        cultists_alive: combat
            .entities
            .monsters
            .iter()
            .filter(|monster| EnemyId::from_id(monster.monster_type) == Some(EnemyId::Cultist))
            .filter(|monster| monster.is_alive_for_action())
            .count(),
        awakened_hp: awakened.map(|monster| monster.current_hp),
        awakened_max_hp: awakened.map(|monster| monster.max_hp),
        total_enemy_hp: combat
            .entities
            .monsters
            .iter()
            .filter(|monster| monster.is_alive_for_action())
            .map(|monster| monster.current_hp.max(0) + monster.block.max(0))
            .sum(),
    }
}

fn is_awakened_one_case(combat: &CombatState) -> bool {
    combat
        .entities
        .monsters
        .iter()
        .any(|monster| EnemyId::from_id(monster.monster_type) == Some(EnemyId::AwakenedOne))
}

fn card_labels(cards: &[CombatCard]) -> Vec<String> {
    cards.iter().map(card_label).collect()
}

fn card_label(card: &CombatCard) -> String {
    format!("{}+{}", cards::java_id(card.id), card.upgrades)
}

fn filter_cards(
    signals: &AwakenedOneDeckSignals,
    predicate: fn(CardId) -> bool,
) -> Vec<CombatCard> {
    signals
        .deck
        .iter()
        .filter(|card| predicate(card.id))
        .cloned()
        .collect()
}

fn power_counterevidence(signals: &AwakenedOneDeckSignals) -> Vec<String> {
    let mut items = Vec::new();
    if signals.powers.iter().any(|card| card.id == CardId::Rupture)
        && !signals.deck.iter().any(|card| is_self_damage(card.id))
    {
        items.push(
            "Rupture has no stable self-damage engine and may be Burning Pact fuel".to_string(),
        );
    }
    if signals
        .powers
        .iter()
        .any(|card| card.id == CardId::DemonForm)
    {
        items.push("Demon Form is valuable scaling but triggers Curiosity and is slow".to_string());
    }
    items
}

fn is_power(card: CardId) -> bool {
    matches!(
        card,
        CardId::DemonForm
            | CardId::Rupture
            | CardId::Barricade
            | CardId::Corruption
            | CardId::FeelNoPain
            | CardId::DarkEmbrace
            | CardId::Inflame
            | CardId::Metallicize
            | CardId::Combust
            | CardId::Brutality
            | CardId::FireBreathing
            | CardId::Evolve
            | CardId::Juggernaut
            | CardId::Berserk
    )
}

fn is_damage_scaling(card: CardId) -> bool {
    matches!(
        card,
        CardId::DemonForm | CardId::LimitBreak | CardId::Inflame | CardId::SpotWeakness
    )
}

fn is_defensive_scaling_or_mitigation(card: CardId) -> bool {
    matches!(
        card,
        CardId::Disarm
            | CardId::Shockwave
            | CardId::Impervious
            | CardId::PowerThrough
            | CardId::FlameBarrier
            | CardId::FeelNoPain
            | CardId::SecondWind
            | CardId::Barricade
            | CardId::Entrench
            | CardId::Corruption
            | CardId::TrueGrit
            | CardId::Metallicize
    )
}

fn is_big_block(card: CardId) -> bool {
    matches!(
        card,
        CardId::Impervious | CardId::PowerThrough | CardId::FlameBarrier
    )
}

fn is_generic_block(card: CardId) -> bool {
    matches!(
        card,
        CardId::Defend | CardId::ShrugItOff | CardId::Armaments | CardId::GhostlyArmor
    )
}

fn is_aoe(card: CardId) -> bool {
    matches!(
        card,
        CardId::Whirlwind | CardId::Cleave | CardId::Immolate | CardId::Combust
    )
}

fn is_access(card: CardId) -> bool {
    matches!(
        card,
        CardId::BurningPact | CardId::Offering | CardId::BattleTrance | CardId::ShrugItOff
    )
}

fn is_self_damage(card: CardId) -> bool {
    matches!(
        card,
        CardId::Offering
            | CardId::Bloodletting
            | CardId::Hemokinesis
            | CardId::Combust
            | CardId::Brutality
    )
}

fn is_curse(card: CardId) -> bool {
    matches!(
        card,
        CardId::Writhe
            | CardId::Normality
            | CardId::Regret
            | CardId::Pain
            | CardId::Parasite
            | CardId::Decay
            | CardId::Doubt
            | CardId::Shame
            | CardId::Injury
            | CardId::Clumsy
            | CardId::CurseOfTheBell
            | CardId::Necronomicurse
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn claim(claim: &'static str, status: &'static str) -> AwakenedOneEvidenceClaim {
        AwakenedOneEvidenceClaim {
            claim,
            status,
            support: Vec::new(),
            counterevidence: Vec::new(),
            unknown: Vec::new(),
        }
    }

    #[test]
    fn full_hp_risk_requires_counterfactual_hp_evidence() {
        let tags = risk_tags(&[
            claim("full_hp_counterfactual_probe", "unknown"),
            claim("defensive_scaling_or_mitigation_present", "unsupported"),
        ]);

        assert!(!tags.contains(&"full_hp_no_win_found"));
        assert_eq!(
            conclusion_from_risk_tags(&tags),
            "boss_plan_thin_with_missing_survival_plan"
        );
    }

    #[test]
    fn expected_awakened_one_case_tags_imply_low_hp_is_not_enough() {
        let tags = risk_tags(&[
            claim("damage_scaling_present", "single_slow_source"),
            claim("defensive_scaling_or_mitigation_present", "unsupported"),
            claim("cultist_deadline_plan", "weak_supported"),
            claim("phase2_dark_echo_plan", "unsupported"),
            claim("awakened_one_power_penalty_exposure", "supported"),
            claim("full_hp_counterfactual_probe", "supports_not_low_hp_only"),
        ]);

        for expected in [
            "awakened_one_power_penalty_exposure",
            "cultist_cleanup_deadline_uncertain",
            "full_hp_no_win_found",
            "missing_defensive_scaling_or_mitigation",
            "phase2_dark_echo_plan_missing",
            "single_slow_damage_scaling_source",
        ] {
            assert!(tags.contains(&expected), "missing {expected}");
        }
        assert_eq!(
            conclusion_from_risk_tags(&tags),
            "likely_boss_plan_insufficient_not_low_hp_only"
        );
    }
}
