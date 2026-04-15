use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::BufRead;
use std::path::Path;
use std::sync::Arc;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum RootPriorQueryKey {
    SpecEpisodeStep {
        spec_name: String,
        episode_id: usize,
        step_index: usize,
    },
    ReplayFrame {
        source_path: String,
        frame: u64,
    },
}

impl RootPriorQueryKey {
    fn normalize_replay_path(path: &str) -> String {
        path.replace('/', "\\").to_ascii_lowercase()
    }

    pub fn normalized(&self) -> Self {
        match self {
            Self::SpecEpisodeStep {
                spec_name,
                episode_id,
                step_index,
            } => Self::SpecEpisodeStep {
                spec_name: spec_name.clone(),
                episode_id: *episode_id,
                step_index: *step_index,
            },
            Self::ReplayFrame { source_path, frame } => Self::ReplayFrame {
                source_path: Self::normalize_replay_path(source_path),
                frame: *frame,
            },
        }
    }

    pub fn as_string(&self) -> String {
        match self.normalized() {
            Self::SpecEpisodeStep {
                spec_name,
                episode_id,
                step_index,
            } => format!("spec_episode_step::{spec_name}::{episode_id}::{step_index}"),
            Self::ReplayFrame { source_path, frame } => {
                format!("replay_frame::{source_path}::{frame}")
            }
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RootPriorArtifactRow {
    pub root_prior_key: RootPriorQueryKey,
    pub sample_origin: String,
    pub group_id: String,
    pub candidate_move: String,
    pub aggregate_score: f32,
    #[serde(default)]
    pub survival_score: Option<f32>,
    #[serde(default)]
    pub tempo_score: Option<f32>,
    #[serde(default)]
    pub setup_payoff_score: Option<f32>,
    #[serde(default)]
    pub kill_window_score: Option<f32>,
    #[serde(default)]
    pub risk_score: Option<f32>,
    #[serde(default)]
    pub mean_return: Option<f32>,
    #[serde(default)]
    pub teacher_score: Option<f32>,
    #[serde(default)]
    pub uncertain: bool,
    #[serde(default)]
    pub teacher_best: bool,
    #[serde(default)]
    pub candidate_rank: Option<usize>,
    #[serde(default)]
    pub candidate_score_hint: Option<f32>,
}

#[derive(Clone, Debug)]
pub struct RootPriorScore {
    pub aggregate_score: f32,
    pub survival_score: Option<f32>,
    pub tempo_score: Option<f32>,
    pub setup_payoff_score: Option<f32>,
    pub kill_window_score: Option<f32>,
    pub risk_score: Option<f32>,
    pub mean_return: Option<f32>,
}

impl From<&RootPriorArtifactRow> for RootPriorScore {
    fn from(value: &RootPriorArtifactRow) -> Self {
        Self {
            aggregate_score: value.aggregate_score,
            survival_score: value.survival_score,
            tempo_score: value.tempo_score,
            setup_payoff_score: value.setup_payoff_score,
            kill_window_score: value.kill_window_score,
            risk_score: value.risk_score,
            mean_return: value.mean_return,
        }
    }
}

pub trait RootPriorProvider: Send + Sync {
    fn score(&self, key: &RootPriorQueryKey, move_label: &str) -> Option<RootPriorScore>;
}

#[derive(Clone)]
pub struct RootPriorConfig {
    pub provider: Arc<dyn RootPriorProvider>,
    pub key: RootPriorQueryKey,
    pub weight: f32,
    pub shadow: bool,
}

#[derive(Clone, Default)]
pub struct LookupRootPriorProvider {
    entries: HashMap<RootPriorQueryKey, HashMap<String, RootPriorArtifactRow>>,
}

impl LookupRootPriorProvider {
    pub fn load_jsonl(path: &Path) -> Result<Self, String> {
        let file = std::fs::File::open(path).map_err(|err| {
            format!(
                "failed to open q_local root prior artifact '{}': {err}",
                path.display()
            )
        })?;
        let reader = std::io::BufReader::new(file);
        let mut entries: HashMap<RootPriorQueryKey, HashMap<String, RootPriorArtifactRow>> =
            HashMap::new();
        for (line_no, line) in reader.lines().enumerate() {
            let text = line.map_err(|err| {
                format!(
                    "failed to read q_local root prior artifact '{}' at line {}: {err}",
                    path.display(),
                    line_no + 1
                )
            })?;
            let trimmed = text.trim();
            if trimmed.is_empty() {
                continue;
            }
            let row: RootPriorArtifactRow = serde_json::from_str(trimmed).map_err(|err| {
                format!(
                    "failed to parse q_local root prior artifact '{}' at line {}: {err}",
                    path.display(),
                    line_no + 1
                )
            })?;
            let normalized_key = row.root_prior_key.normalized();
            entries
                .entry(normalized_key)
                .or_default()
                .insert(row.candidate_move.clone(), row);
        }
        Ok(Self { entries })
    }

    pub fn candidate_count(&self, key: &RootPriorQueryKey) -> usize {
        let normalized = key.normalized();
        self.entries
            .get(&normalized)
            .map(|rows| rows.len())
            .unwrap_or(0)
    }
}

impl RootPriorProvider for LookupRootPriorProvider {
    fn score(&self, key: &RootPriorQueryKey, move_label: &str) -> Option<RootPriorScore> {
        let normalized = key.normalized();
        self.entries
            .get(&normalized)
            .and_then(|rows| rows.get(move_label))
            .map(RootPriorScore::from)
    }
}
