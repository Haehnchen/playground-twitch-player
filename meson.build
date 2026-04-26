project(
  'twitch-player-2',
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

executable(
  'twitch-player-2',
  [
    'src/main.c',
    'src/chat_panel.c',
    'src/twitch_chat.c',
  ],
  dependencies: [gtk_dep, mpv_dep, epoxy_dep],
  install: true,
)
