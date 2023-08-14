use std::{
    io::{self, Read},
    path::PathBuf,
    sync::{Arc, Mutex},
};

use rodio::Sink;

use crate::sound_system::SoundSystem;

use super::ActionState;

#[derive(Deserialize)]
pub struct SingleSoundConfig {}

pub struct Sound {
    // Runtime Data
    state: ActionState,
    volume: f32,
    sink: Option<Sink>,
    sound_data: Arc<Vec<u8>>,

    // Settings
    name: String,
    gain: f32,
    fade_out: bool,
    fade_in: bool,
    pub looped: bool,
}

impl AsRef<[u8]> for Sound {
    fn as_ref(&self) -> &[u8] {
        &self.sound_data
    }
}

impl Sound {
    pub fn load(
        filename: String,
        looped: bool,
        fade_in: bool,
        fade_out: bool,
        gain: f32,
    ) -> io::Result<Sound> {
        let path = PathBuf::from(filename);

        let name = if let Some(stem) = path.file_stem() {
            if let Some(file_name) = stem.to_str() {
                String::from(file_name)
            } else {
                String::from("Unknown")
            }
        } else {
            String::from("Unknown")
        };

        use std::fs::File;
        let mut buf = Vec::new();
        let mut file = File::open(path)?;
        file.read_to_end(&mut buf)?;

        Ok(Sound {
            name,
            sound_data: Arc::new(buf),
            sink: None,
            looped,
            fade_in,
            fade_out,
            state: ActionState::None,
            gain,
            volume: 0.0f32,
        })
    }

    pub fn cursor(self: &Self) -> io::Cursor<Sound> {
        io::Cursor::new(Sound {
            name: self.name.clone(),
            sound_data: self.sound_data.clone(),
            sink: None,
            looped: self.looped,
            fade_in: self.fade_in,
            fade_out: self.fade_out,
            state: self.state,
            gain: self.gain,
            volume: self.volume,
        })
    }

    fn decoder(self: &Self) -> rodio::Decoder<io::Cursor<Sound>> {
        rodio::Decoder::new(self.cursor()).unwrap()
    }

    fn looped_decoder(self: &Self) -> rodio::decoder::LoopedDecoder<io::Cursor<Sound>> {
        rodio::Decoder::new_looped(self.cursor()).unwrap()
    }

    fn create_sink_and_append(&mut self, sound_system: &Arc<Mutex<SoundSystem>>) -> ActionState {
        let sink = {
            let guard_res = sound_system.try_lock();

            match guard_res {
                Ok(mut guard) => guard.get_sink(),
                Err(error) => {
                    panic!("Couldn't lock SoundSystem, {:?}", error);
                }
            }
        };

        let (new_state, volume) = self.append_to_sink(&sink, sound_system);
        self.state = new_state;
        self.volume = volume;

        self.sink = Some(sink);

        self.state
    }

    fn append_to_sink(
        &self,
        sink: &Sink,
        sound_system: &Arc<Mutex<SoundSystem>>,
    ) -> (ActionState, f32) {
        if self.looped {
            sink.append(self.looped_decoder());
        } else {
            sink.append(self.decoder());
        }

        if self.fade_in {
            sink.set_volume(0.0);

            (ActionState::FadingIn, 0.0)
        } else {
            sink.set_volume(
                sound_system
                    .try_lock()
                    .expect("Couldn't lock SoundSystem")
                    .get_volume_factor()
                    * self.gain,
            );

            (ActionState::Playing, self.gain)
        }
    }

    pub fn get_name(&self) -> String {
        self.name.clone()
    }

    pub fn is_playing(&self) -> bool {
        return self.sink.is_some();
    }

    pub fn stop(&mut self) {
        if let Some(ref mut sink) = self.sink {
            sink.stop();
        }
        self.sink = None;
    }

    pub fn play(&mut self, sound_system: &Arc<Mutex<SoundSystem>>) -> ActionState {
        if let Some(sink) = &self.sink {
            if sink.empty() {
                let (new_state, volume) = self.append_to_sink(sink, sound_system);
                self.state = new_state;
                self.volume = volume;

                return new_state;
            } else {
                let repress_mode = sound_system
                    .try_lock()
                    .expect("Could not lock soundsystem")
                    .repress_mode;

                match repress_mode {
                    crate::sound_system::RepressMode::End => {
                        if self.fade_out {
                            self.state = ActionState::FadingOut;
                            return self.state;
                        } else {
                            sink.stop();
                            self.sink = None;
                            self.state = ActionState::Stopped;
                            return self.state;
                        }
                    }
                    crate::sound_system::RepressMode::Interrupt => {
                        sink.stop();
                        self.sink = None;
                        return self.create_sink_and_append(sound_system);
                    }
                }
            }
        } else {
            return self.create_sink_and_append(sound_system);
        }
    }

    pub fn update(&mut self, sound_system: &Arc<Mutex<SoundSystem>>) -> ActionState {
        if self.state == ActionState::Stopped {
            // We stopped last update, and now everything is over.
            self.state = ActionState::None;
            return self.state;
        }

        if let Some(sink) = &self.sink {
            if sink.empty() {
                self.state = ActionState::Stopped;
            } else {
                const FRAMERATE: f32 = 60.0f32;
                const INCREASE_PER_FRAME: f32 = 1.5f32;

                match self.state {
                    ActionState::FadingIn => {
                        self.volume = f32::min(
                            self.volume + (self.gain / FRAMERATE) * INCREASE_PER_FRAME,
                            self.gain,
                        );

                        if self.volume >= self.gain {
                            self.state = ActionState::Playing;
                        }
                    }
                    ActionState::FadingOut => {
                        self.volume = f32::max(
                            self.volume - (self.gain / FRAMERATE) * INCREASE_PER_FRAME,
                            0.0,
                        );
                        if self.volume <= 0.0 {
                            self.state = ActionState::Stopped;
                        }
                    }
                    _ => {
                        // No need to act right now
                    }
                }

                sink.set_volume(
                    sound_system
                        .try_lock()
                        .expect("Couldn't lock SoundSystem.")
                        .get_volume_factor()
                        * self.volume,
                );
            }
        } else {
            // We have no sink.
            self.state = ActionState::None;
        }

        if self.sink.is_some() && self.state == ActionState::Stopped {
            // Nothing playing but still a sink
            // Take it out of option, stop and drop it.
            self.sink.take().unwrap().stop();
        }

        return self.state;
    }

    pub fn is_running(&self) -> ActionState {
        self.state
    }
}
