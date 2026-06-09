use std::fs;
use std::path::PathBuf;

use crate::content::monsters::EnemyId;
use crate::state::core::EngineState;

use super::artifact_commands::default_benchmark_root;
use super::registry::{add_case_to_benchmark_registry, BenchmarkCasePaths};
use super::RunControlSession;

#[derive(Clone, Debug, Default, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct AutoCombatCaptureConfig {
    pub enabled: bool,
    pub root: Option<PathBuf>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AutoCombatCaptureResult {
    pub case_id: String,
    pub capture_path: PathBuf,
    pub benchmark_manifest: PathBuf,
}

pub(super) fn maybe_auto_capture_combat_start(
    session: &mut RunControlSession,
) -> Result<Option<AutoCombatCaptureResult>, String> {
    if !session.auto_capture.enabled {
        return Ok(None);
    }
    if session.auto_capture_last_combat_sequence == Some(session.combat_sequence) {
        return Ok(None);
    }
    if !is_auto_capture_boundary(session) {
        return Ok(None);
    }

    let root = session
        .auto_capture
        .root
        .clone()
        .unwrap_or_else(|| default_benchmark_root(session));
    let case_id = next_available_case_id(&root, &base_case_id(session));
    let paths = BenchmarkCasePaths::for_case(&root, &case_id);
    session.save_current_auto_combat_capture(&paths.capture_path, Some(case_id.clone()))?;
    let paths = add_case_to_benchmark_registry(&root, &case_id)?;
    session.remember_capture_case(root, case_id.clone());
    session.auto_capture_last_combat_sequence = Some(session.combat_sequence);

    Ok(Some(AutoCombatCaptureResult {
        case_id,
        capture_path: paths.capture_path,
        benchmark_manifest: paths.benchmark_manifest,
    }))
}

pub(super) fn render_auto_capture_result(result: &AutoCombatCaptureResult) -> String {
    format!(
        "Auto-captured combat case `{}` to {} and registered {}.\nAfter this combat ends, type `baseline` only if you played it manually.",
        result.case_id,
        result.capture_path.display(),
        result.benchmark_manifest.display()
    )
}

fn is_auto_capture_boundary(session: &RunControlSession) -> bool {
    let Some(active) = session.active_combat.as_ref() else {
        return false;
    };
    if !matches!(active.engine_state, EngineState::CombatPlayerTurn) {
        return false;
    }
    active.combat_state.turn.turn_count == 0
        && active
            .combat_state
            .turn
            .counters
            .card_ids_played_this_combat
            .is_empty()
        && active.combat_state.turn.counters.cards_played_this_turn == 0
}

fn next_available_case_id(root: &PathBuf, base: &str) -> String {
    let mut case_id = base.to_string();
    let mut suffix = 2u32;
    while capture_exists(root, &case_id) {
        case_id = format!("{base}_{suffix}");
        suffix = suffix.saturating_add(1);
    }
    case_id
}

fn capture_exists(root: &PathBuf, case_id: &str) -> bool {
    fs::metadata(BenchmarkCasePaths::for_case(root, case_id).capture_path).is_ok()
}

fn base_case_id(session: &RunControlSession) -> String {
    let enemy_slug = session
        .active_combat
        .as_ref()
        .map(|active| {
            active
                .combat_state
                .entities
                .monsters
                .iter()
                .filter(|monster| monster.is_alive_for_action())
                .take(3)
                .map(|monster| {
                    EnemyId::from_id(monster.monster_type)
                        .map(|enemy| format!("{enemy:?}"))
                        .unwrap_or_else(|| format!("monster{}", monster.monster_type))
                })
                .map(|label| slug(&label))
                .collect::<Vec<_>>()
                .join("_")
        })
        .filter(|slug| !slug.is_empty())
        .unwrap_or_else(|| "combat".to_string());

    format!(
        "act{}_floor{:02}_combat{:02}_{}",
        session.run_state.act_num, session.run_state.floor_num, session.combat_sequence, enemy_slug
    )
}

fn slug(raw: &str) -> String {
    let mut out = String::new();
    let mut last_was_sep = false;
    for ch in raw.chars().flat_map(char::to_lowercase) {
        if ch.is_ascii_alphanumeric() {
            out.push(ch);
            last_was_sep = false;
        } else if !last_was_sep {
            out.push('_');
            last_was_sep = true;
        }
    }
    out.trim_matches('_').to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::monsters::factory::EncounterId;
    use crate::eval::artifact::ArtifactSourceKind;
    use crate::eval::combat_capture::load_combat_capture_v1;
    use crate::eval::run_control::session::{RunControlConfig, RunControlSession};
    use crate::state::core::ClientInput;
    use crate::state::map::node::{MapEdge, MapRoomNode, RoomType};
    use crate::state::map::state::MapState;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn auto_capture_saves_first_combat_boundary_once() {
        let root = unique_temp_dir("run_control_auto_capture");
        let mut session = test_session_with_first_monster_room();
        session.auto_capture = AutoCombatCaptureConfig {
            enabled: true,
            root: Some(root.clone()),
        };

        let outcome = session
            .apply_command(crate::eval::run_control::RunControlCommand::Input(
                ClientInput::SelectMapNode(0),
            ))
            .expect("map input should enter combat and auto-capture");

        assert!(outcome.message.contains("Auto-captured combat case"));
        let captures = capture_files(&root);
        assert_eq!(captures.len(), 1);
        assert!(BenchmarkCasePaths::for_case(
            &root,
            session
                .last_capture_case()
                .expect("auto capture should remember case")
                .case_id
                .as_str()
        )
        .benchmark_manifest
        .exists());
        let capture = load_combat_capture_v1(&captures[0]).expect("auto capture should load");
        assert_eq!(
            capture.provenance.source_kind,
            ArtifactSourceKind::AutoRunControl
        );
        assert_eq!(
            capture.provenance.capture_method,
            "run_control_auto_capture"
        );
        assert_eq!(capture.source.capture_method, "run_control_auto_capture");

        let second =
            maybe_auto_capture_combat_start(&mut session).expect("same combat should not fail");
        assert!(second.is_none());
        assert_eq!(capture_files(&root).len(), 1);

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn auto_capture_is_disabled_by_default() {
        let mut session = RunControlSession::new(RunControlConfig::default());
        let result = maybe_auto_capture_combat_start(&mut session).expect("disabled auto capture");

        assert!(result.is_none());
    }

    fn capture_files(root: &std::path::Path) -> Vec<PathBuf> {
        let dir = root.join("captures");
        let Ok(entries) = fs::read_dir(dir) else {
            return Vec::new();
        };
        entries
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|path| path.extension().and_then(|ext| ext.to_str()) == Some("json"))
            .collect()
    }

    fn test_session_with_first_monster_room() -> RunControlSession {
        let mut session = RunControlSession::new(RunControlConfig::default());
        session.run_state.event_state = None;
        session.engine_state = EngineState::MapNavigation;
        let mut first = MapRoomNode::new(0, 0);
        first.class = Some(RoomType::MonsterRoom);
        first.edges.insert(MapEdge::new(0, 0, 0, 1));
        let mut second = MapRoomNode::new(0, 1);
        second.class = Some(RoomType::MonsterRoom);
        session.run_state.map = MapState::new(vec![vec![first], vec![second]]);
        session.run_state.monster_list = vec![EncounterId::JawWorm, EncounterId::Cultist];
        session
    }

    fn unique_temp_dir(label: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time should be monotonic")
            .as_nanos();
        std::env::temp_dir().join(format!("{label}_{nanos}"))
    }
}
