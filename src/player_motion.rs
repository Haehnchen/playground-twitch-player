use std::ffi::{c_int, c_void};
use std::ptr;

const MOTION_EPSILON: f64 = 0.5;

pub struct PlayerMotionTracker {
    owner: *mut c_void,
    last_x: f64,
    last_y: f64,
    has_last: bool,
}

impl PlayerMotionTracker {
    pub const fn new() -> Self {
        Self {
            owner: ptr::null_mut(),
            last_x: 0.0,
            last_y: 0.0,
            has_last: false,
        }
    }
}

impl Default for PlayerMotionTracker {
    fn default() -> Self {
        Self::new()
    }
}

pub fn player_motion_tracker_ignore_stationary(
    tracker: &mut PlayerMotionTracker,
    owner: *mut c_void,
    x: f64,
    y: f64,
) -> c_int {
    if tracker.has_last
        && tracker.owner == owner
        && (x - tracker.last_x).abs() < MOTION_EPSILON
        && (y - tracker.last_y).abs() < MOTION_EPSILON
    {
        return 1;
    }

    tracker.owner = owner;
    tracker.last_x = x;
    tracker.last_y = y;
    tracker.has_last = true;

    0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tracks_motion_per_owner_and_accumulates_small_movements() {
        let mut tracker = PlayerMotionTracker::new();
        let mut first = 0_u8;
        let mut second = 0_u8;
        let first = ptr::from_mut(&mut first).cast();
        let second = ptr::from_mut(&mut second).cast();
        assert_eq!(
            player_motion_tracker_ignore_stationary(&mut tracker, first, 0.0, 0.0),
            0
        );
        assert_eq!(
            player_motion_tracker_ignore_stationary(&mut tracker, first, 0.2, 0.2),
            1
        );
        assert_eq!(
            player_motion_tracker_ignore_stationary(&mut tracker, first, 0.5, 0.0),
            0
        );
        assert_eq!(
            player_motion_tracker_ignore_stationary(&mut tracker, second, 0.5, 0.0),
            0
        );
        assert_eq!(
            player_motion_tracker_ignore_stationary(&mut tracker, second, 0.5, 0.0),
            1
        );
        assert_eq!(
            player_motion_tracker_ignore_stationary(&mut tracker, second, 0.5, -0.5),
            0
        );
    }
}
