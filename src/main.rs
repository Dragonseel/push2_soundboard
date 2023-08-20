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

#[cfg(feature = "spotify")]
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
    #[error("Config file not found")]
    ConfigFileNotFound(&'static str),

    #[error("Config file read error")]
    ConfigFileReadError,

    #[error("Ableton Push 2 Midi In not found")]
    NoMidiInFound,

    #[error("Ableton Push 2 Midi Out not found")]
    NoMidiOutFound,

    #[error("System mutex could not be locked")]
    MutexError(&'static str),

    #[error("SoundSystem error")]
    SoundSystemError(&'static str),

    #[error("FileWatcher error")]
    FileWatcher(&'static str),

    #[error(transparent)]
    SpotifyError(#[from] rspotify::ClientError),

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

#[macro_export]
macro_rules! lock_or_return_err {
    ($mutex:ident) => {{
        let guard_res = $mutex.try_lock();
        match guard_res {
            Ok(guard) => guard,
            Err(_error) => {
                return Err(MyError::MutexError(stringify!($mutex)));
            }
        }
    }};
}

async fn run() -> Result<(), MyError> {
    let mut file = File::open("config/devices.ron")?;
    let mut config_string = String::new();
    file.read_to_string(&mut config_string)
        .expect("Could not read config file.");

    let device_config: DeviceConfig =
        ron::de::from_str(&config_string).expect("Could not deserialize DeviceConfig.");

    let mut display = Push2Display::new()?;

    let (push2midi, receiver) =
        MidiConnection::new(&device_config.midi_in, &device_config.midi_out)?;

    let mut push2midi = Arc::new(Mutex::new(push2midi));

    let sound_system = Arc::new(Mutex::new(SoundSystem::new(&device_config.sound_device)?));

    let button_mapping = Arc::new(Mutex::new(
        ButtonMap::new(Arc::clone(&sound_system), &push2midi).await?,
    ));

    lock_or_return_err!(button_mapping).clear_button_lights(&push2midi);
    lock_or_return_err!(button_mapping).apply_button_lights(&push2midi);

    let mut tray = TrayItem::new("Push2Soundboard", tray_item::IconSource::Resource("test"))
        .expect("Could not create tray icon");
    tray.add_label("Soundboard")
        .expect("Could not add label to tray icon.");

    let atomic_flag: Arc<AtomicBool> = Arc::new(AtomicBool::new(false));

    let closure_atomic_flag = Arc::clone(&atomic_flag);

    tray.add_menu_item("Quit", move || {
        closure_atomic_flag.store(true, std::sync::atomic::Ordering::SeqCst);
    })
    .expect("Could not create the tray quit menu entry.");

    let game_loop = GameLoop::new(60, 5)?;
    loop {
        for action in game_loop.actions() {
            match action {
                FrameAction::Tick => {
                    while let Ok(msg) = receiver.try_recv() {
                        match msg {
                            MidiMessage::Btn(address, _value) => {
                                lock_or_return_err!(button_mapping)
                                    .activate_button(address, &push2midi)?
                            }
                            MidiMessage::Volume(change) => {
                                lock_or_return_err!(sound_system).change_volume(change)
                            }
                        }
                    }

                    lock_or_return_err!(button_mapping).update(&mut push2midi);

                    if atomic_flag.load(std::sync::atomic::Ordering::SeqCst) {
                        lock_or_return_err!(button_mapping).clear_button_lights(&push2midi);
                        std::process::exit(0);
                    }
                }

                FrameAction::Render {
                    interpolation: _interpolation,
                } => {
                    display.clear(Bgr565::BLACK)?;

                    lock_or_return_err!(button_mapping).display(&mut display)?;

                    display.flush()?;
                }
            }
        }
    }
}
