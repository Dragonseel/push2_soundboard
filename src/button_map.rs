use std::{
    collections::HashMap,
    fs::File,
    io::Read,
    path::Path,
    sync::{
        mpsc::{channel, Receiver},
        Arc, Mutex,
    },
    time::Duration,
};

#[rustfmt::skip]
mod unformatted {

    #[derive(PartialEq, Eq, Hash, Clone, Copy, Deserialize)]
    pub enum ButtonType {
        ControlChange(ControlName),
        Note(NoteName)
    }


    #[derive(PartialEq, Eq, Hash, Clone, Copy, Deserialize)]
    pub enum NoteName {
        Pad0x7, Pad1x7, Pad2x7, Pad3x7, Pad4x7, Pad5x7, Pad6x7, Pad7x7,
        Pad0x6, Pad1x6, Pad2x6, Pad3x6, Pad4x6, Pad5x6, Pad6x6, Pad7x6,
        Pad0x5, Pad1x5, Pad2x5, Pad3x5, Pad4x5, Pad5x5, Pad6x5, Pad7x5,
        Pad0x4, Pad1x4, Pad2x4, Pad3x4, Pad4x4, Pad5x4, Pad6x4, Pad7x4,
        Pad0x3, Pad1x3, Pad2x3, Pad3x3, Pad4x3, Pad5x3, Pad6x3, Pad7x3,
        Pad0x2, Pad1x2, Pad2x2, Pad3x2, Pad4x2, Pad5x2, Pad6x2, Pad7x2,
        Pad0x1, Pad1x1, Pad2x1, Pad3x1, Pad4x1, Pad5x1, Pad6x1, Pad7x1,
        Pad0x0, Pad1x0, Pad2x0, Pad3x0, Pad4x0, Pad5x0, Pad6x0, Pad7x0,
    }

    #[derive(PartialEq, Eq, Hash, Clone, Copy, Deserialize)]
    pub enum ControlName {
        Control01,
    }
}

use crate::{
    actions::Action,
    actions::{sound::Sound, ActionConfig, command::Command},
    midi::MidiConnection,
    sound_system::SoundSystem,
};
use notify::{watcher, DebouncedEvent, ReadDirectoryChangesWatcher, Watcher};
pub use unformatted::{ButtonType, ControlName, NoteName};

#[derive(Deserialize)]
struct ActionConfigs {
    actions: Vec<ActionConfig>,
}

pub struct ButtonMap {
    button_values: HashMap<u8, ButtonType>,
    button_actions: HashMap<ButtonType, Action>,
    file_watcher: Option<Receiver<DebouncedEvent>>,
    file_watcher_intern: Option<ReadDirectoryChangesWatcher>,
}

impl ButtonMap {
    pub fn new() -> ButtonMap {
        let mut file = File::open("config/buttonvalues.ron").unwrap();
        let mut config_string = String::new();
        file.read_to_string(&mut config_string)
            .expect("Could not read config file.");

        let button_values: HashMap<u8, ButtonType> =
            ron::de::from_str(&config_string).expect("Could not deserialize SoundConfig.");

        ButtonMap {
            button_values: button_values,
            button_actions: HashMap::new(),
            file_watcher: None,
            file_watcher_intern: None,
        }
    }

    pub fn add_action(
        &mut self,
        button: ButtonType,
        action: Action,
        midiconn: &mut Arc<Mutex<MidiConnection>>,
    ) {
        if !self.button_actions.contains_key(&button) {
            self.button_actions.insert(button, action);
        } else {
            *self.button_actions.get_mut(&button).unwrap() = action;
        }

        let address = self
            .button_values
            .iter()
            .find_map(|(key, &val)| if val == button { Some(key) } else { None })
            .unwrap();

        midiconn
            .lock()
            .unwrap()
            .send_to_device(&[
                0b10010001,
                *address,
                self.button_actions[&button].get_default_color(),
            ])
            .unwrap();
    }

