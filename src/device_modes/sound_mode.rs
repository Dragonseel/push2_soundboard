use std::{
    collections::HashMap,
    default,
    fs::File,
    io::Read,
    path::Path,
    sync::{
        mpsc::{channel, Receiver},
        Arc, Mutex,
    },
    time::Duration,
};

use embedded_graphics::{
    mono_font::{iso_8859_13::FONT_10X20, MonoTextStyle},
    pixelcolor::Bgr565,
    prelude::{Point, RgbColor, Size},
    primitives::{Primitive, PrimitiveStyle, Rectangle},
    text::Text,
    Drawable,
};
use notify_debouncer_full::{
    new_debouncer,
    notify::{ReadDirectoryChangesWatcher, RecursiveMode, Watcher},
    DebounceEventResult, DebouncedEvent, Debouncer, FileIdMap,
};
use push2_display::Push2Display;

use crate::{
    actions::{command::Command, sound::Sound, Action, ActionConfig, ActionState},
    button_map::{ButtonType, ControlName, NoteName},
    sound_system::SoundSystem,
    MyError, DEFAULT_VOLUME, MAX_VOLUME,
};

use super::LightAction;

#[derive(Deserialize)]
struct ActionConfigs {
    actions: Vec<ActionConfig>,
}

pub struct SoundMode {
    button_actions: HashMap<ButtonType, Action>,
    sound_system: Arc<Mutex<SoundSystem>>,
    file_watcher: Option<Receiver<DebouncedEvent>>,
    file_watcher_intern: Option<Debouncer<ReadDirectoryChangesWatcher, FileIdMap>>,
}

impl SoundMode {
    pub fn new(sound_system: Arc<Mutex<SoundSystem>>) -> SoundMode {
        let mut sound_mode = SoundMode {
            button_actions: default::Default::default(),
            sound_system,
            file_watcher: None,
            file_watcher_intern: None,
        };

        sound_mode.read_config("config/testconfig.ron");

        sound_mode
    }

    pub fn add_action(&mut self, button: ButtonType, action: Action) {
        if !self.button_actions.contains_key(&button) {
            self.button_actions.insert(button, action);
        } else {
            *self.button_actions.get_mut(&button).unwrap() = action;
        }
    }

    pub fn read_config(&mut self, path: &str) {
        let (_tx, rx) = channel();

        // Select recommended watcher for debouncer.
        // Using a callback here, could also be a channel.
        let mut debouncer = new_debouncer(
            Duration::from_secs(2),
            None,
            |result: DebounceEventResult| match result {
                Ok(events) => events.iter().for_each(|_event| {}),
                Err(errors) => errors.iter().for_each(|error| println!("{error:?}")),
            },
        )
        .unwrap();

        // Add a path to be watched. All files and directories at that path and
        // below will be monitored for changes.
        debouncer
            .watcher()
            .watch(Path::new("."), RecursiveMode::Recursive)
            .unwrap();

        // Add the same path to the file ID cache. The cache uses unique file IDs
        // provided by the file system and is used to stich together rename events
        // in case the notification back-end doesn't emit rename cookies.
        debouncer
            .cache()
            .add_root(Path::new("."), RecursiveMode::Recursive);

        self.read_config_impl(Path::new(path));

        self.file_watcher = Some(rx);
        self.file_watcher_intern = Some(debouncer);
    }

    fn read_config_impl(&mut self, path: &Path) {
        let mut file = File::open(path).unwrap();
        let mut config_string = String::new();
        file.read_to_string(&mut config_string)
            .expect("Could not read config file.");

        let action_configs: ActionConfigs =
            ron::de::from_str(&config_string).expect("Could not deserialize SoundConfig.");

        for action in action_configs.actions {
            match action {
                ActionConfig::SoundConfig {
                    button,
                    path,
                    looping,
                    fade_in,
                    fade_out,
                    gain,
                } => self.add_action(
                    button,
                    Action::Sound(Sound::load(path, looping, fade_in, fade_out, gain).unwrap()),
                ),
                ActionConfig::CommandConfig {
                    button,
                    command,
                    mut args,
                } => {
                    for arg in &mut args {
                        *arg = arg.trim().to_string();
                    }
                    self.add_action(button, Action::Command(Command::new(command, args)))
                }
            }
        }
    }
}

