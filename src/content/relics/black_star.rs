use crate::action::ActionInfo;
use smallvec::SmallVec;

pub struct BlackStar;

impl BlackStar {
    // Black star gives an extra relic directly on victory if it's an Elite room.
    // In our headless engine, room types are tied off to external rewards.
    // This serves as an unimplemented stub that triggers if the UI asks.
    pub fn on_victory() -> SmallVec<[ActionInfo; 4]> {
        SmallVec::new()
    }
}
