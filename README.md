# push2_soundboard

A small tool to use the Ableton Push2 midi controller to create a sound control-center for mood setting.
The main idea is to use this tool to control sound effects and music used in pen and paper roleplaying games.

It can play short sound-files on button presses, play looping sound with fade-in and out, it can run arbitrary cmd-commands on buttons.
Also in development is the feature `spotify` which enables a spotify device-mode you can switch to at runtime. This mode currently only displays the song playing on your linked (OAuth authentification flow is used) spotify account, but further control is planned.


## Features

### Sound configuration
- Fade-in and fade-out
- Looping
- Per sound gain
- hot reloaded config-file
- cmd/shell commands fired on button-press

### General
- Configurable device-names
- Color coded playback and config display on buttons
- Interrupt-Mode for repeated play (for example for the classic Airhorn sound)
- Display shows list of playing sounds
- Volume control

### Spotify Control integration
- OAuth authentification of your spotify account
- Display currently playing track on Push2 display

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or
  http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in the
work by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any
additional terms or conditions.
