use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct SourceIdentity {
    pub git_commit: Option<String>,
    pub git_dirty: Option<bool>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn source_identity_is_structured_and_serializable() {
        let source = SourceIdentity {
            git_commit: Some("abc123".to_string()),
            git_dirty: Some(false),
        };

        let value = serde_json::to_value(&source).unwrap();

        assert_eq!(value["git_commit"], "abc123");
        assert_eq!(value["git_dirty"], false);
    }
}
