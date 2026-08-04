use clap::{Args, Subcommand};

#[derive(Debug, Subcommand)]
pub(super) enum CombatScratchCommand {
    /// Bind a new scratch DAG to the current or selected run combat node.
    Start {
        #[arg(long)]
        node: Option<usize>,
        #[arg(long, default_value_t = 512)]
        max_engine_steps_per_transition: usize,
        #[command(flatten)]
        page: CombatScratchPageArgs,
    },
    /// Show the exact scratch cursor state and its bound legal action refs.
    Status {
        #[command(flatten)]
        page: CombatScratchPageArgs,
    },
    /// Apply one exact action ref returned by scratch status.
    Play {
        #[arg(long)]
        action_ref: String,
        #[command(flatten)]
        page: CombatScratchPageArgs,
    },
    /// Move to the parent scratch node without deleting the current branch.
    Back {
        #[command(flatten)]
        page: CombatScratchPageArgs,
    },
    /// Move directly to one retained scratch node.
    Focus {
        #[arg(long)]
        scratch_node: u64,
        #[command(flatten)]
        page: CombatScratchPageArgs,
    },
    /// Run one small potion-free search from the current cursor and append a verified suffix.
    Search {
        #[arg(long, default_value_t = 4)]
        max_quanta: usize,
        #[arg(long, default_value_t = 1_024)]
        quantum_nodes: usize,
        #[arg(long, default_value_t = 100)]
        quantum_ms: u64,
        #[arg(long, default_value_t = 1_000)]
        wall_ms: u64,
        #[command(flatten)]
        page: CombatScratchPageArgs,
    },
    /// Show the compact scratch DAG without serializing combat positions.
    Tree,
    /// Atomically commit the terminal winning prefix as one run combat witness.
    Commit,
    /// Delete the active scratch DAG without changing the run variation tree.
    Clear,
}

#[derive(Clone, Copy, Debug, Args)]
pub(super) struct CombatScratchPageArgs {
    #[arg(long, default_value_t = 0)]
    pub selection_offset: usize,
    #[arg(long, default_value_t = 24, value_parser = clap::value_parser!(u8).range(1..=64))]
    pub selection_limit: u8,
}
