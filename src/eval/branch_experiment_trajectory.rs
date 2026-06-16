use crate::ai::card_reward_policy_v1::{CardRewardSemanticProfileV1, CardRewardSemanticRoleV1};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BranchTrajectorySignatureV1 {
    pub frontload_picks: u8,
    pub transition_frontload_picks: u8,
    pub scaling_picks: u8,
    pub defense_picks: u8,
    pub engine_generator_picks: u8,
    pub engine_payoff_picks: u8,
    pub draw_energy_picks: u8,
    pub setup_keys: Vec<String>,
    pub package_keys: Vec<String>,
}

pub fn summarize_branch_trajectory_v1(
    profiles: &[CardRewardSemanticProfileV1],
) -> BranchTrajectorySignatureV1 {
    let mut signature = BranchTrajectorySignatureV1::default();

    for profile in profiles {
        let has_frontload = has_role(profile, CardRewardSemanticRoleV1::FrontloadDamage);
        let has_non_transition_role = profile.roles.iter().any(is_non_transition_role);
        if has_frontload {
            signature.frontload_picks = signature.frontload_picks.saturating_add(1);
            if !has_non_transition_role {
                signature.transition_frontload_picks =
                    signature.transition_frontload_picks.saturating_add(1);
            }
        }
        if profile.roles.iter().any(is_scaling_role) {
            signature.scaling_picks = signature.scaling_picks.saturating_add(1);
        }
        if profile.roles.iter().any(is_defense_role) {
            signature.defense_picks = signature.defense_picks.saturating_add(1);
        }
        if profile.roles.iter().any(is_engine_generator_role) {
            signature.engine_generator_picks = signature.engine_generator_picks.saturating_add(1);
        }
        if profile.roles.iter().any(is_engine_payoff_role) {
            signature.engine_payoff_picks = signature.engine_payoff_picks.saturating_add(1);
        }
        if profile.roles.iter().any(|role| {
            matches!(
                role,
                CardRewardSemanticRoleV1::CardDraw | CardRewardSemanticRoleV1::EnergySource
            )
        }) {
            signature.draw_energy_picks = signature.draw_energy_picks.saturating_add(1);
        }

        for key in setup_keys_for_profile(profile) {
            push_key(&mut signature.setup_keys, key);
        }
        for key in payoff_package_keys_for_profile(profile) {
            push_key(&mut signature.package_keys, key);
        }
    }

    signature.setup_keys.sort();
    signature.package_keys.sort();
    signature
}

pub fn branch_trajectory_key_v1(signature: &BranchTrajectorySignatureV1) -> String {
    let setups = if signature.setup_keys.is_empty() {
        "-".to_string()
    } else {
        signature.setup_keys.join("+")
    };
    let packages = if signature.package_keys.is_empty() {
        "-".to_string()
    } else {
        signature.package_keys.join("+")
    };
    format!(
        "setup={setups}|pkg={packages}|frontload={}|transition={}|scaling={}|defense={}|engine_gen={}|engine_payoff={}|draw_energy={}",
        signature.frontload_picks,
        signature.transition_frontload_picks,
        signature.scaling_picks,
        signature.defense_picks,
        signature.engine_generator_picks,
        signature.engine_payoff_picks,
        signature.draw_energy_picks
    )
}

pub fn branch_trajectory_family_key_v1(signature: &BranchTrajectorySignatureV1) -> String {
    let setups = if signature.setup_keys.is_empty() {
        "-".to_string()
    } else {
        signature.setup_keys.join("+")
    };
    let packages = if signature.package_keys.is_empty() {
        "-".to_string()
    } else {
        signature.package_keys.join("+")
    };
    format!(
        "setup={setups}|pkg={packages}|transition={}|engine_gen={}|engine_payoff={}|defense={}|draw_energy={}",
        count_bucket(signature.transition_frontload_picks),
        presence_bucket(signature.engine_generator_picks),
        presence_bucket(signature.engine_payoff_picks),
        presence_bucket(signature.defense_picks),
        presence_bucket(signature.draw_energy_picks),
    )
}

fn count_bucket(value: u8) -> &'static str {
    match value {
        0 => "0",
        1 => "1",
        _ => "2plus",
    }
}

fn presence_bucket(value: u8) -> &'static str {
    if value == 0 {
        "0"
    } else {
        "1plus"
    }
}

