use std::ffi::{c_int, c_void};
use std::ptr;

const MOTION_EPSILON: f64 = 0.5;

pub struct PlayerMotionTracker {
    owner: *mut c_void,
    last_x: f64,
    last_y: f64,
    has_last: c_int,
}

impl PlayerMotionTracker {
    pub const fn new() -> Self {
        Self {
            owner: ptr::null_mut(),
            last_x: 0.0,
            last_y: 0.0,
            has_last: 0,
        }
    }
}

pub unsafe fn player_motion_tracker_ignore_stationary(
    tracker: *mut PlayerMotionTracker,
    owner: *mut c_void,
    x: f64,
    y: f64,
) -> c_int {
    let tracker = &mut *tracker;

    if tracker.has_last != 0
        && tracker.owner == owner
        && (x - tracker.last_x).abs() < MOTION_EPSILON
        && (y - tracker.last_y).abs() < MOTION_EPSILON
    {
        return 1;
    }

    tracker.owner = owner;
    tracker.last_x = x;
    tracker.last_y = y;
    tracker.has_last = 1;

    0
}
