use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use sts_simulator::eval::run_control::{build_decision_surface, RunControlSession};

pub const BOOKMARK_REGISTRY_SCHEMA_NAME: &str = "RunPlayBookmarkRegistryV1";
pub const BOOKMARK_REGISTRY_SCHEMA_VERSION: u32 = 1;

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RunPlayBookmarkRegistryV1 {
    pub schema_name: String,
    pub schema_version: u32,
    pub bookmarks: BTreeMap<String, RunPlayBookmarkV1>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RunPlayBookmarkV1 {
    pub name: String,
    pub trace_path: String,
    pub replay_steps: usize,
    pub decision_step: u64,
    pub screen_title: String,
    pub act: u8,
    pub floor: i32,
    pub hp: i32,
    pub max_hp: i32,
    pub gold: i32,
    pub created_at_unix_ms: u128,
}

#[derive(Clone, Debug, PartialEq)]
pub struct GotoBookmarkPlan {
    pub bookmark: RunPlayBookmarkV1,
    pub source_trace_path: PathBuf,
    pub replay_steps: usize,
}

impl Default for RunPlayBookmarkRegistryV1 {
    fn default() -> Self {
        Self {
            schema_name: BOOKMARK_REGISTRY_SCHEMA_NAME.to_string(),
            schema_version: BOOKMARK_REGISTRY_SCHEMA_VERSION,
            bookmarks: BTreeMap::new(),
        }
    }
}

pub fn default_bookmark_registry_path() -> PathBuf {
    PathBuf::from("tools/artifacts/traces/bookmarks.json")
}

pub fn validate_bookmark_name(name: &str) -> Result<(), String> {
    if name.is_empty() {
        return Err("bookmark name cannot be empty".to_string());
    }
    if name.len() > 64 {
        return Err("bookmark name is too long; use 64 characters or fewer".to_string());
    }
    if !name
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-' | '.'))
    {
        return Err(
            "bookmark name may only contain ASCII letters, numbers, '.', '_' or '-'".to_string(),
        );
    }
    Ok(())
}

pub fn mark_current_boundary(
    registry_path: &Path,
    name: &str,
    trace_path: &Path,
    replay_steps: usize,
    session: &RunControlSession,
) -> Result<RunPlayBookmarkV1, String> {
    validate_bookmark_name(name)?;
    if replay_steps == 0 {
        return Err(
            "cannot mark before the trace has recorded any successful state-changing step"
                .to_string(),
        );
    }
    let mut registry = load_bookmark_registry(registry_path)?;
    let surface = build_decision_surface(session);
    let (hp, max_hp) = session_hp(session);
    let bookmark = RunPlayBookmarkV1 {
        name: name.to_string(),
        trace_path: trace_path.display().to_string(),
        replay_steps,
        decision_step: session.decision_step,
        screen_title: surface.view.header.title,
        act: session.run_state.act_num,
        floor: session.run_state.floor_num,
        hp,
        max_hp,
        gold: session.run_state.gold,
        created_at_unix_ms: current_unix_ms(),
    };
    registry
        .bookmarks
        .insert(name.to_string(), bookmark.clone());
    save_bookmark_registry(registry_path, &registry)?;
    Ok(bookmark)
}

pub fn resolve_goto_bookmark(registry_path: &Path, name: &str) -> Result<GotoBookmarkPlan, String> {
    validate_bookmark_name(name)?;
    let registry = load_bookmark_registry(registry_path)?;
    let bookmark = registry
        .bookmarks
        .get(name)
        .cloned()
        .ok_or_else(|| format!("bookmark '{name}' does not exist"))?;
    Ok(GotoBookmarkPlan {
        source_trace_path: PathBuf::from(&bookmark.trace_path),
        replay_steps: bookmark.replay_steps,
        bookmark,
    })
}

pub fn render_bookmarks(registry_path: &Path) -> Result<String, String> {
    let registry = load_bookmark_registry(registry_path)?;
    if registry.bookmarks.is_empty() {
        return Ok("Bookmarks: none".to_string());
    }
    let mut lines = vec!["Bookmarks:".to_string()];
    for bookmark in registry.bookmarks.values() {
        lines.push(format!(
            "  {} | {} | Act {} Floor {} | HP {}/{} | Gold {} | replay_steps={} | goto: --goto {} | {}",
            bookmark.name,
            bookmark.screen_title,
            bookmark.act,
            bookmark.floor,
            bookmark.hp,
            bookmark.max_hp,
            bookmark.gold,
            bookmark.replay_steps,
            bookmark.name,
            bookmark.trace_path
        ));
    }
    Ok(lines.join("\n"))
}

pub fn load_bookmark_registry(path: &Path) -> Result<RunPlayBookmarkRegistryV1, String> {
    if !path.exists() {
        return Ok(RunPlayBookmarkRegistryV1::default());
    }
    let payload = fs::read_to_string(path)
        .map_err(|err| format!("failed to read bookmark registry {}: {err}", path.display()))?;
    let registry: RunPlayBookmarkRegistryV1 = serde_json::from_str(&payload).map_err(|err| {
        format!(
            "failed to parse bookmark registry {}: {err}",
            path.display()
        )
    })?;
    if registry.schema_name != BOOKMARK_REGISTRY_SCHEMA_NAME {
        return Err(format!(
            "unsupported bookmark registry schema '{}'",
            registry.schema_name
        ));
    }
    if registry.schema_version != BOOKMARK_REGISTRY_SCHEMA_VERSION {
        return Err(format!(
            "unsupported bookmark registry version {}",
            registry.schema_version
        ));
    }
    Ok(registry)
}

fn save_bookmark_registry(path: &Path, registry: &RunPlayBookmarkRegistryV1) -> Result<(), String> {
    if let Some(parent) = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        fs::create_dir_all(parent).map_err(|err| {
            format!(
                "failed to create bookmark registry directory {}: {err}",
                parent.display()
            )
        })?;
    }
    let payload = serde_json::to_string_pretty(registry)
        .map_err(|err| format!("failed to serialize bookmark registry: {err}"))?;
    fs::write(path, payload).map_err(|err| {
        format!(
            "failed to write bookmark registry {}: {err}",
            path.display()
        )
    })
}

