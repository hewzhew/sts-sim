use crate::content::monsters::factory::EncounterId;
use crate::state::run::RunState;

use super::types::{StrategyThreatProfileV1, StrategyThreatTagV1};

pub fn threat_profile_from_run_state_v1(run_state: &RunState) -> StrategyThreatProfileV1 {
    let mut profile = StrategyThreatProfileV1 {
        boss: run_state.boss_key.map(|boss| format!("{boss:?}")),
        tags: Vec::new(),
        evidence: Vec::new(),
    };

    if let Some(boss) = run_state.boss_key {
        add_boss_threats(boss, &mut profile);
    }
    add_act_elite_pool_threats(run_state.act_num, &mut profile);

    profile
}

pub fn empty_threat_profile_v1() -> StrategyThreatProfileV1 {
    StrategyThreatProfileV1::default()
}

fn add_boss_threats(boss: EncounterId, profile: &mut StrategyThreatProfileV1) {
    match boss {
        EncounterId::TheGuardian => {
            push_tags(
                profile,
                &[
                    StrategyThreatTagV1::HighIncomingDamage,
                    StrategyThreatTagV1::ModeShiftThreshold,
                    StrategyThreatTagV1::WeakValuable,
                ],
            );
            profile
                .evidence
                .push("Act boss The Guardian: mode shift and large attacks reward controlled damage timing and mitigation".to_string());
        }
        EncounterId::Hexaghost => {
            push_tags(
                profile,
                &[
                    StrategyThreatTagV1::MultiHit,
                    StrategyThreatTagV1::StatusFlood,
                    StrategyThreatTagV1::HighIncomingDamage,
                ],
            );
            profile
                .evidence
                .push("Act boss Hexaghost: multi-hit attacks and Burn pressure reward mitigation and race damage".to_string());
        }
        EncounterId::SlimeBoss => {
            push_tags(
                profile,
                &[
                    StrategyThreatTagV1::SplitThreshold,
                    StrategyThreatTagV1::StatusFlood,
                    StrategyThreatTagV1::AoEValuable,
                    StrategyThreatTagV1::HighIncomingDamage,
                ],
            );
            profile
                .evidence
                .push("Act boss Slime Boss: split threshold and status pressure reward burst timing and post-split AoE".to_string());
        }
        EncounterId::Automaton => {
            push_tags(
                profile,
                &[
                    StrategyThreatTagV1::ArtifactBlocksDebuff,
                    StrategyThreatTagV1::AoEValuable,
                    StrategyThreatTagV1::HighIncomingDamage,
                    StrategyThreatTagV1::SetupWindow,
                ],
            );
            profile
                .evidence
                .push("Act boss Bronze Automaton: artifact, minions, and Hyper Beam reward setup plus artifact-aware control".to_string());
        }
        EncounterId::TheChamp => {
            push_tags(
                profile,
                &[
                    StrategyThreatTagV1::StrengthDebuffValuable,
                    StrategyThreatTagV1::WeakValuable,
                    StrategyThreatTagV1::HighIncomingDamage,
                    StrategyThreatTagV1::LongFightScaling,
                ],
            );
            profile
                .evidence
                .push("Act boss Champ: long fight and execute phase reward strength down, weak, and scaling".to_string());
        }
        EncounterId::AwakenedOne => {
            push_tags(
                profile,
                &[
                    StrategyThreatTagV1::PowerPunish,
                    StrategyThreatTagV1::AoEValuable,
                    StrategyThreatTagV1::LongFightScaling,
                    StrategyThreatTagV1::HighIncomingDamage,
                ],
            );
            profile
                .evidence
                .push("Act boss Awakened One: power scaling, cultists, and second phase reward careful power use and long-fight plans".to_string());
        }
        EncounterId::TimeEater => {
            push_tags(
                profile,
                &[
                    StrategyThreatTagV1::CardPlayLimit,
                    StrategyThreatTagV1::LongFightScaling,
                    StrategyThreatTagV1::HighIncomingDamage,
                    StrategyThreatTagV1::WeakValuable,
                ],
            );
            profile
                .evidence
                .push("Act boss Time Eater: card play limit and long fight reward dense turns, mitigation, and scaling".to_string());
        }
        _ => {}
    }
}

fn add_act_elite_pool_threats(act: u8, profile: &mut StrategyThreatProfileV1) {
    match act {
        1 => {
            push_tags(
                profile,
                &[
                    StrategyThreatTagV1::SkillPunish,
                    StrategyThreatTagV1::StatusFlood,
                    StrategyThreatTagV1::AoEValuable,
                    StrategyThreatTagV1::StrengthDebuffValuable,
                    StrategyThreatTagV1::SetupWindow,
                ],
            );
            profile.evidence.push(
                "Act 1 elite pool: Nob punishes skills, Sentries flood statuses, Lagavulin rewards setup/debuff answers"
                    .to_string(),
            );
        }
        2 => {
            push_tags(
                profile,
                &[
                    StrategyThreatTagV1::HighIncomingDamage,
                    StrategyThreatTagV1::AoEValuable,
                    StrategyThreatTagV1::MultiHit,
                ],
            );
            profile.evidence.push(
                "Act 2 elite pool: Slavers/Gremlin Leader/Book reward frontload, AoE, and mitigation"
                    .to_string(),
            );
        }
        3 => {
            push_tags(
                profile,
                &[
                    StrategyThreatTagV1::HighIncomingDamage,
                    StrategyThreatTagV1::AoEValuable,
                    StrategyThreatTagV1::LongFightScaling,
                ],
            );
            profile.evidence.push(
                "Act 3 elite pool: Reptomancer/Nemesis/Giant Head reward burst control, AoE, and long-fight scaling"
                    .to_string(),
            );
        }
        _ => {}
    }
}

fn push_tags(profile: &mut StrategyThreatProfileV1, tags: &[StrategyThreatTagV1]) {
    for tag in tags {
        if !profile.tags.contains(tag) {
            profile.tags.push(*tag);
        }
    }
}
