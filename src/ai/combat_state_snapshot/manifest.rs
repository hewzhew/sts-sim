#![allow(unused_imports)]

use super::*;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceManifest {
    pub cardcrawl_root: SourcePath,
    pub game_version: String,
    pub decompile_manifest_hash: String,
    pub source_files: Vec<SourceFileManifestEntry>,
    pub simulator_commit: String,
    pub schema_hash: String,
    pub content_manifest_hash: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceFileManifestEntry {
    pub source_path: SourcePath,
    pub sha256: String,
    pub byte_len: u64,
    pub line_count: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum CombatSnapshotOrigin {
    AuthoredProbe { name: String },
    ReplayExtracted { replay_id: String },
    BridgeExtracted { capture_id: String },
}
