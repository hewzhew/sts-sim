use sts_simulator::runtime::branch::{
    OracleAnalysisWorkspaceArtifactV1, OracleAnalysisWorkspaceV1, OracleRunBudget, OracleRunConfig,
};

const SEED: u64 = 20_260_713_006;

fn workspace() -> OracleAnalysisWorkspaceV1 {
    OracleAnalysisWorkspaceV1::new(OracleRunConfig {
        seed: SEED,
        ascension: 0,
        budget: OracleRunBudget::default(),
    })
    .expect("analysis workspace")
}

#[test]
fn variations_are_created_without_mutating_the_parent_and_can_be_navigated() {
    let mut workspace = workspace();
    let root = workspace.view().expect("root view");
    let choice = root.choices.first().expect("root map choice").clone();
    let root_hp = root.current_hp;

    let child = workspace
        .try_choice(&choice.choice_ref)
        .expect("materialize child variation");
    assert_ne!(child.node_id, root.node_id);
    assert_eq!(workspace.session.cursor_node_id(), child.node_id);

    let parent_after = workspace
        .session
        .view_node(root.node_id)
        .expect("parent remains inspectable");
    assert_eq!(parent_after.current_hp, root_hp);
    assert!(parent_after
        .choices
        .iter()
        .any(|candidate| candidate.choice_ref == choice.choice_ref));
    let edge = parent_after
        .children
        .iter()
        .find(|edge| edge.child_node_id == child.node_id)
        .expect("variation edge")
        .edge_id;

    assert_eq!(workspace.session.back().expect("back"), root.node_id);
    workspace.session.follow_edge(edge).expect("follow child");
    assert_eq!(workspace.session.cursor_node_id(), child.node_id);
    workspace.session.promote_cursor();
    assert_eq!(workspace.session.mainline_node_id(), child.node_id);
}

#[test]
fn engine_owned_checkpoint_roundtrips_navigation_and_rejects_tampered_choices() {
    let mut workspace = workspace();
    let root = workspace.view().expect("root view");
    let choice = root.choices.first().expect("root map choice").clone();
    let child = workspace
        .try_choice(&choice.choice_ref)
        .expect("materialize child variation");
    workspace.session.promote_cursor();

    let mut tampered = choice.choice_ref.clone();
    tampered.push('0');
    assert!(workspace.try_choice(&tampered).is_err());

    let bytes =
        serde_json::to_vec(&workspace.artifact().expect("artifact")).expect("serialize artifact");
    let artifact = serde_json::from_slice::<OracleAnalysisWorkspaceArtifactV1>(&bytes)
        .expect("deserialize artifact");
    let mut restored = OracleAnalysisWorkspaceV1::restore(artifact).expect("restore workspace");
    let restored_view = restored.view().expect("restored view");
    assert_eq!(restored_view.node_id, child.node_id);
    assert_eq!(restored.session.mainline_node_id(), child.node_id);
    assert_eq!(
        restored.session.back().expect("restored back"),
        root.node_id
    );
}
