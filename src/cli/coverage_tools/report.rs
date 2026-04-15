use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::io;
use std::path::Path;

use serde::Serialize;

use crate::bot::coverage_signatures::{InteractionSignature, ObservedInteractionRecord};

#[derive(Default, Debug, Serialize)]
pub struct InteractionCoverageReport {
    pub generated_from: Vec<String>,
    pub total_records: usize,
    pub unique_signatures: usize,
    pub unique_sources: usize,
    pub observed_from_counts: BTreeMap<String, usize>,
    pub source_signature_counts: BTreeMap<String, usize>,
    pub archetype_signature_counts: BTreeMap<String, usize>,
    pub archetype_source_counts: BTreeMap<String, usize>,
    pub archetype_sources: BTreeMap<String, Vec<String>>,
    pub low_diversity_sources: Vec<String>,
    pub low_order_sources: Vec<String>,
    pub high_value_signature_counts: BTreeMap<String, usize>,
    pub high_value_undercovered_sources: Vec<String>,
    pub uncovered_cards: Vec<String>,
    pub uncovered_potions: Vec<String>,
    pub notes: Vec<String>,
}

pub fn write_coverage_outputs(
    records: &[ObservedInteractionRecord],
    generated_from: Vec<String>,
    coverage_path: &Path,
    report_path: &Path,
    notes: Vec<String>,
) -> std::io::Result<()> {
    let mut unique_signatures = HashSet::new();
    let mut source_signature_counts: BTreeMap<String, usize> = BTreeMap::new();
    let mut archetype_signature_counts: BTreeMap<String, usize> = BTreeMap::new();
    let mut archetype_source_sets: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    let mut observed_from_counts: BTreeMap<String, usize> = BTreeMap::new();
    let mut high_value_signature_counts: BTreeMap<String, usize> = BTreeMap::new();
    let mut high_value_source_flags: HashMap<String, BTreeSet<String>> = HashMap::new();
    let mut source_unique_signature_sets: HashMap<String, BTreeSet<String>> = HashMap::new();

    for record in records {
        unique_signatures.insert(record.signature_key.clone());
        *observed_from_counts
            .entry(record.observed_from.clone())
            .or_insert(0) += 1;
        *source_signature_counts
            .entry(record.signature.source_id.clone())
            .or_insert(0) += 1;
        for tag in &record.signature.archetype_tags {
            *archetype_signature_counts.entry(tag.clone()).or_insert(0) += 1;
            archetype_source_sets
                .entry(tag.clone())
                .or_default()
                .insert(record.signature.source_id.clone());
        }
        source_unique_signature_sets
            .entry(record.signature.source_id.clone())
            .or_default()
            .insert(record.signature_key.clone());

        let flags = high_value_flags(&record.signature);
        for flag in &flags {
            *high_value_signature_counts.entry(flag.clone()).or_insert(0) += 1;
        }
        high_value_source_flags
            .entry(record.signature.source_id.clone())
            .or_default()
            .extend(flags);
    }

    let low_diversity_sources: Vec<String> = source_signature_counts
        .iter()
        .filter(|(_, count)| **count <= 1)
        .map(|(source, _)| source.clone())
        .collect();

    let low_order_sources: Vec<String> = source_unique_signature_sets
        .iter()
        .filter(|(source, signatures)| {
            signatures.len() <= 1
                && high_value_source_flags
                    .get(*source)
                    .is_none_or(|flags| flags.is_empty())
        })
        .map(|(source, _)| source.clone())
        .collect();

    let high_value_undercovered_sources: Vec<String> = source_signature_counts
        .keys()
        .filter(|source| {
            let flags = high_value_source_flags.get(*source);
            flags.is_none_or(|f| f.is_empty())
        })
        .cloned()
        .collect();

    let observed_sources: HashSet<&str> =
        source_signature_counts.keys().map(String::as_str).collect();
    let uncovered_cards: Vec<String> = known_card_sources()
        .into_iter()
        .filter(|name| !observed_sources.contains(name.as_str()))
        .collect();
    let uncovered_potions: Vec<String> = known_potion_sources()
        .into_iter()
        .filter(|name| !observed_sources.contains(name.as_str()))
        .collect();

    let report = InteractionCoverageReport {
        generated_from,
        total_records: records.len(),
        unique_signatures: unique_signatures.len(),
        unique_sources: source_signature_counts.len(),
        observed_from_counts,
        source_signature_counts,
        archetype_signature_counts,
        archetype_source_counts: archetype_source_sets
            .iter()
            .map(|(tag, sources)| (tag.clone(), sources.len()))
            .collect(),
        archetype_sources: archetype_source_sets
            .into_iter()
            .map(|(tag, sources)| (tag, sources.into_iter().collect()))
            .collect(),
        low_diversity_sources,
        low_order_sources,
        high_value_signature_counts,
        high_value_undercovered_sources,
        uncovered_cards,
        uncovered_potions,
        notes,
    };

    if let Some(parent) = coverage_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    if let Some(parent) = report_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let coverage_json = serde_json::to_string_pretty(records)
        .map_err(|err| io::Error::other(format!("serialize coverage records failed: {err}")))?;
    let report_json = serde_json::to_string_pretty(&report)
        .map_err(|err| io::Error::other(format!("serialize coverage report failed: {err}")))?;

    std::fs::write(coverage_path, coverage_json)?;
    std::fs::write(report_path, report_json)?;
    Ok(())
}

