use std::{
    collections::HashMap,
    fs::File,
    io::Read,
    sync::{Arc, Mutex},
};

use push2_display::Push2Display;

#[rustfmt::skip]
mod unformatted {

    #[derive(PartialEq, Eq, Hash, Clone, Copy, Deserialize, Debug)]
    pub enum ButtonType {
        ControlChange(ControlName),
        Note(NoteName)
    }


    #[derive(PartialEq, Eq, Hash, Clone, Copy, Deserialize, Debug)]
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

    #[derive(PartialEq, Eq, Hash, Clone, Copy, Deserialize, Debug)]
    pub enum ControlName {
        Control29,
        Control20,
        Control21,
        Control24,
        Control25,
    }
}

use crate::{
    device_modes::{sound_mode::SoundMode, DeviceMode, LightAction},
    midi::MidiConnection,
    sound_system::SoundSystem,
    MyError,
};

#[cfg(feature = "spotify")]
use crate::device_modes::spotify_mode::SpotifyMode;

pub use unformatted::{ButtonType, ControlName, NoteName};

pub struct ButtonMap {
    button_values: HashMap<u8, ButtonType>,

    device_modes: Vec<Box<dyn DeviceMode>>,
    current_mode: usize,
}

impl ButtonMap {
    pub fn new(
        sound_system: Arc<Mutex<SoundSystem>>,
        midiconn: &Arc<Mutex<MidiConnection>>,
    ) -> Result<ButtonMap, MyError> {
        let file = File::open("config/buttonvalues.ron");

        let mut file = match file {
            Ok(handle) => handle,
            Err(_err) => {
                return Err(MyError::ConfigFileNotFound("Button values"));
            }
        };

        let mut config_string = String::new();

        if file.read_to_string(&mut config_string).is_err() {
            return Err(MyError::ConfigFileReadError);
        }

        let button_values: HashMap<u8, ButtonType> =
            ron::de::from_str(&config_string).expect("Could not deserialize SoundConfig.");

        let mut device_modes: Vec<Box<dyn DeviceMode>> = Vec::new();
        device_modes.push(Box::new(SoundMode::new(sound_system)?));

        #[cfg(feature = "spotify")]
        device_modes.push(Box::new(SpotifyMode::new()?));

        device_modes[0].apply_button_lights(midiconn, &button_values)?;

        Ok(ButtonMap {
            button_values: button_values,
            device_modes,
            current_mode: 0_usize,
        })
    }

    pub fn activate_button(
        &mut self,
        address: u8,
        midiconn: &Arc<Mutex<MidiConnection>>,
    ) -> Result<(), MyError> {
        let mut light_action = LightAction::None;

        if self.button_values.contains_key(&address) {
            match &self.button_values[&address] {
                ButtonType::ControlChange(control_name) => {
                    let mut control_change = false;
                    if *control_name == ControlName::Control20 {
                        self.current_mode = 0;
                        control_change = true;
                    } else if *control_name == ControlName::Control21 {
                        if self.device_modes.len() > 1 {
                            self.current_mode = 1;
                        }

                        control_change = true;
                    }

                    light_action =
                        self.device_modes[self.current_mode].control_press(*control_name)?;

                    if control_change {
                        light_action = LightAction::ClearAndReapply;
                    }
                }
                ButtonType::Note(note_name) => {
                    light_action = self.device_modes[self.current_mode].button_press(*note_name)?
                }
            }
        }

        match light_action {
            LightAction::None => {}
            LightAction::Reapply => {
                self.device_modes[self.current_mode]
                    .apply_button_lights(midiconn, &self.button_values)?;
            }
            LightAction::ClearAndReapply => {
                self.clear_button_lights(midiconn)?;
                self.device_modes[self.current_mode]
                    .apply_button_lights(midiconn, &self.button_values)?;
            }
        }

        Ok(())
    }

    pub fn update(&mut self, midiconn: &mut Arc<Mutex<MidiConnection>>) -> Result<(), MyError> {
        let light_action: LightAction = self.device_modes[self.current_mode].update()?;

        match light_action {
            LightAction::None => {}
            LightAction::Reapply => {
                self.device_modes[self.current_mode]
                    .apply_button_lights(midiconn, &self.button_values)?;
            }
            LightAction::ClearAndReapply => {
                self.clear_button_lights(midiconn)?;
                self.device_modes[self.current_mode]
                    .apply_button_lights(midiconn, &self.button_values)?;
            }
        }

        Ok(())
    }

    pub fn clear_button_lights(
        &mut self,
        midiconn: &Arc<Mutex<MidiConnection>>,
    ) -> Result<(), MyError> {
        let mutex_guard = midiconn.try_lock();

        let mut mutex_guard = match mutex_guard {
            Ok(value) => value,
            Err(_err) => {
                return Err(MyError::MutexError("Could not lock midi"));
            }
        };

        for (address, _name) in &self.button_values {
            mutex_guard.send_to_device(&[
                match _name {
                    ButtonType::ControlChange(_) => 0b10110000,
                    ButtonType::Note(_) => 0b10010000,
                },
                *address,
                0u8,
            ])?
        }

        Ok(())
    }

    pub fn apply_button_lights(
        &mut self,
        midiconn: &Arc<Mutex<MidiConnection>>,
    ) -> Result<(), MyError> {
        // let current device-mode update the lights
        self.device_modes[self.current_mode].apply_button_lights(midiconn, &self.button_values)?;

        Ok(())
    }

    pub fn display(&self, display: &mut Push2Display) -> Result<(), MyError> {
        self.device_modes[self.current_mode].display(display)?;

        Ok(())
    }
}
