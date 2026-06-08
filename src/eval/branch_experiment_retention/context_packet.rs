use std::collections::BTreeSet;

use crate::ai::card_reward_policy_v1::{CardRewardSemanticProfileV1, CardRewardSemanticRoleV1};
use crate::ai::noncombat_strategy_v1::{StrategyDeckFormationNeedV1, StrategyPackageIdV2};

use super::{
    complete_package_count, profile_has_any_role, transition_attack_count,
    BranchRetentionCandidateInputV1, DEFENSE_ENGINE_ROLES, SCALING_ROLES,
};

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub(super) enum BranchRetentionContextKeyV2 {
    MatchesFormationFrontloadNeed,
    MatchesFormationBlockNeed,
    MatchesFormationScalingNeed,
    MatchesFormationDrawEnergyNeed,
    MatchesFormationConsistencyNeed,
    OpensPackageSetup,
    ClosesPackage,
    SupportsCommittedPackage,
    ImmediateSafetyPatch,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(super) struct BranchRetentionContextPacketV2 {
    pub(super) keys: BTreeSet<BranchRetentionContextKeyV2>,
}

pub(super) fn branch_retention_context_packet_v2(
    candidate: &BranchRetentionCandidateInputV1,
) -> BranchRetentionContextPacketV2 {
    let mut packet = BranchRetentionContextPacketV2::default();
    let Some(formation) = candidate.strategy_formation.as_ref() else {
        return packet;
    };

    if formation
        .needs
        .contains(&StrategyDeckFormationNeedV1::Frontload)
        && candidate.choice_profiles.iter().any(|profile| {
            profile
                .roles
                .contains(&CardRewardSemanticRoleV1::FrontloadDamage)
        })
    {
        packet
            .keys
            .insert(BranchRetentionContextKeyV2::MatchesFormationFrontloadNeed);
    }
    if formation
        .needs
        .contains(&StrategyDeckFormationNeedV1::Block)
        && candidate
            .choice_profiles
            .iter()
            .any(|profile| profile_has_any_role(profile, DEFENSE_ENGINE_ROLES))
    {
        packet
            .keys
            .insert(BranchRetentionContextKeyV2::MatchesFormationBlockNeed);
    }
    if formation
        .needs
        .contains(&StrategyDeckFormationNeedV1::Scaling)
        && candidate
            .choice_profiles
            .iter()
            .any(|profile| profile_has_any_role(profile, SCALING_ROLES))
    {
        packet
            .keys
            .insert(BranchRetentionContextKeyV2::MatchesFormationScalingNeed);
    }
    if formation
        .needs
        .contains(&StrategyDeckFormationNeedV1::DrawEnergy)
        && candidate
            .choice_profiles
            .iter()
            .any(|profile| profile_has_any_role(profile, DRAW_ENERGY_ROLES))
    {
        packet
            .keys
            .insert(BranchRetentionContextKeyV2::MatchesFormationDrawEnergyNeed);
    }
    if formation
        .needs
        .contains(&StrategyDeckFormationNeedV1::Consistency)
        && transition_attack_count(&candidate.choice_profiles) == 0
    {
        packet
            .keys
            .insert(BranchRetentionContextKeyV2::MatchesFormationConsistencyNeed);
    }
    if !candidate.trajectory.setup_keys.is_empty() {
        packet
            .keys
            .insert(BranchRetentionContextKeyV2::OpensPackageSetup);
    }
    if complete_package_count(&candidate.trajectory) > 0 {
        packet
            .keys
            .insert(BranchRetentionContextKeyV2::ClosesPackage);
    }
    if formation.strengths.iter().any(|package| {
        choice_profiles_support_committed_package(*package, &candidate.choice_profiles)
    }) {
        packet
            .keys
            .insert(BranchRetentionContextKeyV2::SupportsCommittedPackage);
    }
    if candidate.max_hp > 0
        && candidate.hp * 100 < candidate.max_hp * 65
        && candidate.choice_profiles.iter().any(|profile| {
            profile
                .roles
                .contains(&CardRewardSemanticRoleV1::FrontloadDamage)
                || profile_has_any_role(profile, DEFENSE_ENGINE_ROLES)
        })
    {
        packet
            .keys
            .insert(BranchRetentionContextKeyV2::ImmediateSafetyPatch);
    }

    packet
}

pub(super) fn context_score(
    packet: &BranchRetentionContextPacketV2,
    keys: &[BranchRetentionContextKeyV2],
) -> i32 {
    keys.iter().filter(|key| packet.keys.contains(key)).count() as i32
}

fn choice_profiles_support_committed_package(
    package: StrategyPackageIdV2,
    profiles: &[CardRewardSemanticProfileV1],
) -> bool {
    match package {
        StrategyPackageIdV2::FrontloadSurvival => profiles.iter().any(|profile| {
            profile
                .roles
                .contains(&CardRewardSemanticRoleV1::FrontloadDamage)
                || profile_has_any_role(profile, DEFENSE_ENGINE_ROLES)
        }),
        StrategyPackageIdV2::WeakControl => profiles.iter().any(|profile| {
            profile.roles.contains(&CardRewardSemanticRoleV1::Weak)
                || profile
                    .roles
                    .contains(&CardRewardSemanticRoleV1::EnemyStrengthDown)
        }),
        StrategyPackageIdV2::StrengthScaling => profiles
            .iter()
            .any(|profile| profile_has_any_role(profile, STRENGTH_PACKAGE_ROLES)),
        StrategyPackageIdV2::UpgradeSink => profiles
            .iter()
            .any(|profile| profile_has_any_role(profile, UPGRADE_PACKAGE_ROLES)),
        StrategyPackageIdV2::ExhaustEngine => profiles
            .iter()
            .any(|profile| profile_has_any_role(profile, EXHAUST_PACKAGE_ROLES)),
        StrategyPackageIdV2::BlockEngine => profiles
            .iter()
            .any(|profile| profile_has_any_role(profile, BLOCK_PACKAGE_ROLES)),
        StrategyPackageIdV2::StrikeDensity => profiles.iter().any(|profile| {
            profile
                .roles
                .contains(&CardRewardSemanticRoleV1::StrikePayoff)
        }),
        StrategyPackageIdV2::StatusPackage => profiles
            .iter()
            .any(|profile| profile_has_any_role(profile, STATUS_PACKAGE_ROLES)),
        StrategyPackageIdV2::SelfDamage => profiles.iter().any(|profile| {
            profile
                .roles
                .contains(&CardRewardSemanticRoleV1::SelfDamagePayoff)
        }),
        StrategyPackageIdV2::EnergyDraw => profiles
            .iter()
            .any(|profile| profile_has_any_role(profile, DRAW_ENERGY_ROLES)),
        StrategyPackageIdV2::CombatPatchWindow
        | StrategyPackageIdV2::CorePlanProtection
        | StrategyPackageIdV2::RecoveryPressure
        | StrategyPackageIdV2::HpSafety => profiles.iter().any(|profile| {
            profile
                .roles
                .contains(&CardRewardSemanticRoleV1::FrontloadDamage)
                || profile_has_any_role(profile, DEFENSE_ENGINE_ROLES)
        }),
        StrategyPackageIdV2::UpgradeCommitment => profiles
            .iter()
            .any(|profile| profile_has_any_role(profile, UPGRADE_PACKAGE_ROLES)),
        StrategyPackageIdV2::GoldPlan
        | StrategyPackageIdV2::PotionCapacity
        | StrategyPackageIdV2::ShopRemoveWindow
        | StrategyPackageIdV2::RelicConstraints => false,
    }
}

const DRAW_ENERGY_ROLES: &[CardRewardSemanticRoleV1] = &[
    CardRewardSemanticRoleV1::CardDraw,
    CardRewardSemanticRoleV1::EnergySource,
];

const STRENGTH_PACKAGE_ROLES: &[CardRewardSemanticRoleV1] = &[
    CardRewardSemanticRoleV1::ScalingSource,
    CardRewardSemanticRoleV1::StrengthPayoff,
];

const UPGRADE_PACKAGE_ROLES: &[CardRewardSemanticRoleV1] =
    &[CardRewardSemanticRoleV1::UpgradePayoff];

const EXHAUST_PACKAGE_ROLES: &[CardRewardSemanticRoleV1] = &[
    CardRewardSemanticRoleV1::ExhaustGenerator,
    CardRewardSemanticRoleV1::ExhaustPayoff,
];

const BLOCK_PACKAGE_ROLES: &[CardRewardSemanticRoleV1] = &[
    CardRewardSemanticRoleV1::Block,
    CardRewardSemanticRoleV1::BlockRetention,
    CardRewardSemanticRoleV1::BlockMultiplier,
    CardRewardSemanticRoleV1::BlockPayoff,
];

const STATUS_PACKAGE_ROLES: &[CardRewardSemanticRoleV1] = &[
    CardRewardSemanticRoleV1::StatusGenerator,
    CardRewardSemanticRoleV1::StatusPayoff,
];
