#include "player_motion.h"

#include <math.h>

#define MOTION_EPSILON 0.5

gboolean player_motion_tracker_ignore_stationary(PlayerMotionTracker *tracker, gpointer owner, double x, double y)
{
    if (tracker->has_last &&
        tracker->owner == owner &&
        fabs(x - tracker->last_x) < MOTION_EPSILON &&
        fabs(y - tracker->last_y) < MOTION_EPSILON) {
        return TRUE;
    }

    tracker->owner = owner;
    tracker->last_x = x;
    tracker->last_y = y;
    tracker->has_last = TRUE;

    return FALSE;
}
