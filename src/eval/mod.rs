pub mod artifact;
#[cfg(feature = "control-full")]
pub mod campfire_evaluation;
#[cfg(feature = "control-full")]
pub mod campfire_projection;
#[cfg(feature = "control-full")]
pub mod campfire_survival_scenarios;
#[cfg(feature = "control-full")]
pub mod campfire_threat_panel;
pub mod card_reward_value_loop;
pub mod combat_action_imitation;
pub mod combat_capture;
pub mod combat_case;
pub mod combat_lab_v1;
pub mod combat_search_v2;
pub mod combat_state_features;
pub(crate) mod event_boundary_classifier_v1;
#[cfg(feature = "control-full")]
pub mod event_boundary_packet_v1;
pub mod fingerprint;
#[cfg(feature = "control-full")]
pub mod reward_boundary_packet_v1;
#[cfg(feature = "control-full")]
pub mod reward_semantic_live_sample_v1;
pub mod run_control;

pub(crate) fn repository_root() -> &'static std::path::Path {
    std::path::Path::new(env!("STS_REPOSITORY_ROOT"))
}
