project(
  'twitch-player',
  'c',
  version: '0.1.0',
  default_options: [
    'warning_level=2',
    'c_std=c11',
  ],
)

gtk_dep = dependency('gtk4', version: '>=4.10')
gio_dep = dependency('gio-2.0')
mpv_dep = dependency('mpv')
epoxy_dep = dependency('epoxy')
json_glib_dep = dependency('json-glib-1.0')
libsoup_dep = dependency('libsoup-3.0')
gnome = import('gnome')

resources = gnome.compile_resources(
  'twitch-player-resources',
  'data/local.twitch-player.gresource.xml',
  source_dir: 'data',
  c_name: 'twitch_player',
)

executable(
  'twitch-player',
  [
    'src/main.c',
    'src/player_session.c',
    'src/single_player.c',
    'src/grid_player.c',
    'src/player_icons.c',
    'src/player_motion.c',
    'src/settings.c',
    'src/chat_assets.c',
    'src/chat_panel.c',
    'src/settings_window.c',
    'src/twitch_chat.c',
    'src/twitch_stream_info.c',
    resources,
  ],
  dependencies: [gtk_dep, mpv_dep, epoxy_dep, json_glib_dep, libsoup_dep],
  install: true,
)

test_twitch_chat = executable(
  'test-twitch-chat',
  'tests/test_twitch_chat.c',
  dependencies: [gio_dep],
)
test('twitch-chat', test_twitch_chat)

test_twitch_stream_info = executable(
  'test-twitch-stream-info',
  'tests/test_twitch_stream_info.c',
  dependencies: [gio_dep, json_glib_dep, libsoup_dep],
)
test('twitch-stream-info', test_twitch_stream_info)

test_app_settings = executable(
  'test-app-settings',
  [
    'tests/test_settings.c',
    'src/settings.c',
  ],
  dependencies: [gio_dep, json_glib_dep],
)
test('settings', test_app_settings)

test_chat_assets = executable(
  'test-chat-assets',
  'tests/test_chat_assets.c',
  dependencies: [gtk_dep],
)
test('chat-assets', test_chat_assets)

test_chat_panel = executable(
  'test-chat-panel',
  [
    'tests/test_chat_panel.c',
    'src/chat_assets.c',
    'src/twitch_chat.c',
  ],
  dependencies: [gtk_dep, gio_dep],
)
test('chat-panel', test_chat_panel)