fn high_value_flags(signature: &InteractionSignature) -> Vec<String> {
    let mut flags = Vec::new();
    if signature.pending_choice != "none" {
        flags.push("pending_choice".to_string());
    }
    if signature
        .pile_tags
        .iter()
        .any(|tag| matches!(tag.as_str(), "exhaust" | "draw" | "shuffle"))
    {
        flags.push("pile_chain".to_string());
    }
    if signature
        .outcome_tags
        .iter()
        .any(|tag| matches!(tag.as_str(), "half_dead" | "revives" | "spawns"))
    {
        flags.push("rebirth_or_spawn".to_string());
    }
    if signature.power_tags.iter().any(|tag| {
        matches!(
            tag.as_str(),
            "Artifact" | "NoDraw" | "Shackled" | "Malleable" | "CurlUp"
        )
    }) {
        flags.push("reactive_power".to_string());
    }
    flags
}

fn known_card_sources() -> Vec<String> {
    let mut ids: Vec<_> = crate::content::cards::build_java_id_map()
        .values()
        .copied()
        .collect();
    ids.sort_by_key(|id| crate::content::cards::get_card_definition(*id).name);
    ids.dedup();
    ids.into_iter()
        .filter_map(|id| {
            let def = crate::content::cards::get_card_definition(id);
            match def.card_type {
                crate::content::cards::CardType::Attack
                | crate::content::cards::CardType::Skill
                | crate::content::cards::CardType::Power => Some(def.name.to_string()),
                _ => None,
            }
        })
        .collect()
}

fn known_potion_sources() -> Vec<String> {
    all_potion_ids()
        .into_iter()
        .map(|id| {
            crate::content::potions::get_potion_definition(id)
                .name
                .to_string()
        })
        .collect()
}

fn all_potion_ids() -> Vec<crate::content::potions::PotionId> {
    use crate::content::potions::PotionId::*;
    vec![
        FirePotion,
        ExplosivePotion,
        PoisonPotion,
        WeakenPotion,
        FearPotion,
        BlockPotion,
        BloodPotion,
        EnergyPotion,
        StrengthPotion,
        DexterityPotion,
        SpeedPotion,
        SteroidPotion,
        SwiftPotion,
        FocusPotion,
        AttackPotion,
        SkillPotion,
        PowerPotion,
        ColorlessPotion,
        BottledMiracle,
        BlessingOfTheForge,
        AncientPotion,
        RegenPotion,
        EssenceOfSteel,
        LiquidBronze,
        DistilledChaosPotion,
        DuplicationPotion,
        CunningPotion,
        PotionOfCapacity,
        LiquidMemories,
        GamblersBrew,
        Elixir,
        StancePotion,
        FairyPotion,
        SmokeBomb,
        FruitJuice,
        EntropicBrew,
        SneckoOil,
        GhostInAJar,
        HeartOfIron,
        CultistPotion,
        Ambrosia,
        EssenceOfDarkness,
    ]
}
