use sts_simulator::eval::branch_experiment::{
    BranchExperimentBranchReportV1, BranchExperimentChoiceCardV1, BranchExperimentChoiceV1,
    BranchExperimentReportV1,
};

#[derive(Clone, Debug, Default)]
pub(crate) struct ChoiceFocus {
    common_prefix_len: usize,
    skipped_labels: Vec<String>,
}

impl ChoiceFocus {
    pub(crate) fn for_report(report: &BranchExperimentReportV1) -> Self {
        if report.branches.len() < 2 {
            return Self::default();
        }
        let common_prefix_len = common_choice_prefix_len(&report.branches);
        let has_focused_suffix = report
            .branches
            .iter()
            .any(|branch| branch.choices.len() > common_prefix_len);
        if common_prefix_len == 0 || !has_focused_suffix {
            return Self::default();
        }

        let skipped_labels = report
            .branches
            .first()
            .into_iter()
            .flat_map(|branch| branch.choices.iter().take(common_prefix_len))
            .map(super::choice_display_label)
            .collect();
        Self {
            common_prefix_len,
            skipped_labels,
        }
    }

    pub(crate) fn has_skipped_prefix(&self) -> bool {
        self.common_prefix_len > 0
    }

    pub(crate) fn notice_line(&self) -> Option<String> {
        self.has_skipped_prefix().then(|| {
            format!(
                "Decision focus: skipped shared prefix [{}]",
                self.skipped_labels.join(" -> ")
            )
        })
    }

    pub(crate) fn focused_choice<'a>(
        &self,
        branch: &'a BranchExperimentBranchReportV1,
    ) -> Option<&'a BranchExperimentChoiceV1> {
        if self.has_skipped_prefix() {
            branch.choices.get(self.common_prefix_len)
        } else {
            branch.choices.first()
        }
    }

    pub(crate) fn render_choice_path(&self, branch: &BranchExperimentBranchReportV1) -> String {
        let choices = branch
            .choices
            .iter()
            .skip(self.common_prefix_len)
            .collect::<Vec<_>>();
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

fn common_choice_prefix_len(branches: &[BranchExperimentBranchReportV1]) -> usize {
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
