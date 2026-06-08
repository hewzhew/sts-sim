use std::collections::BTreeSet;

use sts_simulator::eval::branch_experiment::{
    BranchExperimentBranchReportV1, BranchExperimentChoiceCardV1, BranchExperimentChoiceV1,
    BranchExperimentReportV1,
};

#[derive(Clone, Debug, Default)]
pub(crate) struct ChoiceFocus {
    common_prefix_len: usize,
    skipped_labels: Vec<String>,
    target_boundary: Option<String>,
    target_boundary_key: Option<String>,
    matched_branch_ids: BTreeSet<String>,
    total_branches: usize,
}

impl ChoiceFocus {
    pub(crate) fn for_report(report: &BranchExperimentReportV1) -> Self {
        Self::from_branches(report, None)
    }

    pub(crate) fn for_target_boundary(
        report: &BranchExperimentReportV1,
        target_boundary: &str,
    ) -> Self {
        Self::from_branches(report, Some(target_boundary))
    }

    fn from_branches(report: &BranchExperimentReportV1, target_boundary: Option<&str>) -> Self {
        let target_boundary_key = target_boundary.map(normalized_key);
        let branches = report
            .branches
            .iter()
            .filter(|branch| {
                target_boundary_key.as_deref().map_or(true, |key| {
                    normalized_key(&branch.summary.boundary_title) == key
                })
            })
            .collect::<Vec<_>>();
        let matched_branch_ids = branches
            .iter()
            .map(|branch| branch.branch_id.clone())
            .collect::<BTreeSet<_>>();
        if branches.len() < 2 {
            return Self {
                target_boundary: target_boundary.map(ToOwned::to_owned),
                target_boundary_key,
                matched_branch_ids,
                total_branches: report.branches.len(),
                ..Self::default()
            };
        }
        let common_prefix_len = common_choice_prefix_len(&branches);
        let has_focused_suffix = branches
            .iter()
            .any(|branch| branch.choices.len() > common_prefix_len);
        if common_prefix_len == 0 || !has_focused_suffix {
            return Self {
                target_boundary: target_boundary.map(ToOwned::to_owned),
                target_boundary_key,
                matched_branch_ids,
                total_branches: report.branches.len(),
                ..Self::default()
            };
        }

        let skipped_labels = report
            .branches
            .iter()
            .find(|branch| matched_branch_ids.contains(&branch.branch_id))
            .into_iter()
            .flat_map(|branch| branch.choices.iter().take(common_prefix_len))
            .map(super::choice_display_label)
            .collect();
        Self {
            common_prefix_len,
            skipped_labels,
            target_boundary: target_boundary.map(ToOwned::to_owned),
            target_boundary_key,
            matched_branch_ids,
            total_branches: report.branches.len(),
        }
    }

    pub(crate) fn is_targeted(&self) -> bool {
        self.target_boundary.is_some()
    }

    pub(crate) fn has_skipped_prefix(&self) -> bool {
        self.common_prefix_len > 0
    }

    pub(crate) fn notice_line(&self) -> Option<String> {
        match (self.target_boundary.as_deref(), self.has_skipped_prefix()) {
            (Some(boundary), true) => Some(format!(
                "Decision focus: target boundary [{boundary}] matched {}/{} branch(es); skipped shared prefix [{}]",
                self.matched_branch_ids.len(),
                self.total_branches,
                self.skipped_labels.join(" -> ")
            )),
            (Some(boundary), false) => Some(format!(
                "Decision focus: target boundary [{boundary}] matched {}/{} branch(es)",
                self.matched_branch_ids.len(),
                self.total_branches
            )),
            (None, true) => Some(format!(
                "Decision focus: skipped shared prefix [{}]",
                self.skipped_labels.join(" -> ")
            )),
            (None, false) => None,
        }
    }

    pub(crate) fn branch_in_scope(&self, branch: &BranchExperimentBranchReportV1) -> bool {
        !self.is_targeted() || self.matched_branch_ids.contains(&branch.branch_id)
    }

    pub(crate) fn focused_choice<'a>(
        &self,
        branch: &'a BranchExperimentBranchReportV1,
    ) -> Option<&'a BranchExperimentChoiceV1> {
        if !self.branch_in_scope(branch) {
            return None;
        }
        if let Some(target_key) = self.target_boundary_key.as_deref() {
            if let Some(choice) = branch
                .choices
                .iter()
                .find(|choice| normalized_key(&choice.kind) == target_key)
            {
                return Some(choice);
            }
        }
        if self.has_skipped_prefix() {
            branch.choices.get(self.common_prefix_len)
        } else {
            branch.choices.first()
        }
    }

    pub(crate) fn render_choice_path(&self, branch: &BranchExperimentBranchReportV1) -> String {
        if !self.branch_in_scope(branch) {
            return "-".to_string();
        }
        let start_index = if let Some(target_key) = self.target_boundary_key.as_deref() {
            branch
                .choices
                .iter()
                .position(|choice| normalized_key(&choice.kind) == target_key)
                .unwrap_or(self.common_prefix_len)
        } else {
            self.common_prefix_len
        };
        let choices = branch.choices.iter().skip(start_index).collect::<Vec<_>>();
        if choices.is_empty() {
            return "-".to_string();
        }
        choices
            .into_iter()
            .map(super::choice_display_label)
            .collect::<Vec<_>>()
            .join(" -> ")
    }
}

fn common_choice_prefix_len(branches: &[&BranchExperimentBranchReportV1]) -> usize {
    let min_len = branches
        .iter()
        .map(|branch| branch.choices.len())
        .min()
        .unwrap_or(0);
    let Some(first_branch) = branches.first() else {
        return 0;
    };
    let mut prefix_len = 0;
    for index in 0..min_len {
        let key = choice_prefix_key(&first_branch.choices[index]);
        if branches
            .iter()
            .all(|branch| choice_prefix_key(&branch.choices[index]) == key)
        {
            prefix_len += 1;
        } else {
            break;
        }
    }
    prefix_len
}

fn normalized_key(value: &str) -> String {
    value
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric())
        .map(|ch| ch.to_ascii_lowercase())
        .collect()
}

fn choice_prefix_key(choice: &BranchExperimentChoiceV1) -> ChoicePrefixKey {
    ChoicePrefixKey {
        kind: choice.kind.clone(),
        card: choice.card.map(|card| format!("{card:?}")),
        upgrades: choice.upgrades,
        selected_cards: choice.selected_cards.iter().map(card_key).collect(),
        effect_kind: choice.effect_kind.clone(),
        effect_key: choice.effect_key.clone(),
        effect_label: choice.effect_label.clone(),
        label: choice.label.clone(),
        command: choice.command.clone(),
    }
}

fn card_key(card: &BranchExperimentChoiceCardV1) -> String {
    format!("{:?}+{}", card.card, card.upgrades)
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct ChoicePrefixKey {
    kind: String,
    card: Option<String>,
    upgrades: Option<u8>,
    selected_cards: Vec<String>,
    effect_kind: String,
    effect_key: String,
    effect_label: String,
    label: String,
    command: String,
}