impl SoundMode {
    pub fn playing_sound_names(&self) -> Vec<(String, bool)> {
        let mut names = vec![];

        for (_button, action) in &self.button_actions {
            match action {
                Action::Sound(sound) => {
                    if sound.is_playing() {
                        names.push((sound.get_name(), sound.looped));
                    }
                }
                Action::Command(_) => {}
            }
        }

        names
    }

    fn draw_volume(
        sound_system: &Arc<Mutex<SoundSystem>>,
        display: &mut Push2Display,
    ) -> Result<(), MyError> {
        const VOLUME_BAR_X: i32 = 880;
        const VOLUME_BAR_Y: i32 = 10;

        const VOLUME_BAR_HEIGHT: u32 = 140;
        const VOLUME_BAR_WIDTH: u32 = 30;

        const TEXT_SIZE_OFFSET: f32 = 5.0;

        // Outline
        Rectangle::new(
            Point {
                x: VOLUME_BAR_X,
                y: VOLUME_BAR_Y,
            },
            Size {
                width: VOLUME_BAR_WIDTH,
                height: VOLUME_BAR_HEIGHT,
            },
        )
        .into_styled(PrimitiveStyle::with_stroke(Bgr565::WHITE, 2))
        .draw(display)?;

        let volume_factor = sound_system
            .try_lock()
            .expect("Couldn't lock SoundSystem.")
            .get_volume_factor()
            / (MAX_VOLUME as f32 / DEFAULT_VOLUME as f32);

        // Fill for current volume
        Rectangle::new(
            Point {
                x: VOLUME_BAR_X,
                y: ((VOLUME_BAR_Y as f32) + (1.0 - volume_factor) * (VOLUME_BAR_HEIGHT as f32))
                    as i32,
            },
            Size {
                width: VOLUME_BAR_WIDTH,
                height: ((VOLUME_BAR_HEIGHT as f32) * volume_factor) as u32,
            },
        )
        .into_styled(PrimitiveStyle::with_fill(Bgr565::WHITE))
        .draw(display)?;

        // 1.0 marker
        Rectangle::new(
            Point {
                x: VOLUME_BAR_X - 5,
                y: ((VOLUME_BAR_Y as f32) + (1.0 - 1.0 / 4.0) * (VOLUME_BAR_HEIGHT as f32)) as i32,
            },
            Size {
                width: 5,
                height: 2,
            },
        )
        .into_styled(PrimitiveStyle::with_fill(Bgr565::WHITE))
        .draw(display)?;

        // 1.0 Text
        Text::new(
            "100%",
            Point {
                x: VOLUME_BAR_X - 50,
                y: ((VOLUME_BAR_Y as f32)
                    + (1.0 - 1.0 / 4.0) * (VOLUME_BAR_HEIGHT as f32)
                    + TEXT_SIZE_OFFSET) as i32,
            },
            MonoTextStyle::new(&FONT_10X20, Bgr565::WHITE),
        )
        .draw(display)?;

        // 4.0 marker
        Rectangle::new(
            Point {
                x: VOLUME_BAR_X - 5,
                y: VOLUME_BAR_Y as i32,
            },
            Size {
                width: 5,
                height: 2,
            },
        )
        .into_styled(PrimitiveStyle::with_fill(Bgr565::WHITE))
        .draw(display)?;

        // 1.0 Text
        Text::new(
            "400%",
            Point {
                x: VOLUME_BAR_X - 50,
                y: ((VOLUME_BAR_Y as f32)
                    + (1.0 - 4.0 / 4.0) * (VOLUME_BAR_HEIGHT as f32)
                    + TEXT_SIZE_OFFSET) as i32,
            },
            MonoTextStyle::new(&FONT_10X20, Bgr565::WHITE),
        )
        .draw(display)?;

        // 0.0 marker
        Rectangle::new(
            Point {
                x: VOLUME_BAR_X - 5,
                y: ((VOLUME_BAR_Y as f32) + (1.0 - 0.0 / 4.0) * (VOLUME_BAR_HEIGHT as f32)) as i32,
            },
            Size {
                width: 5,
                height: 2,
            },
        )
        .into_styled(PrimitiveStyle::with_fill(Bgr565::WHITE))
        .draw(display)?;

        // 0.0 Text
        Text::new(
            "0%",
            Point {
                x: VOLUME_BAR_X - 30,
                y: ((VOLUME_BAR_Y as f32)
                    + (1.0 - 0.0 / 4.0) * (VOLUME_BAR_HEIGHT as f32)
                    + TEXT_SIZE_OFFSET) as i32,
            },
            MonoTextStyle::new(&FONT_10X20, Bgr565::WHITE),
        )
        .draw(display)?;

        Ok(())
    }

