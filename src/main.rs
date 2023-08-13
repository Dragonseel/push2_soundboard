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

use embedded_graphics::{pixelcolor::Bgr565, prelude::*};
use std::io::{BufRead, BufReader};
use std::net::TcpListener;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};
use std::{convert::Infallible, fs::File, io::Read};
use tray_item::TrayItem;

use crate::midi::MidiMessage;

mod actions;
mod button_map;
mod device_modes;
mod midi;
mod sound_system;
mod spotify;

#[tokio::main]
async fn main() {
    match run().await {
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

const MAX_VOLUME: u32 = 400;
const DEFAULT_VOLUME: u32 = 100;

#[derive(Deserialize)]
struct DeviceConfig {
    sound_device: String,
    midi_in: String,
    midi_out: String,
}

async fn run() -> Result<(), MyError> {
    let mut file = File::open("config/devices.ron").unwrap();
    let mut config_string = String::new();
    file.read_to_string(&mut config_string)
        .expect("Could not read config file.");

    let device_config: DeviceConfig =
        ron::de::from_str(&config_string).expect("Could not deserialize DeviceConfig.");

    let mut display = Push2Display::new()?;

    println!("Created display.");

    let (push2midi, receiver) =
        MidiConnection::new(&device_config.midi_in, &device_config.midi_out)?;

    println!("Created midi.");

    let mut push2midi = Arc::new(Mutex::new(push2midi));

    let spotify = Arc::new(spotify::Spotify::new().await);
    let sound_system = Arc::new(Mutex::new(SoundSystem::new(&device_config.sound_device)));

    let button_mapping = Arc::new(Mutex::new(ButtonMap::new(
        Arc::clone(&sound_system),
        &push2midi,
    )));
    button_mapping
        .try_lock()
        .unwrap()
        .clear_button_lights(&push2midi);
    button_mapping.try_lock().unwrap().apply_button_lights(&push2midi).unwrap();

    let mut tray =
        TrayItem::new("Push2Soundboard", tray_item::IconSource::Resource("test")).unwrap();
    tray.add_label("Soundboard").unwrap();

    let atomic_flag: Arc<AtomicBool> = Arc::new(AtomicBool::new(false));

    let closure_atomic_flag = Arc::clone(&atomic_flag);

    tray.add_menu_item("Quit", move || {
        closure_atomic_flag.store(true, std::sync::atomic::Ordering::SeqCst);
    })
    .unwrap();

    let game_loop = GameLoop::new(60, 5)?;
    loop {
        for action in game_loop.actions() {
            match action {
                FrameAction::Tick => {
                    while let Ok(msg) = receiver.try_recv() {
                        match msg {
                            MidiMessage::Btn(address, _value) => {
                                button_mapping
                                    .try_lock()
                                    .unwrap()
                                    .activate_button(address, &push2midi)
                                    .await;
                            }
                            MidiMessage::Volume(change) => {
                                sound_system
                                    .try_lock()
                                    .expect("Couldn't lock SoundSystem.")
                                    .change_volume(change);
                            }
                        }
                    }

                    button_mapping.try_lock().unwrap().update(&mut push2midi);

                    if atomic_flag.load(std::sync::atomic::Ordering::SeqCst) {
                        button_mapping
                            .try_lock()
                            .unwrap()
                            .clear_button_lights(&push2midi);
                        std::process::exit(0);
                    }
                }

                FrameAction::Render {
                    interpolation: _interpolation,
                } => {
                    display.clear(Bgr565::BLACK)?;

                    button_mapping.try_lock().unwrap().display(&mut display)?;

                    display.flush()?;
                }
            }
        }
    }
}