    pub fn activate_button(
        &mut self,
        address: u8,
        sound_system: &mut SoundSystem,
        midiconn: Arc<Mutex<MidiConnection>>,
    ) {
        if self.button_values.contains_key(&address) {
            match &self.button_values[&address] {
                ButtonType::ControlChange(control) => {
                    match control {
                        ControlName::Control01 => {
                            // Toggle Internal State

                            let mut internal_state = sound_system.repress_mode;

                            match internal_state {
                                crate::sound_system::RepressMode::End => {
                                    internal_state = crate::sound_system::RepressMode::Interrupt
                                }
                                crate::sound_system::RepressMode::Interrupt => {
                                    internal_state = crate::sound_system::RepressMode::End
                                }
                            }

                            sound_system.repress_mode = internal_state;

                            midiconn
                                .lock()
                                .unwrap()
                                .send_to_device(&[
                                    0b10110000,
                                    address,
                                    match internal_state {
                                        crate::sound_system::RepressMode::End => 127u8,
                                        crate::sound_system::RepressMode::Interrupt => 126u8,
                                    },
                                ])
                                .unwrap();
                        }
                    }
                }
                ButtonType::Note(_note) => {
                    if self
                        .button_actions
                        .contains_key(&self.button_values[&address])
                    {
                        let _playing = self
                            .button_actions
                            .get_mut(&self.button_values[&address])
                            .unwrap()
                            .execute(sound_system);

                        midiconn
                            .lock()
                            .unwrap()
                            .send_to_device(&[
                                0b10010000,
                                address,
                                self.button_actions
                                    .get_mut(&self.button_values[&address])
                                    .unwrap()
                                    .get_active_color(),
                            ])
                            .unwrap();
                    }
                }
            }
        }
    }

    pub fn update(
        &mut self,
        sound_system: &mut SoundSystem,
        midiconn: &mut Arc<Mutex<MidiConnection>>,
    ) {
        let mut need_reload = None;

        if let Some(ref watcher) = self.file_watcher {
            while let Ok(msg) = watcher.try_recv() {
                match msg {
                    DebouncedEvent::Write(path) => {
                        need_reload = Some(path);
                    }
                    _ => (),
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

            self.clear_button_lights(Arc::clone(midiconn));

            // parse new sounds
            self.read_config_impl(&changed, midiconn)
        }

        for (btn_name, action) in &mut self.button_actions {
            match action {
                Action::Sound(sound) => {
                    if !sound.update(sound_system) {
                        let address = self
                            .button_values
                            .iter()
                            .find_map(|(key, &val)| if val == *btn_name { Some(key) } else { None })
                            .unwrap();

                        midiconn
                            .lock()
                            .unwrap()
                            .send_to_device(&[
                                0b10010000,
                                *address,
                                if sound.looped { 125 } else { 62u8 },
                            ])
                            .unwrap();
                    }
                }
                Action::Command(_) => {}
            }
        }
    }

    pub fn clear_button_lights(&mut self, midiconn: Arc<Mutex<MidiConnection>>) {
        for (address, _name) in &self.button_values {
            midiconn
                .lock()
                .unwrap()
                .send_to_device(&[
                    match _name {
                        ButtonType::ControlChange(_) => 0b10110000,
                        ButtonType::Note(_) => 0b10010000,
                    },
                    *address,
                    0u8,
                ])
                .unwrap();
        }
    }

    pub fn init_control_states(
        &mut self,
        sound_system: &mut SoundSystem,
        midiconn: Arc<Mutex<MidiConnection>>,
    ) {
        /* ControlName::Control01  =>  Repress Mode */
        {
            midiconn
                .lock()
                .unwrap()
                .send_to_device(&[
                    0b10110000,
                    29,
                    match sound_system.repress_mode {
                        crate::sound_system::RepressMode::End => 127u8,
                        crate::sound_system::RepressMode::Interrupt => 126u8,
                    },
                ])
                .unwrap();
        }
    }

    fn read_config_impl(&mut self, path: &Path, midiconn: &mut Arc<Mutex<MidiConnection>>) {
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
                    midiconn,
                ),
                ActionConfig::CommandConfig { button, command, args } => {
                    self.add_action(button, Action::Command(Command::new(command, args)), midiconn)
                }
            }
        }
    }

    pub fn read_config(&mut self, path: &str, midiconn: &mut Arc<Mutex<MidiConnection>>) {
        let (tx, rx) = channel();

        let mut watcher = watcher(tx, Duration::from_secs(2)).unwrap();

        watcher
            .watch(path, notify::RecursiveMode::NonRecursive)
            .unwrap();

        self.read_config_impl(Path::new(path), midiconn);

        self.file_watcher = Some(rx);
        self.file_watcher_intern = Some(watcher);
    }

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
}