    fn display_sounds(&self, display: &mut Push2Display) -> Result<(), MyError> {
        // One-shots header
        Text::new(
            "One-Shots",
            Point { x: 50, y: 15 },
            MonoTextStyle::new(&FONT_10X20, Bgr565::WHITE),
        )
        .draw(display)?;

        Rectangle::new(
            Point { x: 50, y: 20 },
            Size {
                width: 90,
                height: 2,
            },
        )
        .into_styled(PrimitiveStyle::with_fill(Bgr565::WHITE))
        .draw(display)?;

        // Loops header
        Text::new(
            "Loops",
            Point { x: 400, y: 15 },
            MonoTextStyle::new(&FONT_10X20, Bgr565::WHITE),
        )
        .draw(display)?;

        Rectangle::new(
            Point { x: 400, y: 20 },
            Size {
                width: 50,
                height: 2,
            },
        )
        .into_styled(PrimitiveStyle::with_fill(Bgr565::WHITE))
        .draw(display)?;

        // Running sounds
        let mut num_oneshots = 0;
        let mut num_looped = 0;

        for (name, looping) in self.playing_sound_names().iter() {
            Text::new(
                name,
                Point {
                    x: if *looping { 400 } else { 50 },
                    y: if *looping { num_looped } else { num_oneshots } * 15 + 40,
                },
                MonoTextStyle::new(&FONT_10X20, Bgr565::WHITE),
            )
            .draw(display)?;

            if *looping {
                num_looped += 1;
            } else {
                num_oneshots += 1;
            }
        }

        // Volume bar

        SoundMode::draw_volume(&self.sound_system, display)?;

        Ok(())
    }
}

impl super::DeviceMode for SoundMode {
    fn button_press(&mut self, note_name: NoteName) -> LightAction {
        if self
            .button_actions
            .contains_key(&ButtonType::Note(note_name))
        {
            let playing = self
                .button_actions
                .get_mut(&ButtonType::Note(note_name))
                .unwrap()
                .execute(&mut self.sound_system);

            if playing == ActionState::FadingOut || playing == ActionState::Stopped {
                println!("Stopping a sound.");
            }
            return LightAction::Reapply;
        }

        return LightAction::None;
    }

    fn control_press(&mut self, control_name: ControlName) -> LightAction {
        match control_name {
            ControlName::Control29 => {
                // Toggle Internal State

                let mut internal_state = self
                    .sound_system
                    .try_lock()
                    .expect("Couldn't lock SoundSystem")
                    .repress_mode;

                match internal_state {
                    crate::sound_system::RepressMode::End => {
                        internal_state = crate::sound_system::RepressMode::Interrupt
                    }
                    crate::sound_system::RepressMode::Interrupt => {
                        internal_state = crate::sound_system::RepressMode::End
                    }
                }

                self.sound_system
                    .try_lock()
                    .expect("Couldn't lock SoundSystem")
                    .repress_mode = internal_state;

                return LightAction::Reapply;
            }
            ControlName::Control20 => {
                return LightAction::Reapply;
            }
            ControlName::Control21 => {
                return LightAction::Reapply;
            }
            ControlName::Control24 => {
                return LightAction::None;
            }
            ControlName::Control25 => {
                return LightAction::None;
            }
        }
    }

