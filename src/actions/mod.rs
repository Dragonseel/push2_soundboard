use crate::{button_map::ButtonType, sound_system::SoundSystem};

use self::{
    command::{Command, SingleCommandConfig},
    sound::{SingleSoundConfig, Sound},
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
            Action::Command(command) => {
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
            Action::Command(command) => {
                return 123_u8;
            }
        }
    }

    pub fn execute(&mut self, sound_system: &mut SoundSystem) -> bool {
        match self {
            Action::Sound(sound) => sound.play(sound_system),
            Action::Command(command) => command.execute(),
        }
    }
}
