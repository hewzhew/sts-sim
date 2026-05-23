use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::ai::combat_state_key::{
    combat_exact_state_key, stable_dominance_bucket_key, stable_outcome_key,
};
use crate::content::cards::java_id;
use crate::content::monsters::EnemyId;
use crate::eval::artifact::{
    ArtifactHeaderV1, ArtifactProvenanceV1, ArtifactTrustLevel, ARTIFACT_PRODUCER,
};
use crate::eval::fingerprint::{
    combat_legal_action_set_fingerprint_v1, combat_state_fingerprint_v1,
    CombatActionFingerprintDescriptorV1, StateFingerprintV1, FINGERPRINT_ALGORITHM_DEBUG,
    FINGERPRINT_ALGORITHM_JSON,
};
use crate::runtime::combat::Intent;
use crate::sim::combat::{combat_terminal, stable_boundary, CombatPosition, CombatTerminal};
use crate::sim::combat_identity::validate_combat_card_identity_for_capture;
use crate::state::core::EngineState;
use crate::state::run::RunState;

pub const COMBAT_CAPTURE_SCHEMA_NAME: &str = "CombatCaptureV1";
pub const COMBAT_CAPTURE_SCHEMA_VERSION: u32 = 1;

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CombatCaptureV1 {
    pub schema_name: String,
    pub schema_version: u32,
    #[serde(default = "default_combat_capture_header")]
    pub header: ArtifactHeaderV1,
    pub capture_kind: CombatCaptureKind,
    #[serde(default = "default_combat_capture_trust_level")]
    pub trust_level: ArtifactTrustLevel,
    pub information_boundary: String,
    pub label: Option<String>,
    #[serde(default = "default_combat_capture_provenance")]
    pub provenance: ArtifactProvenanceV1,
    pub source: CombatCaptureSourceV1,
    pub integrity: CombatCaptureIntegrityV1,
    #[serde(default)]
    pub fingerprints: Option<StateFingerprintV1>,
    #[serde(default)]
    pub legal_actions: Option<CombatCaptureLegalActionsV1>,
    pub summary: CombatCaptureSummaryV1,
    pub position: CombatPosition,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CombatCaptureKind {
    CombatPosition,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CombatCaptureSourceV1 {
    pub producer: String,
    pub capture_method: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CombatCaptureIntegrityV1 {
    pub fingerprint_algorithm: String,
    pub exact_state_fingerprint: String,
    pub stable_outcome_fingerprint: Option<String>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CombatCaptureLegalActionsV1 {
    pub fingerprint_algorithm: String,
    pub count: usize,
    pub candidate_set_hash: String,
    pub candidate_order_hash: String,
    pub descriptors: Vec<CombatActionFingerprintDescriptorV1>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CombatCaptureSummaryV1 {
    pub engine_state: String,
    pub terminal: CombatTerminal,
    pub stable_boundary: bool,
    pub player_class: String,
    pub ascension_level: u8,
    pub player_hp: i32,
    pub player_max_hp: i32,
    pub player_block: i32,
    pub energy: u8,
    pub turn_count: u32,
    pub hand: Vec<CombatCaptureCardSummaryV1>,
    pub draw_count: usize,
    pub discard_count: usize,
    pub exhaust_count: usize,
    pub limbo_count: usize,
    pub queued_cards_count: usize,
    pub potions: Vec<Option<CombatCapturePotionSummaryV1>>,
    pub monsters: Vec<CombatCaptureMonsterSummaryV1>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CombatCaptureCardSummaryV1 {
    pub uuid: u32,
    pub card_id: String,
    pub upgrades: u8,
    pub cost_for_turn: i32,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CombatCapturePotionSummaryV1 {
    pub uuid: u32,
    pub potion_id: String,
    pub can_use: bool,
    pub can_discard: bool,
    pub requires_target: bool,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CombatCaptureMonsterSummaryV1 {
    pub slot: u8,
    pub entity_id: usize,
    pub enemy_id: String,
    pub hp: i32,
    pub max_hp: i32,
    pub block: i32,
    pub alive: bool,
    pub escaped: bool,
    pub dying: bool,
    pub half_dead: bool,
    pub planned_move_id: u8,
    pub visible_intent: String,
    pub preview_damage_per_hit: i32,
}

impl CombatCaptureV1 {
    pub fn position(&self) -> &CombatPosition {
        &self.position
    }
}

pub fn capture_combat_position_v1(
    label: Option<String>,
    position: &CombatPosition,
) -> Result<CombatCaptureV1, String> {
    capture_combat_position_with_provenance_v1(
        label,
        position,
        ArtifactProvenanceV1::exact_combat_position(),
    )
}

pub fn capture_combat_position_from_run_v1(
    label: Option<String>,
    position: &CombatPosition,
    run_state: &RunState,
) -> Result<CombatCaptureV1, String> {
    capture_combat_position_with_provenance_v1(
        label,
        position,
        ArtifactProvenanceV1::manual_run_control(run_state),
    )
}

pub fn capture_combat_position_with_provenance_v1(
    label: Option<String>,
    position: &CombatPosition,
    provenance: ArtifactProvenanceV1,
) -> Result<CombatCaptureV1, String> {
    if !active_combat_capture_boundary(&position.engine, &position.combat) {
        return Err(
            "CombatCaptureV1 requires an active stable combat decision boundary".to_string(),
        );
    }
    validate_combat_card_identity_for_capture(&position.combat)?;

    let integrity = integrity_for_position(&position);
    let fingerprints = combat_state_fingerprint_v1(position);
    let legal_actions = legal_actions_for_position(position);
    let summary = summary_for_position(&position);
    Ok(CombatCaptureV1 {
        schema_name: COMBAT_CAPTURE_SCHEMA_NAME.to_string(),
        schema_version: COMBAT_CAPTURE_SCHEMA_VERSION,
        header: default_combat_capture_header(),
        capture_kind: CombatCaptureKind::CombatPosition,
        trust_level: ArtifactTrustLevel::Restorable,
        information_boundary: "engine_truth_exact_combat_position".to_string(),
        label,
        provenance,
        source: CombatCaptureSourceV1 {
            producer: ARTIFACT_PRODUCER.to_string(),
            capture_method: "exact_combat_position".to_string(),
        },
        integrity,
        fingerprints: Some(fingerprints),
        legal_actions: Some(legal_actions),
        summary,
        position: position.clone(),
    })
}

pub fn load_combat_capture_v1(path: &Path) -> Result<CombatCaptureV1, String> {
    let payload = fs::read_to_string(path).map_err(|err| err.to_string())?;
    let mut capture: CombatCaptureV1 =
        serde_json::from_str(&payload).map_err(|err| err.to_string())?;
    repair_combat_capture_lineage_v1(&mut capture);
    validate_combat_capture_v1(&capture)?;
    Ok(capture)
}

pub fn save_combat_capture_v1(path: &Path, capture: &CombatCaptureV1) -> Result<(), String> {
    validate_combat_capture_v1(capture)?;
    if let Some(parent) = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        fs::create_dir_all(parent).map_err(|err| err.to_string())?;
    }
    let payload = serde_json::to_string_pretty(capture).map_err(|err| err.to_string())?;
    fs::write(path, payload).map_err(|err| err.to_string())
}

pub fn validate_combat_capture_v1(capture: &CombatCaptureV1) -> Result<(), String> {
    if capture.schema_name != COMBAT_CAPTURE_SCHEMA_NAME {
        return Err(format!(
            "unsupported combat capture schema '{}'",
            capture.schema_name
        ));
    }
    if capture.schema_version != COMBAT_CAPTURE_SCHEMA_VERSION {
        return Err(format!(
            "unsupported combat capture schema_version {}",
            capture.schema_version
        ));
    }
    if capture.capture_kind != CombatCaptureKind::CombatPosition {
        return Err("unsupported combat capture kind".to_string());
    }
    if capture.header != default_combat_capture_header() {
        return Err("combat capture artifact header does not match schema".to_string());
    }
    validate_capture_provenance(&capture.provenance)?;
    if !active_combat_capture_boundary(&capture.position.engine, &capture.position.combat) {
        return Err(
            "combat capture position is not an active stable combat decision boundary".to_string(),
        );
    }
    validate_combat_card_identity_for_capture(&capture.position.combat)?;

    let expected = integrity_for_position(&capture.position);
    if capture.integrity != expected {
        return Err("combat capture integrity fingerprints do not match position".to_string());
    }
    let expected_fingerprints = combat_state_fingerprint_v1(&capture.position);
    if capture.fingerprints.as_ref() != Some(&expected_fingerprints) {
        return Err("combat capture state fingerprints do not match position".to_string());
    }
    let expected_legal_actions = legal_actions_for_position(&capture.position);
    if capture.legal_actions.as_ref() != Some(&expected_legal_actions) {
        return Err("combat capture legal action fingerprints do not match position".to_string());
    }
    if capture.summary != summary_for_position(&capture.position) {
        return Err("combat capture summary does not match position".to_string());
    }
    Ok(())
}

fn integrity_for_position(position: &CombatPosition) -> CombatCaptureIntegrityV1 {
    let exact = combat_exact_state_key(&position.engine, &position.combat);
    let stable = stable_dominance_bucket_key(&position.engine, &position.combat)
        .map(|_| stable_outcome_key(&position.engine, &position.combat));
    CombatCaptureIntegrityV1 {
        fingerprint_algorithm: FINGERPRINT_ALGORITHM_DEBUG.to_string(),
        exact_state_fingerprint: fingerprint_debug(&exact),
        stable_outcome_fingerprint: stable.as_ref().map(fingerprint_debug),
    }
}

fn legal_actions_for_position(position: &CombatPosition) -> CombatCaptureLegalActionsV1 {
    let legal = combat_legal_action_set_fingerprint_v1(&position.engine, &position.combat);
    CombatCaptureLegalActionsV1 {
        fingerprint_algorithm: FINGERPRINT_ALGORITHM_JSON.to_string(),
        count: legal.count,
        candidate_set_hash: legal.candidate_set_hash,
        candidate_order_hash: legal.candidate_order_hash,
        descriptors: legal.descriptors,
    }
}

fn repair_combat_capture_lineage_v1(capture: &mut CombatCaptureV1) {
    if capture.fingerprints.is_none() {
        capture.fingerprints = Some(combat_state_fingerprint_v1(&capture.position));
    }
    if capture.legal_actions.is_none() {
        capture.legal_actions = Some(legal_actions_for_position(&capture.position));
    }
}

fn validate_capture_provenance(provenance: &ArtifactProvenanceV1) -> Result<(), String> {
    if provenance.producer.trim().is_empty() {
        return Err("combat capture provenance producer cannot be empty".to_string());
    }
    if provenance.capture_method.trim().is_empty() {
        return Err("combat capture provenance capture_method cannot be empty".to_string());
    }
    if provenance.trainable_as_action_label || provenance.policy_quality_claim {
        return Err(
            "combat capture provenance must not claim teacher-label or policy-quality authority"
                .to_string(),
        );
    }
    Ok(())
}

fn active_combat_capture_boundary(
    engine: &EngineState,
    combat: &crate::runtime::combat::CombatState,
) -> bool {
    stable_boundary(engine, combat)
        && matches!(
            engine,
            EngineState::CombatPlayerTurn | EngineState::PendingChoice(_)
        )
}

fn summary_for_position(position: &CombatPosition) -> CombatCaptureSummaryV1 {
    let combat = &position.combat;
    CombatCaptureSummaryV1 {
        engine_state: format!("{:?}", position.engine),
        terminal: combat_terminal(&position.engine, combat),
        stable_boundary: stable_boundary(&position.engine, combat),
        player_class: combat.meta.player_class.clone(),
        ascension_level: combat.meta.ascension_level,
        player_hp: combat.entities.player.current_hp,
        player_max_hp: combat.entities.player.max_hp,
        player_block: combat.entities.player.block,
        energy: combat.turn.energy,
        turn_count: combat.turn.turn_count,
        hand: combat
            .zones
            .hand
            .iter()
            .map(|card| CombatCaptureCardSummaryV1 {
                uuid: card.uuid,
                card_id: java_id(card.id).to_string(),
                upgrades: card.upgrades,
                cost_for_turn: card.cost_for_turn_java(),
            })
            .collect(),
        draw_count: combat.zones.draw_pile.len(),
        discard_count: combat.zones.discard_pile.len(),
        exhaust_count: combat.zones.exhaust_pile.len(),
        limbo_count: combat.zones.limbo.len(),
        queued_cards_count: combat.zones.queued_cards.len(),
        potions: combat
            .entities
            .potions
            .iter()
            .map(|slot| {
                slot.as_ref().map(|potion| CombatCapturePotionSummaryV1 {
                    uuid: potion.uuid,
                    potion_id: format!("{:?}", potion.id),
                    can_use: potion.can_use,
                    can_discard: potion.can_discard,
                    requires_target: potion.requires_target,
                })
            })
            .collect(),
        monsters: combat
            .entities
            .monsters
            .iter()
            .map(|monster| {
                let observation = combat
                    .runtime
                    .monster_protocol
                    .get(&monster.id)
                    .map(|protocol| &protocol.observation);
                let turn_plan = monster.turn_plan();
                CombatCaptureMonsterSummaryV1 {
                    slot: monster.slot,
                    entity_id: monster.id,
                    enemy_id: EnemyId::from_id(monster.monster_type)
                        .map(|enemy| format!("{enemy:?}"))
                        .unwrap_or_else(|| format!("monster_type:{}", monster.monster_type)),
                    hp: monster.current_hp,
                    max_hp: monster.max_hp,
                    block: monster.block,
                    alive: monster.is_alive_for_action(),
                    escaped: monster.is_escaped,
                    dying: monster.is_dying,
                    half_dead: monster.half_dead,
                    planned_move_id: monster.planned_move_id(),
                    visible_intent: observation
                        .filter(|obs| obs.visible_intent != Intent::Unknown)
                        .map(|obs| format!("{:?}", obs.visible_intent))
                        .unwrap_or_else(|| format!("{:?}", turn_plan.summary_spec())),
                    preview_damage_per_hit: observation
                        .filter(|obs| obs.preview_damage_per_hit > 0)
                        .map(|obs| obs.preview_damage_per_hit)
                        .or_else(|| turn_plan.attack().map(|attack| attack.base_damage))
                        .unwrap_or(0),
                }
            })
            .collect(),
    }
}

fn fingerprint_debug<T: std::fmt::Debug>(value: &T) -> String {
    crate::eval::fingerprint::hash_debug(value)
}

fn default_combat_capture_header() -> ArtifactHeaderV1 {
    ArtifactHeaderV1::new(
        COMBAT_CAPTURE_SCHEMA_NAME,
        COMBAT_CAPTURE_SCHEMA_VERSION,
        "combat_capture",
    )
}

fn default_combat_capture_trust_level() -> ArtifactTrustLevel {
    ArtifactTrustLevel::Restorable
}

fn default_combat_capture_provenance() -> ArtifactProvenanceV1 {
    ArtifactProvenanceV1::unknown()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::eval::artifact::{ArtifactSourceKind, ArtifactTrustLevel};
    use crate::fixtures::combat_start_spec::{compile_combat_start_spec, CombatStartSpec};
    use crate::sim::combat::CombatPosition;

    #[test]
    fn combat_capture_roundtrips_exact_position() {
        let position = jaw_worm_position();
        let capture = capture_combat_position_v1(Some("jaw_worm_capture".to_string()), &position)
            .expect("stable combat start should capture");

        let payload = serde_json::to_string_pretty(&capture).expect("capture should serialize");
        let loaded: CombatCaptureV1 =
            serde_json::from_str(&payload).expect("capture should deserialize");

        validate_combat_capture_v1(&loaded).expect("loaded capture should validate");
        assert_eq!(loaded.position, position);
        assert_eq!(loaded.header, default_combat_capture_header());
        assert_eq!(loaded.trust_level, ArtifactTrustLevel::Restorable);
        assert_eq!(
            loaded.provenance.source_kind,
            ArtifactSourceKind::ExactCombatPosition
        );
        assert_eq!(loaded.integrity, capture.integrity);
        assert_eq!(loaded.fingerprints, capture.fingerprints);
        assert_eq!(loaded.legal_actions, capture.legal_actions);
        assert!(loaded
            .legal_actions
            .as_ref()
            .is_some_and(|actions| actions.count > 0 && !actions.descriptors.is_empty()));
        assert_eq!(loaded.summary, capture.summary);
    }

    #[test]
    fn combat_capture_validation_rejects_tampered_summary() {
        let position = jaw_worm_position();
        let mut capture = capture_combat_position_v1(None, &position).expect("capture should work");
        capture.summary.player_hp -= 1;

        let err =
            validate_combat_capture_v1(&capture).expect_err("tampered summary should be rejected");

        assert!(err.contains("summary"));
    }

    #[test]
    fn combat_capture_validation_rejects_tampered_fingerprint() {
        let position = jaw_worm_position();
        let mut capture = capture_combat_position_v1(None, &position).expect("capture should work");
        capture.integrity.exact_state_fingerprint =
            "0000000000000000000000000000000000000000000000000000000000000000".to_string();

        let err = validate_combat_capture_v1(&capture)
            .expect_err("tampered fingerprint should be rejected");

        assert!(err.contains("fingerprints"));
    }

    #[test]
    fn combat_capture_validation_rejects_tampered_state_fingerprint() {
        let position = jaw_worm_position();
        let mut capture = capture_combat_position_v1(None, &position).expect("capture should work");
        capture
            .fingerprints
            .as_mut()
            .expect("capture should have fingerprints")
            .public_observation_hash =
            "0000000000000000000000000000000000000000000000000000000000000000".to_string();

        let err = validate_combat_capture_v1(&capture)
            .expect_err("tampered state fingerprint should be rejected");

        assert!(err.contains("state fingerprints"));
    }

    #[test]
    fn combat_capture_validation_rejects_card_uuid_identity_conflict() {
        let mut position = jaw_worm_position();
        let uuid = position.combat.zones.hand[0].uuid;
        position
            .combat
            .zones
            .discard_pile
            .push(crate::runtime::combat::CombatCard::new(
                crate::content::cards::CardId::Slimed,
                uuid,
            ));

        let err = capture_combat_position_v1(None, &position)
            .expect_err("capture should reject same uuid mapping to different card ids");

        assert!(err.contains("card identity conflict"));
        assert!(err.contains("active uuid"));
    }

    #[test]
    fn combat_capture_validation_rejects_stale_card_uuid_counter() {
        let mut position = jaw_worm_position();
        let max_uuid = position
            .combat
            .meta
            .master_deck_snapshot
            .iter()
            .map(|card| card.uuid)
            .max()
            .expect("test deck should contain cards");
        position.combat.zones.card_uuid_counter = max_uuid - 1;

        let err = capture_combat_position_v1(None, &position)
            .expect_err("capture should reject stale future card uuid counter");

        assert!(err.contains("card_uuid_counter"));
        assert!(err.contains("not fresh"));
    }

    #[test]
    fn combat_capture_rejects_postcombat_engine_boundaries() {
        let mut position = jaw_worm_position();
        position.engine = EngineState::RewardScreen(crate::state::rewards::RewardState::new());

        let err = capture_combat_position_v1(None, &position)
            .expect_err("postcombat boundary should not be a search start capture");

        assert!(err.contains("active stable combat decision boundary"));
    }

    #[test]
    fn combat_capture_rejects_combat_start_request() {
        let mut position = jaw_worm_position();
        position.engine = EngineState::CombatStart(crate::state::core::CombatStartRequest::event(
            crate::content::monsters::factory::EncounterId::JawWorm,
            crate::state::rewards::RewardState::new(),
            true,
            false,
            false,
            crate::state::core::PostCombatReturn::MapNavigation,
        ));

        let err = capture_combat_position_v1(None, &position)
            .expect_err("CombatStart should not be a search start capture");

        assert!(err.contains("active stable combat decision boundary"));
    }

    fn jaw_worm_position() -> CombatPosition {
        let spec: CombatStartSpec = serde_json::from_str(
            r#"{
                "name": "jaw_worm_starter",
                "player_class": "Ironclad",
                "ascension_level": 0,
                "encounter_id": "JawWorm",
                "room_type": "monster",
                "seed": 1,
                "player_current_hp": 80,
                "player_max_hp": 80,
                "master_deck": [
                    {"id": "Strike_R", "count": 5},
                    {"id": "Defend_R", "count": 4},
                    "Bash"
                ]
            }"#,
        )
        .expect("test start spec should parse");
        let (engine, combat) =
            compile_combat_start_spec(&spec).expect("test start spec should compile");
        CombatPosition::new(engine, combat)
    }
}
