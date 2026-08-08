//! Versioned handoff from production run checkpoints into combat learning.
//!
//! The envelope deliberately contains only exact run-control checkpoints plus
//! their independently recomputed combat-root identity and compact context.
//! Python callers may carry the opaque bytes but never decode simulator state.

use std::collections::BTreeSet;
use std::io::{self, Cursor, Write};

use serde::{Deserialize, Serialize};

use super::{
    CombatLearningRootContextV1, CombatLearningRootIdentityV1, CombatLearningRootV1,
    RunControlSessionCheckpointV1,
};

pub const COMBAT_LEARNING_ROOT_ARTIFACT_MAGIC: &[u8] = b"STS-COMBAT-LEARNING-ROOTS\0";
pub const COMBAT_LEARNING_ROOT_ARTIFACT_FORMAT_VERSION: u32 = 1;

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CombatLearningRootArtifactV1 {
    identity: CombatLearningRootIdentityV1,
    context: CombatLearningRootContextV1,
    session: RunControlSessionCheckpointV1,
}

impl CombatLearningRootArtifactV1 {
    pub fn identity(&self) -> &CombatLearningRootIdentityV1 {
        &self.identity
    }

    pub fn context(&self) -> &CombatLearningRootContextV1 {
        &self.context
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CombatLearningRootBatchArtifactV1 {
    roots: Vec<CombatLearningRootArtifactV1>,
}

impl CombatLearningRootBatchArtifactV1 {
    pub fn from_checkpoints(
        checkpoints: impl IntoIterator<Item = RunControlSessionCheckpointV1>,
    ) -> Result<Self, String> {
        let roots = checkpoints
            .into_iter()
            .map(|session| {
                let root = CombatLearningRootV1::from_checkpoint(session.clone())?;
                Ok(CombatLearningRootArtifactV1 {
                    identity: root.identity().clone(),
                    context: *root.context(),
                    session,
                })
            })
            .collect::<Result<Vec<_>, String>>()?;
        let artifact = Self { roots };
        artifact.validate(None)?;
        Ok(artifact)
    }

    pub fn roots(&self) -> &[CombatLearningRootArtifactV1] {
        &self.roots
    }

    pub fn encode(&self, max_bytes: usize) -> Result<Vec<u8>, String> {
        self.validate(None)?;
        encode_artifact(self, max_bytes)
    }

    pub fn decode(
        payload: &[u8],
        expected_root_count: usize,
        max_bytes: usize,
    ) -> Result<Self, String> {
        if expected_root_count == 0 {
            return Err("combat learning root artifact expected count must be positive".to_owned());
        }
        if payload.len() > max_bytes {
            return Err(
                "combat learning root artifact exceeds its caller-provided byte limit".to_owned(),
            );
        }
        let header_bytes = artifact_header_bytes();
        if payload.len() < header_bytes {
            return Err("combat learning root artifact ended before its header".to_owned());
        }
        if &payload[..COMBAT_LEARNING_ROOT_ARTIFACT_MAGIC.len()]
            != COMBAT_LEARNING_ROOT_ARTIFACT_MAGIC
        {
            return Err("combat learning root artifact magic is invalid".to_owned());
        }
        let version_start = COMBAT_LEARNING_ROOT_ARTIFACT_MAGIC.len();
        let encoded_version = u32::from_be_bytes(
            payload[version_start..header_bytes]
                .try_into()
                .map_err(|_| "combat learning root artifact version is truncated")?,
        );
        if encoded_version != COMBAT_LEARNING_ROOT_ARTIFACT_FORMAT_VERSION {
            return Err("combat learning root artifact format version is unsupported".to_owned());
        }

        let mut decoder = rmp_serde::Deserializer::new(Cursor::new(&payload[header_bytes..]));
        let artifact = Self::deserialize(&mut decoder)
            .map_err(|error| format!("cannot decode combat learning root artifact: {error}"))?;
        if usize::try_from(decoder.position()).ok() != Some(payload.len() - header_bytes) {
            return Err("combat learning root artifact contains trailing bytes".to_owned());
        }
        artifact.validate(Some(expected_root_count))?;
        if encode_artifact(&artifact, max_bytes)? != payload {
            return Err("combat learning root artifact encoding is not canonical".to_owned());
        }
        Ok(artifact)
    }

    pub fn into_checkpoints(self) -> Result<Vec<RunControlSessionCheckpointV1>, String> {
        self.validate(None)?;
        Ok(self.roots.into_iter().map(|root| root.session).collect())
    }

    fn validate(&self, expected_root_count: Option<usize>) -> Result<(), String> {
        if self.roots.is_empty() {
            return Err("combat learning root artifact must contain at least one root".to_owned());
        }
        if let Some(expected) = expected_root_count {
            if self.roots.len() != expected {
                return Err(format!(
                    "combat learning root artifact contains {} roots, expected {expected}",
                    self.roots.len()
                ));
            }
        }

        let mut identities = BTreeSet::new();
        for (index, captured) in self.roots.iter().enumerate() {
            let recomputed = CombatLearningRootV1::from_checkpoint(captured.session.clone())
                .map_err(|error| {
                    format!("combat learning root artifact root {index} is invalid: {error}")
                })?;
            if recomputed.identity() != captured.identity() {
                return Err(format!(
                    "combat learning root artifact root {index} identity does not match its session"
                ));
            }
            if recomputed.context() != captured.context() {
                return Err(format!(
                    "combat learning root artifact root {index} context does not match its session"
                ));
            }
            if !identities.insert(captured.identity.root_id.as_str()) {
                return Err(format!(
                    "combat learning root artifact repeats exact root {}",
                    captured.identity.root_id
                ));
            }
        }
        Ok(())
    }
}

struct BoundedArtifactWriter {
    bytes: Vec<u8>,
    max_bytes: usize,
}

impl BoundedArtifactWriter {
    fn new(max_bytes: usize) -> Result<Self, String> {
        let header_bytes = artifact_header_bytes();
        if max_bytes < header_bytes {
            return Err(
                "combat learning root artifact byte limit is smaller than its header".to_owned(),
            );
        }
        let mut bytes = Vec::with_capacity(max_bytes.min(64 * 1024));
        bytes.extend_from_slice(COMBAT_LEARNING_ROOT_ARTIFACT_MAGIC);
        bytes.extend_from_slice(&COMBAT_LEARNING_ROOT_ARTIFACT_FORMAT_VERSION.to_be_bytes());
        Ok(Self { bytes, max_bytes })
    }

    fn finish(self) -> Vec<u8> {
        self.bytes
    }
}

impl Write for BoundedArtifactWriter {
    fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
        let next =
            self.bytes.len().checked_add(buffer.len()).ok_or_else(|| {
                io::Error::other("combat learning root artifact byte count overflow")
            })?;
        if next > self.max_bytes {
            return Err(io::Error::other(
                "combat learning root artifact exceeds its caller-provided byte limit",
            ));
        }
        self.bytes.extend_from_slice(buffer);
        Ok(buffer.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

fn artifact_header_bytes() -> usize {
    COMBAT_LEARNING_ROOT_ARTIFACT_MAGIC.len() + std::mem::size_of::<u32>()
}

fn encode_artifact(
    artifact: &CombatLearningRootBatchArtifactV1,
    max_bytes: usize,
) -> Result<Vec<u8>, String> {
    let mut writer = BoundedArtifactWriter::new(max_bytes)?;
    rmp_serde::encode::write_named(&mut writer, artifact)
        .map_err(|error| format!("cannot encode combat learning root artifact: {error}"))?;
    Ok(writer.finish())
}

#[cfg(test)]
mod tests {
    use crate::content::cards::CardId;
    use crate::content::monsters::exordium::jaw_worm::JawWorm;
    use crate::content::monsters::{EnemyId, MonsterBehavior};
    use crate::runtime::combat::CombatCard;
    use crate::state::core::{ActiveCombat, CombatContext, EngineState, RoomCombatContext};
    use crate::state::map::node::RoomType;
    use crate::test_support::{blank_test_combat, test_monster};

    use super::*;
    use crate::eval::run_control::{LearningBoundaryV1, LearningEnvV1, RunControlSession};

    #[test]
    fn artifact_round_trip_recomputes_exact_combat_roots() {
        let first = combat_root_checkpoint(20);
        let second = combat_root_checkpoint(21);
        let artifact =
            CombatLearningRootBatchArtifactV1::from_checkpoints([first.clone(), second.clone()])
                .expect("capture exact roots");
        let payload = artifact.encode(1024 * 1024).expect("encode roots");
        let restored = CombatLearningRootBatchArtifactV1::decode(&payload, 2, 1024 * 1024)
            .expect("decode roots");

        assert_eq!(restored.roots().len(), 2);
        for checkpoint in restored.into_checkpoints().expect("validated checkpoints") {
            assert!(matches!(
                LearningEnvV1::from_checkpoint(checkpoint)
                    .expect("restore learning environment")
                    .observe()
                    .expect("observe restored combat"),
                LearningBoundaryV1::Combat { .. }
            ));
        }
        assert!(CombatLearningRootBatchArtifactV1::decode(&payload, 1, 1024 * 1024,).is_err());
        assert!(CombatLearningRootBatchArtifactV1::decode(&payload, 2, 16).is_err());
    }

    #[test]
    fn artifact_rejects_non_combat_duplicate_and_tampered_roots() {
        let non_combat = RunControlSessionCheckpointV1::from_session(&RunControlSession::new(
            Default::default(),
        ));
        assert!(CombatLearningRootBatchArtifactV1::from_checkpoints([non_combat]).is_err());

        let root = combat_root_checkpoint(20);
        assert!(
            CombatLearningRootBatchArtifactV1::from_checkpoints([root.clone(), root.clone(),])
                .is_err()
        );

        let mut artifact =
            CombatLearningRootBatchArtifactV1::from_checkpoints([root]).expect("capture root");
        artifact.roots[0].context.hp -= 1;
        assert!(artifact.encode(1024 * 1024).is_err());
    }

    fn combat_root_checkpoint(monster_hp: i32) -> RunControlSessionCheckpointV1 {
        let mut session = RunControlSession::new(Default::default());
        let mut combat = blank_test_combat();
        combat.zones.hand = vec![CombatCard::new(CardId::Strike, 51)];
        combat.entities.power_db.insert(11, Vec::new());
        combat.entities.power_db.insert(7, Vec::new());
        combat
            .runtime
            .monster_protocol
            .insert(11, Default::default());
        combat
            .runtime
            .monster_protocol
            .insert(7, Default::default());
        let mut monster = test_monster(EnemyId::JawWorm);
        monster.id = 7;
        monster.current_hp = monster_hp;
        monster.max_hp = monster_hp;
        monster.set_planned_move_id(1);
        let plan = JawWorm::turn_plan(&combat, &monster);
        monster.set_planned_steps(plan.steps);
        monster.set_planned_visible_spec(plan.visible_spec);
        combat.entities.monsters.push(monster);
        session.engine_state = EngineState::CombatPlayerTurn;
        session.active_combat = Some(ActiveCombat::new(
            EngineState::CombatPlayerTurn,
            combat,
            CombatContext::Room(RoomCombatContext {
                room_type: RoomType::MonsterRoom,
            }),
        ));
        RunControlSessionCheckpointV1::from_session(&session)
    }
}
