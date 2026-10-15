mod resolve;

pub use crate::world_action::{Actor, ScheduledAction, WorldAction};
pub use resolve::{apply_world_action, ActionOutcome};
