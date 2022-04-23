use std::{
    io::{self, Read},
    path::PathBuf,
    sync::Arc,
};

use rodio::Sink;

use crate::sound_system::SoundSystem;

#[derive(PartialEq, Clone, Copy)]
enum SoundState {
    Stopped,
    FadingIn,
    FadingOut,
    Playing,
}

pub struct Sound {
    // Runtime Data
    state: SoundState,
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
            state: SoundState::Stopped,
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

    fn create_sink_and_append(&mut self, sound_system: &mut SoundSystem) {
        let sink = sound_system.get_sink();

        let (new_state, volume) = self.append_to_sink(&sink, sound_system);
        self.state = new_state;
        self.volume = volume;

        self.sink = Some(sink);
    }

    fn append_to_sink(&self, sink: &Sink, sound_system: &mut SoundSystem) -> (SoundState, f32) {
        if self.looped {
            sink.append(self.looped_decoder());
        } else {
            sink.append(self.decoder());
        }

        if self.fade_in {
            sink.set_volume(0.0);

            (SoundState::FadingIn, 0.0)
        } else {
            sink.set_volume(sound_system.get_volume_factor() * self.gain);

            (SoundState::Playing, self.gain)
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

    pub fn play(&mut self, sound_system: &mut SoundSystem) -> bool {
        if let Some(sink) = &self.sink {
            if sink.empty() {
                let (new_state, volume) = self.append_to_sink(sink, sound_system);
                self.state = new_state;
                self.volume = volume;

                return true;
            } else {
                match sound_system.repress_mode {
                    crate::sound_system::RepressMode::End => {
                        if self.fade_out {
                            self.state = SoundState::FadingOut;
                            return true;
                        } else {
                            sink.stop();
                            self.sink = None;
                            return false;
                        }
                    }
                    crate::sound_system::RepressMode::Interrupt => {
                        sink.stop();
                        self.sink = None;
                        self.create_sink_and_append(sound_system);
                        return true;
                    }
                }
            }
        } else {
            self.create_sink_and_append(sound_system);
            return true;
        }
    }

    pub fn update(&mut self, sound_system: &mut SoundSystem) -> bool {
        if let Some(sink) = &self.sink {
            if sink.empty() {
                self.state = SoundState::Stopped;
            } else {
                const FRAMERATE: f32 = 60.0f32;
                const INCREASE_PER_FRAME: f32 = 1.5f32;

                match self.state {
                    SoundState::FadingIn => {
                        self.volume = f32::min(
                            self.volume + (self.gain / FRAMERATE) * INCREASE_PER_FRAME,
                            self.gain,
                        );

                        if self.volume >= self.gain {
                            self.state = SoundState::Playing;
                        }
                    }
                    SoundState::FadingOut => {
                        self.volume = f32::max(
                            self.volume - (self.gain / FRAMERATE) * INCREASE_PER_FRAME,
                            0.0,
                        );
                        if self.volume <= 0.0 {
                            self.state = SoundState::Stopped;
                        }
                    }
                    _ => {
                        // No need to act right now
                    }
                }

                sink.set_volume(sound_system.get_volume_factor() * self.volume);
            }
        } else {
            self.state = SoundState::Stopped;
        }

        if self.sink.is_some() && self.state == SoundState::Stopped {
            // Nothing playing but still a sink
            // Take it out of option, stop and drop it.
            self.sink.take().unwrap().stop();

            return false;
        } else {
            return true;
        }
    }
}
