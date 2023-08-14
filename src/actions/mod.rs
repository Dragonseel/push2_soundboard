use std::sync::{Arc, Mutex};

use crate::{
    button_map::ButtonType,
    sound_system::SoundSystem,
};

use self::{
    command::Command,
    sound::Sound,
};

pub mod command;
pub mod sound;

#[derive(Deserialize)]
pub enum ActionConfig {
    SoundConfig {
        button: ButtonType,
        path: String,
        looping: bool,
        fade_in: bool,
        fade_out: bool,
        gain: f32,
    },
    CommandConfig {
        button: ButtonType,
        command: String,
        args: Vec<String>,
    },
}

pub enum Action {
    Sound(Sound),
    Command(Command),
}

#[derive(PartialEq, Clone, Copy)]
pub enum ActionState {
    None,
    Stopped,
    Started,
    FadingIn,
    FadingOut,
    Playing,
}

impl Action {
    pub fn get_default_color(&self) -> u8 {
        match self {
            Action::Sound(sound) => {
                if sound.looped {
                    return 125_u8;
                } else {
                    return 56_u8;
                }
            }
            Action::Command(_command) => {
                return 72_u8;
            }
        }
    }

    pub fn get_active_color(&self) -> u8 {
        match self {
            Action::Sound(sound) => {
                if sound.looped {
                    return 127u8;
                } else {
                    return 126u8;
                }
            }
            Action::Command(_command) => {
                return 123_u8;
            }
        }
    }

    pub fn execute(&mut self, sound_system: &mut Arc<Mutex<SoundSystem>>) -> ActionState {
        match self {
            Action::Sound(sound) => sound.play(sound_system),
            Action::Command(command) => command.execute(),
        }
    }

    pub fn update(&mut self, sound_system: &mut Arc<Mutex<SoundSystem>>) -> ActionState {
        match self {
            Action::Sound(sound) => {
                return sound.update(sound_system);
            }
            Action::Command(cmd) => {
                return cmd.update();
            }
        }
    }

    pub fn is_running(&self) -> ActionState {
        match self {
            Action::Sound(sound) => sound.is_running(),
            Action::Command(cmd) => cmd.is_running(),
        }
    }
}
