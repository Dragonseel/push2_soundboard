use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use push2_display::Push2Display;

use crate::{
    button_map::{ButtonType, ControlName, NoteName},
    midi::MidiConnection,
};

pub mod sound_mode;
pub mod spotify_mode;

#[derive(PartialEq)]
pub enum LightAction {
    None,
    Reapply,
    ClearAndReapply,
}

pub trait DeviceMode {
    fn button_press(&mut self, note_name: NoteName) -> LightAction;

    fn control_press(&mut self, control_name: ControlName) -> LightAction;

    fn apply_button_lights(
        &mut self,
        midiconn: &Arc<Mutex<MidiConnection>>,
        button_values: &HashMap<u8, ButtonType>,
    );

    fn update(&mut self) -> LightAction;

    fn display(&self, display: &mut Push2Display);
}
