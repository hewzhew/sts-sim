fn normalize_axis_part_v1(value: &str) -> String {
    value
        .chars()
        .filter_map(|ch| {
            if ch.is_ascii_alphanumeric() {
                Some(ch.to_ascii_lowercase())
            } else if ch == '_' || ch == '-' {
                Some('_')
            } else {
                None
            }
        })
        .collect::<String>()
        .trim_matches('_')
        .to_string()
}

pub(crate) fn decision_candidate_axis_key_v1(
    boundary_kind: &str,
    boundary_scope: &str,
    family: &str,
) -> Option<String> {
    let boundary_kind = normalize_axis_part_v1(boundary_kind);
    let boundary_scope = normalize_axis_part_v1(boundary_scope);
    let family = normalize_axis_part_v1(family);
    if boundary_kind.is_empty() || boundary_scope.is_empty() || family.is_empty() {
        return None;
    }
    Some(format!("{boundary_kind}:{boundary_scope}:{family}"))
}

pub(crate) fn shop_decision_candidate_axis_v1(family: &str) -> Option<String> {
    decision_candidate_axis_key_v1("shop", "shop", family)
}