    fn apply_button_lights(
        &mut self,
        midiconn: &Arc<Mutex<crate::midi::MidiConnection>>,
        button_values: &HashMap<u8, ButtonType>,
    ) {
        let mut mutex_guard = midiconn.try_lock().expect("Couldn't lock MidiConnection");
        let sound_guard = self
            .sound_system
            .try_lock()
            .expect("Couldn't lock SoundSystem.");

        for (address, name) in button_values {
            match name {
                ButtonType::ControlChange(control_name) => match control_name {
                    ControlName::Control29 => {
                        mutex_guard
                            .send_to_device(&[
                                0b10110000,
                                *address,
                                match sound_guard.repress_mode {
                                    crate::sound_system::RepressMode::End => 127u8,
                                    crate::sound_system::RepressMode::Interrupt => 126u8,
                                },
                            ])
                            .unwrap();
                    }
                    ControlName::Control20 => {
                        mutex_guard
                            .send_to_device(&[0b10110000, *address, 125u8])
                            .unwrap();
                    }
                    ControlName::Control21 => {
                        mutex_guard
                            .send_to_device(&[0b10110000, *address, 123u8])
                            .unwrap();
                    }
                    ControlName::Control24 => {
                        mutex_guard
                            .send_to_device(&[0b10110000, *address, 0u8])
                            .unwrap();
                    }
                    ControlName::Control25 => {
                        mutex_guard
                            .send_to_device(&[0b10110000, *address, 0u8])
                            .unwrap();
                    }
                },
                ButtonType::Note(_note_name) => {
                    if self.button_actions.contains_key(name) {
                        match self.button_actions[name].is_running() {
                            ActionState::None | ActionState::Stopped => {
                                mutex_guard
                                    .send_to_device(&[
                                        0b10010000,
                                        *address,
                                        self.button_actions[name].get_default_color(),
                                    ])
                                    .unwrap();
                            }
                            ActionState::Playing
                            | ActionState::Started
                            | ActionState::FadingIn
                            | ActionState::FadingOut => {
                                mutex_guard
                                    .send_to_device(&[
                                        0b10010000,
                                        *address,
                                        self.button_actions[name].get_active_color(),
                                    ])
                                    .unwrap();
                            }
                        }
                    } else {
                        mutex_guard
                            .send_to_device(&[0b10010000, *address, 0_u8])
                            .unwrap();
                    }
                }
            }
        }
    }

    fn update(&mut self) -> LightAction {
        let mut need_reload = None;

        let mut need_ligh_refresh = LightAction::None;

        if let Some(ref watcher) = self.file_watcher {
            while let Ok(msg) = watcher.try_recv() {
                for path in msg.event.paths {
                    need_reload = Some(path.clone());
                }
            }
        }

        if let Some(changed) = need_reload {
            // stop all sounds
            for (_btn_name, action) in &mut self.button_actions {
                match action {
                    Action::Sound(sound) => {
                        sound.stop();
                    }
                    Action::Command(_) => {}
                }
            }

            // unload all sounds
            self.button_actions.clear();

            // parse new sounds
            self.read_config_impl(&changed);

            need_ligh_refresh = LightAction::ClearAndReapply;
        }

        for (_btn_name, action) in &mut self.button_actions {
            let result = action.update(&mut self.sound_system);

            match result {
                ActionState::None | ActionState::Playing => {}
                ActionState::Stopped
                | ActionState::Started
                | ActionState::FadingIn
                | ActionState::FadingOut => {
                    if need_ligh_refresh == LightAction::None {
                        need_ligh_refresh = LightAction::Reapply;
                    }
                }
            }
        }

        return need_ligh_refresh;
    }

    fn display(&self, display: &mut push2_display::Push2Display) {
        self.display_sounds(display).unwrap();
    }
}
