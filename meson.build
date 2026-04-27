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
    'src/chat_assets.c',
    'src/chat_panel.c',
    'src/twitch_chat.c',
    'src/twitch_stream_info.c',
    resources,
  ],
  dependencies: [gtk_dep, mpv_dep, epoxy_dep, json_glib_dep, libsoup_dep],
  install: true,
)