fn setup_keys_for_profile(profile: &CardRewardSemanticProfileV1) -> Vec<&'static str> {
    let mut keys = Vec::new();
    if profile.roles.iter().any(|role| {
        matches!(
            role,
            CardRewardSemanticRoleV1::BlockRetention | CardRewardSemanticRoleV1::BlockMultiplier
        )
    }) {
        keys.push("block_engine");
    }
    if profile.roles.iter().any(|role| {
        matches!(
            role,
            CardRewardSemanticRoleV1::ScalingSource | CardRewardSemanticRoleV1::StrengthPayoff
        )
    }) {
        keys.push("strength_scaling");
    }
    if profile
        .roles
        .iter()
        .any(|role| matches!(role, CardRewardSemanticRoleV1::ExhaustGenerator))
    {
        keys.push("exhaust_engine");
    }
    if profile
        .roles
        .iter()
        .any(|role| matches!(role, CardRewardSemanticRoleV1::StatusGenerator))
    {
        keys.push("status_package");
    }
    keys
}

fn payoff_package_keys_for_profile(profile: &CardRewardSemanticProfileV1) -> Vec<&'static str> {
    let mut keys = Vec::new();
    if has_role(profile, CardRewardSemanticRoleV1::BlockPayoff) {
        keys.push("block_engine");
    }
    if has_role(profile, CardRewardSemanticRoleV1::StrengthPayoff) {
        keys.push("strength_scaling");
    }
    if has_role(profile, CardRewardSemanticRoleV1::StrikePayoff) {
        keys.push("strike_density");
    }
    if has_role(profile, CardRewardSemanticRoleV1::UpgradePayoff) {
        keys.push("upgrade_sink");
    }
    if has_role(profile, CardRewardSemanticRoleV1::ExhaustPayoff) {
        keys.push("exhaust_engine");
    }
    if has_role(profile, CardRewardSemanticRoleV1::StatusPayoff) {
        keys.push("status_package");
    }
    if has_role(profile, CardRewardSemanticRoleV1::SelfDamagePayoff) {
        keys.push("self_damage");
    }
    keys
}

fn has_role(profile: &CardRewardSemanticProfileV1, role: CardRewardSemanticRoleV1) -> bool {
    profile.roles.contains(&role)
}

fn is_non_transition_role(role: &CardRewardSemanticRoleV1) -> bool {
    matches!(
        role,
        CardRewardSemanticRoleV1::Block
            | CardRewardSemanticRoleV1::CardDraw
            | CardRewardSemanticRoleV1::EnergySource
            | CardRewardSemanticRoleV1::Vulnerable
            | CardRewardSemanticRoleV1::Weak
            | CardRewardSemanticRoleV1::EnemyStrengthDown
            | CardRewardSemanticRoleV1::ScalingSource
            | CardRewardSemanticRoleV1::StrengthPayoff
            | CardRewardSemanticRoleV1::BlockRetention
            | CardRewardSemanticRoleV1::BlockPayoff
            | CardRewardSemanticRoleV1::BlockMultiplier
            | CardRewardSemanticRoleV1::StrikePayoff
            | CardRewardSemanticRoleV1::UpgradePayoff
            | CardRewardSemanticRoleV1::ExhaustGenerator
            | CardRewardSemanticRoleV1::ExhaustPayoff
            | CardRewardSemanticRoleV1::StatusGenerator
            | CardRewardSemanticRoleV1::StatusPayoff
            | CardRewardSemanticRoleV1::SelfDamagePayoff
    )
}

fn is_scaling_role(role: &CardRewardSemanticRoleV1) -> bool {
    matches!(role, CardRewardSemanticRoleV1::ScalingSource)
}

fn is_defense_role(role: &CardRewardSemanticRoleV1) -> bool {
    matches!(
        role,
        CardRewardSemanticRoleV1::Block
            | CardRewardSemanticRoleV1::Weak
            | CardRewardSemanticRoleV1::EnemyStrengthDown
            | CardRewardSemanticRoleV1::BlockRetention
            | CardRewardSemanticRoleV1::BlockMultiplier
    )
}

fn is_engine_generator_role(role: &CardRewardSemanticRoleV1) -> bool {
    matches!(
        role,
        CardRewardSemanticRoleV1::ScalingSource
            | CardRewardSemanticRoleV1::BlockRetention
            | CardRewardSemanticRoleV1::BlockMultiplier
            | CardRewardSemanticRoleV1::ExhaustGenerator
            | CardRewardSemanticRoleV1::StatusGenerator
            | CardRewardSemanticRoleV1::EnergySource
    )
}

