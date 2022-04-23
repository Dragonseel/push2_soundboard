//#![cfg_attr(
//    all(target_os = "windows", not(feature = "console"),),
//   windows_subsystem = "windows"
//)]

use anyhow::Result;
use button_map::ButtonMap;
use midi::MidiConnection;
use sound_system::SoundSystem;
use thiserror::Error;

use midir::{ConnectError, InitError, MidiInput, MidiOutput, PortInfoError, SendError};
// use wayang::{Wayang, WayangError};
use gameloop::{FrameAction, GameLoop, GameLoopError};
use push2_display::*;

use embedded_graphics::{
    mono_font::{iso_8859_13::FONT_10X20, MonoTextStyle},
    pixelcolor::Bgr565,
    prelude::*,
    primitives::{PrimitiveStyle, Rectangle},
    text::Text,
};
use std::sync::{Arc, Mutex};
use std::{convert::Infallible, fs::File, io::Read};
use tray_item::TrayItem;

use crate::midi::MidiMessage;

mod button_map;
mod midi;
mod sound;
mod sound_system;

#[derive(Deserialize)]
struct DeviceConfig {
    sound_device: String,
    midi_in: String,
    midi_out: String,
}

fn main() {
    match run() {
        Ok(_) => (),
        Err(err) => println!("Error: {}", err),
    }
}

#[macro_use]
extern crate serde_derive;

#[derive(Error, Debug)]
pub enum MyError {
    #[error("Ableton Push 2 Midi In not found")]
    NoMidiInFound,

    #[error("Ableton Push 2 Midi Out not found")]
    NoMidiOutFound,

    /// Represents all other cases of `std::io::Error`.
    #[error(transparent)]
    IOError(#[from] std::io::Error),

    #[error(transparent)]
    GameLoopE(#[from] GameLoopError),

    #[error(transparent)]
    MidirError(#[from] ConnectError<MidiInput>),

    #[error(transparent)]
    MidirError2(#[from] InitError),

    // #[error(transparent)]
    // Push2Error(#[from] WayangError),
    #[error(transparent)]
    Push2Error(#[from] Push2DisplayError),

    #[error(transparent)]
    MidiError3(#[from] PortInfoError),

    #[error(transparent)]
    MidirError4(#[from] ConnectError<MidiOutput>),

    #[error(transparent)]
    MidirError5(#[from] SendError),

    #[error(transparent)]
    Infallible(#[from] Infallible),

    #[error(transparent)]
    Other(#[from] anyhow::Error), // source and Display delegate to anyhow::Error
}

fn run() -> Result<(), MyError> {
    let mut file = File::open("config/devices.ron").unwrap();
    let mut config_string = String::new();
    file.read_to_string(&mut config_string)
        .expect("Could not read config file.");

    let device_config: DeviceConfig =
        ron::de::from_str(&config_string).expect("Could not deserialize DeviceConfig.");

    let mut display = Push2Display::new()?;

    let (push2midi, receiver) =
        MidiConnection::new(&device_config.midi_in, &device_config.midi_out)?;

    let mut push2midi = Arc::new(Mutex::new(push2midi));

    let button_mapping = Arc::new(Mutex::new(ButtonMap::new()));
    button_mapping
        .lock()
        .unwrap()
        .clear_button_lights(Arc::clone(&push2midi));

    button_mapping
        .lock()
        .unwrap()
        .read_config("config/example.ron", &mut push2midi);

    let mut sound_system = SoundSystem::new(&device_config.sound_device);

    let mut tray = TrayItem::new("Push2Soundboard", "test").unwrap();
    tray.add_label("Soundboard").unwrap();

    let closure_midi = Arc::clone(&push2midi);
    let closure_buttons = Arc::clone(&button_mapping);

    tray.add_menu_item("Quit", move || {
        closure_buttons
            .lock()
            .unwrap()
            .clear_button_lights(Arc::clone(&closure_midi));
        std::process::exit(0);
    })
    .unwrap();

    button_mapping
        .lock()
        .unwrap()
        .init_control_states(&mut sound_system, Arc::clone(&push2midi));

    let game_loop = GameLoop::new(60, 5)?;
    loop {
        for action in game_loop.actions() {
            match action {
                FrameAction::Tick => {
                    if let Ok(msg) = receiver.try_recv() {
                        match msg {
                            MidiMessage::Btn(address, _value) => {
                                button_mapping.lock().unwrap().activate_button(
                                    address,
                                    &mut sound_system,
                                    Arc::clone(&push2midi),
                                );
                            }
                        }
                    }

                    button_mapping.lock().unwrap().update(&mut push2midi);
                }

                FrameAction::Render {
                    interpolation: _interpolation,
                } => {
                    display.clear(Bgr565::BLACK)?;

                    // One-shots header
                    Text::new(
                        "One-Shots",
                        Point { x: 50, y: 15 },
                        MonoTextStyle::new(&FONT_10X20, Bgr565::WHITE),
                    )
                    .draw(&mut display)?;

                    Rectangle::new(
                        Point { x: 50, y: 20 },
                        Size {
                            width: 90,
                            height: 2,
                        },
                    )
                    .into_styled(PrimitiveStyle::with_fill(Bgr565::WHITE))
                    .draw(&mut display)?;

                    // Loops header
                    Text::new(
                        "Loops",
                        Point { x: 400, y: 15 },
                        MonoTextStyle::new(&FONT_10X20, Bgr565::WHITE),
                    )
                    .draw(&mut display)?;

                    Rectangle::new(
                        Point { x: 400, y: 20 },
                        Size {
                            width: 50,
                            height: 2,
                        },
                    )
                    .into_styled(PrimitiveStyle::with_fill(Bgr565::WHITE))
                    .draw(&mut display)?;

                    let mut num_oneshots = 0;
                    let mut num_looped = 0;

                    for (name, looping) in
                        button_mapping.lock().unwrap().playing_sound_names().iter()
                    {
                        Text::new(
                            name,
                            Point {
                                x: if *looping { 400 } else { 50 },
                                y: if *looping { num_looped } else { num_oneshots } * 15 + 40,
                            },
                            MonoTextStyle::new(&FONT_10X20, Bgr565::WHITE),
                        )
                        .draw(&mut display)?;

                        if *looping {
                            num_looped += 1;
                        } else {
                            num_oneshots += 1;
                        }
                    }

                    display.flush()?;
                }
            }
        }
    }
}
