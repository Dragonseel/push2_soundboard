use std::{
    io::{self, Read},
    sync::Arc,
};

use rodio::Sink;

use crate::sound_system::SoundSystem;

pub struct Sound {
    sound_data: Arc<Vec<u8>>,
    sink: Option<Sink>,
    pub looped: bool,
    fade_in: bool,
    fade_out: bool,
    fading_in: bool,
    fading_out: bool,
    gain: f32,
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
        use std::fs::File;
        let mut buf = Vec::new();
        let mut file = File::open(filename)?;
        file.read_to_end(&mut buf)?;
        Ok(Sound {
            sound_data: Arc::new(buf),
            sink: None,
            looped,
            fade_in,
            fade_out,
            fading_in: false,
            fading_out: false,
            gain,
        })
    }

    pub fn cursor(self: &Self) -> io::Cursor<Sound> {
        io::Cursor::new(Sound {
            sound_data: self.sound_data.clone(),
            sink: None,
            looped: self.looped,
            fade_in: self.fade_in,
            fade_out: self.fade_out,
            fading_in: self.fading_in,
            fading_out: self.fading_out,
            gain: self.gain,
        })
    }

    fn decoder(self: &Self) -> rodio::Decoder<io::Cursor<Sound>> {
        rodio::Decoder::new(self.cursor()).unwrap()
    }

    fn looped_decoder(self: &Self) -> rodio::decoder::LoopedDecoder<io::Cursor<Sound>> {
        rodio::Decoder::new_looped(self.cursor()).unwrap()
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
                if self.looped {
                    sink.append(self.looped_decoder());
                } else {
                    sink.append(self.decoder());
                }

                if self.fade_in {
                    sink.set_volume(0.0);
                    self.fading_in = true;
                } else {
                    sink.set_volume(self.gain);
                }

                return true;
            } else {
                match sound_system.get_repress_mode() {
                    crate::sound_system::RepressMode::End => {
                        if self.fade_out {
                            self.fading_out = true;
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
                        let sink = sound_system.get_sink();

                        if self.looped {
                            sink.append(self.looped_decoder());
                        } else {
                            sink.append(self.decoder());
                        }

                        if self.fade_in {
                            sink.set_volume(0.0);
                            self.fading_in = true;
                        } else {
                            sink.set_volume(self.gain);
                        }

                        self.sink = Some(sink);
                        return true;
                    }
                }
            }
        } else {
            let sink = sound_system.get_sink();

            if self.looped {
                sink.append(self.looped_decoder());
            } else {
                sink.append(self.decoder());
            }

            if self.fade_in {
                sink.set_volume(0.0);
                self.fading_in = true;
            } else {
                sink.set_volume(self.gain);
            }

            self.sink = Some(sink);
            return true;
        }
    }

    pub fn update(&mut self) -> bool {
        let mut playing;
        if let Some(sink) = &self.sink {
            if sink.empty() {
                playing = false;
            } else {
                playing = true;

                if self.fading_in {
                    sink.set_volume(f32::min(
                        sink.volume() + (self.gain / 60.0) * 1.5,
                        self.gain,
                    ));
                    if sink.volume() >= self.gain {
                        self.fading_in = false;
                    }
                }

                if self.fading_out {
                    self.fading_in = false;
                    sink.set_volume(f32::max(sink.volume() - (self.gain / 60.0) * 1.5, 0.0));
                    if sink.volume() <= 0.0 {
                        self.fading_out = false;
                        playing = false;
                    }
                }
            }
        } else {
            playing = false;
        }

        if self.sink.is_some() && !playing {
            self.sink.take().unwrap().stop();
        }

        playing
    }
}
