#pragma once

#include <glib.h>

typedef struct {
    gpointer owner;
    double last_x;
    double last_y;
    gboolean has_last;
} PlayerMotionTracker;

gboolean player_motion_tracker_ignore_stationary(PlayerMotionTracker *tracker, gpointer owner, double x, double y);
