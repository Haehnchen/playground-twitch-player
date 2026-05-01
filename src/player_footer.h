#pragma once

#include <gtk/gtk.h>

typedef struct _PlayerFooterStreamInfo PlayerFooterStreamInfo;

PlayerFooterStreamInfo *player_footer_stream_info_new(void);
GtkWidget *player_footer_stream_info_get_widget(PlayerFooterStreamInfo *info);
void player_footer_stream_info_set(PlayerFooterStreamInfo *info, const char *title, const char *metadata);
void player_footer_stream_info_clear(PlayerFooterStreamInfo *info);
void player_footer_stream_info_free(PlayerFooterStreamInfo *info);