fn current_unix_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or(0)
}

fn session_hp(session: &RunControlSession) -> (i32, i32) {
    session
        .active_combat
        .as_ref()
        .map(|active| {
            (
                active.combat_state.entities.player.current_hp,
                active.combat_state.entities.player.max_hp,
            )
        })
        .unwrap_or((session.run_state.current_hp, session.run_state.max_hp))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bookmark_name_rejects_path_like_values() {
        assert!(validate_bookmark_name("before_reward").is_ok());
        assert!(validate_bookmark_name("act1.shop-1").is_ok());
        assert!(validate_bookmark_name("../bad").is_err());
        assert!(validate_bookmark_name("bad/name").is_err());
        assert!(validate_bookmark_name("").is_err());
    }

    #[test]
    fn mark_current_boundary_saves_and_overwrites_named_bookmark() {
        let dir = unique_temp_dir("bookmarks_mark");
        let registry_path = dir.join("bookmarks.json");
        let first_trace = dir.join("first.trace.json");
        let second_trace = dir.join("second.trace.json");
        let mut session = RunControlSession::new(Default::default());

        let first = mark_current_boundary(&registry_path, "retry", &first_trace, 3, &session)
            .expect("bookmark should save");
        session.decision_step = 7;
        session.run_state.gold = 123;
        let second = mark_current_boundary(&registry_path, "retry", &second_trace, 8, &session)
            .expect("bookmark should overwrite");

        let registry = load_bookmark_registry(&registry_path).expect("registry should load");
        assert_eq!(registry.bookmarks.len(), 1);
        assert_eq!(registry.bookmarks["retry"], second);
        assert_ne!(first.trace_path, second.trace_path);
        assert_eq!(registry.bookmarks["retry"].replay_steps, 8);
        assert_eq!(registry.bookmarks["retry"].decision_step, 7);
        assert_eq!(registry.bookmarks["retry"].gold, 123);

        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn resolve_goto_bookmark_returns_trace_path_and_replay_steps() {
        let dir = unique_temp_dir("bookmarks_goto");
        let registry_path = dir.join("bookmarks.json");
        let trace_path = dir.join("seed.trace.json");
        let session = RunControlSession::new(Default::default());
        mark_current_boundary(&registry_path, "before_card", &trace_path, 11, &session)
            .expect("bookmark should save");

        let plan =
            resolve_goto_bookmark(&registry_path, "before_card").expect("bookmark should resolve");

        assert_eq!(plan.source_trace_path, trace_path);
        assert_eq!(plan.replay_steps, 11);
        assert_eq!(plan.bookmark.name, "before_card");

        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn render_bookmarks_lists_human_summary() {
        let dir = unique_temp_dir("bookmarks_render");
        let registry_path = dir.join("bookmarks.json");
        let trace_path = dir.join("seed.trace.json");
        let session = RunControlSession::new(Default::default());
        mark_current_boundary(&registry_path, "start", &trace_path, 1, &session)
            .expect("bookmark should save");

        let rendered = render_bookmarks(&registry_path).expect("bookmarks should render");

        assert!(rendered.contains("Bookmarks:"));
        assert!(rendered.contains("start | Neow Intro"));
        assert!(rendered.contains("replay_steps=1"));
        assert!(rendered.contains("goto: --goto start"));

        let _ = fs::remove_dir_all(dir);
    }

    fn unique_temp_dir(label: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time should be after unix epoch")
            .as_nanos();
        std::env::temp_dir().join(format!("{label}_{}_{}", std::process::id(), nanos))
    }
}
