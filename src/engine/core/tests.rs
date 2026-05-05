//! Test layering for `engine::core`:
//! - `control_flow`: player-input and pending-choice state-machine behavior
//! - `snapshot_regressions`: live-style snapshot fixtures built through state sync
//! - `silent`: character-specific coverage kept separate from generic engine regressions
//! - `support`: shared fixtures/helpers only; avoid placing assertions there

mod control_flow;
mod silent;
mod snapshot_regressions;
mod support;
