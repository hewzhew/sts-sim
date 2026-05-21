use super::*;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct UnsupportedActionPayload {
    pub source_class: String,
    pub source_fields: BTreeMap<String, String>,
    pub abort_reason: UnsupportedActionAbortReason,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum UnsupportedActionAbortReason {
    UnmodeledActionSubclass,
    UnmodeledSourceField { field_name: String },
    OpaqueEngineState { field_name: String },
    Unknown { source_name: String },
}
