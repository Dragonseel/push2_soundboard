use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use push2_display::Push2Display;

use crate::{
    button_map::{ButtonType, ControlName, NoteName},
    midi::MidiConnection,
    MyError,
};

pub mod sound_mode;

#[cfg(feature = "spotify")]
pub mod spotify_mode;

#[derive(PartialEq)]
pub enum LightAction {
    None,
    Reapply,
    ClearAndReapply,
}

pub trait DeviceMode {
    fn button_press(&mut self, note_name: NoteName) -> Result<LightAction, MyError>;

    fn control_press(&mut self, control_name: ControlName) -> Result<LightAction, MyError>;

    fn apply_button_lights(
        &mut self,
        midiconn: &Arc<Mutex<MidiConnection>>,
        button_values: &HashMap<u8, ButtonType>,
    ) -> Result<(), MyError>;

    fn update(&mut self) -> Result<LightAction, MyError>;

    fn display(&self, display: &mut Push2Display) -> Result<(), MyError>;
}