fn is_engine_payoff_role(role: &CardRewardSemanticRoleV1) -> bool {
    matches!(
        role,
        CardRewardSemanticRoleV1::StrengthPayoff
            | CardRewardSemanticRoleV1::BlockPayoff
            | CardRewardSemanticRoleV1::StrikePayoff
            | CardRewardSemanticRoleV1::UpgradePayoff
            | CardRewardSemanticRoleV1::ExhaustPayoff
            | CardRewardSemanticRoleV1::StatusPayoff
            | CardRewardSemanticRoleV1::SelfDamagePayoff
    )
}

fn push_key(keys: &mut Vec<String>, key: &str) {
    if !keys.iter().any(|existing| existing == key) {
        keys.push(key.to_string());
    }
}

#[cfg(test)]
mod tests {
    use crate::ai::card_reward_policy_v1::{CardRewardSemanticProfileV1, CardRewardSemanticRoleV1};
    use crate::content::cards::CardId;

    #[test]
    fn trajectory_distinguishes_transition_frontload_from_engine_shape() {
        let transition = super::summarize_branch_trajectory_v1(&[
            profile("Twin Strike", &[CardRewardSemanticRoleV1::FrontloadDamage]),
            profile("Cleave", &[CardRewardSemanticRoleV1::FrontloadDamage]),
        ]);

        let block_engine = super::summarize_branch_trajectory_v1(&[
            profile("Barricade", &[CardRewardSemanticRoleV1::BlockRetention]),
            profile("Entrench", &[CardRewardSemanticRoleV1::BlockMultiplier]),
            profile("Body Slam", &[CardRewardSemanticRoleV1::BlockPayoff]),
        ]);

        assert_eq!(transition.transition_frontload_picks, 2);
        assert!(transition.package_keys.is_empty());
        assert!(transition.setup_keys.is_empty());
        assert_eq!(block_engine.transition_frontload_picks, 0);
        assert_eq!(block_engine.engine_payoff_picks, 1);
        assert!(block_engine
            .package_keys
            .contains(&"block_engine".to_string()));
        assert!(block_engine
            .setup_keys
            .contains(&"block_engine".to_string()));
    }

    #[test]
    fn trajectory_family_key_buckets_redundant_transition_counts() {
        let two_transition = super::summarize_branch_trajectory_v1(&[
            profile("Twin Strike", &[CardRewardSemanticRoleV1::FrontloadDamage]),
            profile("Cleave", &[CardRewardSemanticRoleV1::FrontloadDamage]),
        ]);
        let three_transition = super::summarize_branch_trajectory_v1(&[
            profile("Twin Strike", &[CardRewardSemanticRoleV1::FrontloadDamage]),
            profile("Cleave", &[CardRewardSemanticRoleV1::FrontloadDamage]),
            profile(
                "Sword Boomerang",
                &[CardRewardSemanticRoleV1::FrontloadDamage],
            ),
        ]);
        let block_engine = super::summarize_branch_trajectory_v1(&[
            profile("Barricade", &[CardRewardSemanticRoleV1::BlockRetention]),
            profile("Body Slam", &[CardRewardSemanticRoleV1::BlockPayoff]),
        ]);

        assert_eq!(
            super::branch_trajectory_family_key_v1(&two_transition),
            super::branch_trajectory_family_key_v1(&three_transition)
        );
        assert_ne!(
            super::branch_trajectory_family_key_v1(&two_transition),
            super::branch_trajectory_family_key_v1(&block_engine)
        );
    }

    #[test]
    fn trajectory_ignores_generic_package_payoff_as_a_package_key() {
        let generic = super::summarize_branch_trajectory_v1(&[profile(
            "Generic Payoff",
            &[CardRewardSemanticRoleV1::PackagePayoff],
        )]);

        assert!(generic.package_keys.is_empty());
        assert_eq!(generic.engine_payoff_picks, 0);
    }

    fn profile(name: &str, roles: &[CardRewardSemanticRoleV1]) -> CardRewardSemanticProfileV1 {
        CardRewardSemanticProfileV1 {
            card: CardId::Strike,
            name: name.to_string(),
            roles: roles.to_vec(),
            dependencies: Vec::new(),
            unsupported_mechanics: Vec::new(),
        }
    }
}
