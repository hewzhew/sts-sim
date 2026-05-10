#![allow(unused_imports)]

use super::*;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceCoverageEntry {
    pub source_file: SourcePath,
    pub source_class: String,
    pub source_member: String,
    pub mechanic_role: String,
    pub classification: CoverageClassification,
    pub schema_path: Option<SchemaPath>,
    pub public_visibility: PublicVisibility,
    pub replay_required: bool,
    pub rust_owner_module: Option<RustPath>,
    pub rust_status: RustMigrationStatus,
    pub migration_decision: Option<String>,
    pub acceptance_check: Option<String>,
    pub notes: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum CoverageClassification {
    Modeled,
    Derived,
    RenderOnly,
    RunLevelMaterialized,
    NonCombat,
    UnsupportedAbort,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum PublicVisibility {
    Public,
    Privileged,
    DebugOnly,
    Hidden,
    NotApplicable,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MigrationLedgerEntry {
    pub java_source: SourcePath,
    pub java_methods: Vec<String>,
    pub java_fields: Vec<String>,
    pub java_semantic_behavior: String,
    pub rust_module: RustPath,
    pub rust_type: String,
    pub migration_kind: MigrationKind,
    pub preserved_features: Vec<String>,
    pub intentional_structural_changes: Vec<String>,
    pub semantic_equivalence_tests: Vec<String>,
    pub unsupported_cases: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum MigrationKind {
    DirectModel,
    DerivedModel,
    StructuralRedesign,
    UnsupportedAbort,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum RustMigrationStatus {
    Keep,
    Rewrite,
    Delete,
    AdapterOnly,
    Unknown,
}
