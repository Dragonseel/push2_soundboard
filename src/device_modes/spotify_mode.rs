use std::{
    sync::mpsc::{channel, Receiver, Sender},
    thread,
};

use embedded_graphics::{
    mono_font::{iso_8859_13::FONT_10X20, MonoTextStyle},
    pixelcolor::Bgr565,
    prelude::{Point, RgbColor},
    text::Text,
    Drawable,
};
use rspotify::model::SimplifiedPlaylist;

use crate::{
    button_map::EncoderName,
    spotify::{self},
    MyError,
};

enum Query {
    CurrentSong(Option<String>),
    Play,
    Pause,
    Skip,
    CurrentUserPlaylists(Option<Vec<SimplifiedPlaylist>>),
    PlayPlaylist(Option<SimplifiedPlaylist>),
}

pub struct SpotifyMode {
    last_updated: std::time::Instant,
    _worker_thread: std::thread::JoinHandle<()>,

    sender: Sender<Query>,
    receiver: Receiver<Query>,

    playing_song: Option<String>,
    playlists: Option<Vec<SimplifiedPlaylist>>,

    selected_playlist: usize,
}

impl SpotifyMode {
    pub fn new() -> Result<SpotifyMode, MyError> {
        let spotify = spotify::Spotify::new()?;

        let (thread_sender, main_receiver) = channel();
        let (main_sender, thread_receiver) = channel();

        let handle = thread::spawn(move || {
            while let Ok(msg) = thread_receiver.recv() {
                match msg {
                    Query::CurrentSong(_) => {
                        let song = spotify.get_current_song();
                        thread_sender
                            .send(Query::CurrentSong(Some(song)))
                            .expect("Spotify worker thread: Could not send query.");
                    }
                    Query::Play => {
                        spotify
                            .play()
                            .expect("Spotify worker thread: Could not spotify play.");
                        thread_sender
                            .send(Query::Play)
                            .expect("Spotify worker thread: Could not send query.");
                    }
                    Query::Pause => {
                        spotify
                            .pause()
                            .expect("Spotify worker thread: Could not spotify pause.");
                        thread_sender
                            .send(Query::Pause)
                            .expect("Spotify worker thread: Could not send query.");
                    }
                    Query::Skip => {
                        spotify
                            .skip()
                            .expect("Spotify worker thread: Could not spotify skip.");
                        thread_sender
                            .send(Query::Skip)
                            .expect("Spotify worker thread: Could not send query.");
                    }
                    Query::CurrentUserPlaylists(_) => {
                        let playlists = spotify
                            .get_user_playlists()
                            .expect("Spotify worker thread: Could not get playlists.");
                        thread_sender
                            .send(Query::CurrentUserPlaylists(Some(playlists)))
                            .expect("Spotify worker thread: Could not send query.");
                    }
                    Query::PlayPlaylist(playlist) => {
                        if let Some(playlist) = playlist {
                            spotify
                                .play_playlist(playlist)
                                .expect("Spotify worker thread: Could not play playlist");
                        }
                        thread_sender.send(Query::PlayPlaylist(None)).expect("Spotify worker thread: could not send query.");
                    }
                }
            }

            return ();
        });

        Ok(SpotifyMode {
            last_updated: std::time::Instant::now(),
            _worker_thread: handle,
            playing_song: None,
            playlists: None,
            selected_playlist: 0,
            sender: main_sender,
            receiver: main_receiver,
        })
    }
}

impl super::DeviceMode for SpotifyMode {
    fn button_press(
        &mut self,
        note_name: crate::button_map::NoteName,
    ) -> Result<super::LightAction, MyError> {
        match note_name {
            crate::button_map::NoteName::Pad0x7 => (),
            crate::button_map::NoteName::Pad1x7 => (),
            crate::button_map::NoteName::Pad2x7 => (),
            crate::button_map::NoteName::Pad3x7 => (),
            crate::button_map::NoteName::Pad4x7 => (),
            crate::button_map::NoteName::Pad5x7 => (),
            crate::button_map::NoteName::Pad6x7 => (),
            crate::button_map::NoteName::Pad7x7 => (),
            crate::button_map::NoteName::Pad0x6 => (),
            crate::button_map::NoteName::Pad1x6 => (),
            crate::button_map::NoteName::Pad2x6 => (),
            crate::button_map::NoteName::Pad3x6 => (),
            crate::button_map::NoteName::Pad4x6 => (),
            crate::button_map::NoteName::Pad5x6 => (),
            crate::button_map::NoteName::Pad6x6 => (),
            crate::button_map::NoteName::Pad7x6 => (),
            crate::button_map::NoteName::Pad0x5 => (),
            crate::button_map::NoteName::Pad1x5 => (),
            crate::button_map::NoteName::Pad2x5 => (),
            crate::button_map::NoteName::Pad3x5 => (),
            crate::button_map::NoteName::Pad4x5 => (),
            crate::button_map::NoteName::Pad7x5 => (),
            crate::button_map::NoteName::Pad5x5 => (),
            crate::button_map::NoteName::Pad6x5 => (),
            crate::button_map::NoteName::Pad0x4 => (),
            crate::button_map::NoteName::Pad1x4 => (),
            crate::button_map::NoteName::Pad2x4 => (),
            crate::button_map::NoteName::Pad3x4 => (),
            crate::button_map::NoteName::Pad4x4 => (),
            crate::button_map::NoteName::Pad5x4 => (),
            crate::button_map::NoteName::Pad6x4 => (),
            crate::button_map::NoteName::Pad7x4 => (),
            crate::button_map::NoteName::Pad0x3 => (),
            crate::button_map::NoteName::Pad1x3 => (),
            crate::button_map::NoteName::Pad2x3 => (),
            crate::button_map::NoteName::Pad3x3 => (),
            crate::button_map::NoteName::Pad4x3 => (),
            crate::button_map::NoteName::Pad5x3 => (),
            crate::button_map::NoteName::Pad6x3 => (),
            crate::button_map::NoteName::Pad7x3 => (),
            crate::button_map::NoteName::Pad3x1 => (),
            crate::button_map::NoteName::Pad0x2 => (),
            crate::button_map::NoteName::Pad5x1 => (),
            crate::button_map::NoteName::Pad1x2 => (),
            crate::button_map::NoteName::Pad7x1 => (),
            crate::button_map::NoteName::Pad2x2 => (),
            crate::button_map::NoteName::Pad3x2 => (),
            crate::button_map::NoteName::Pad4x2 => (),
            crate::button_map::NoteName::Pad5x2 => (),
            crate::button_map::NoteName::Pad6x2 => (),
            crate::button_map::NoteName::Pad7x2 => (),
            crate::button_map::NoteName::Pad0x1 => (),
            crate::button_map::NoteName::Pad1x1 => (),
            crate::button_map::NoteName::Pad2x1 => (),
            crate::button_map::NoteName::Pad4x1 => (),
            crate::button_map::NoteName::Pad6x1 => (),
            crate::button_map::NoteName::Pad0x0 => self
                .sender
                .send(Query::Play)
                .expect("Could not send query to thread."),
            crate::button_map::NoteName::Pad1x0 => self
                .sender
                .send(Query::Pause)
                .expect("Could not send query to thread."),
            crate::button_map::NoteName::Pad2x0 => {
                self.sender
                    .send(Query::Skip)
                    .expect("Could not send query to thread.");
                self.sender
                    .send(Query::CurrentSong(None))
                    .expect("Could not send query to thread.");
            }
            crate::button_map::NoteName::Pad3x0 => {
                if let Some(playlists) = &self.playlists {
                    if playlists.len() > self.selected_playlist {
                        self.sender
                            .send(Query::PlayPlaylist(Some(playlists[self.selected_playlist].clone())))
                            .expect("Could not send query to thread.");
                    }
                }
                self.sender
                    .send(Query::CurrentSong(None))
                    .expect("Could not send query to thread.");
            }
            crate::button_map::NoteName::Pad4x0 => (),
            crate::button_map::NoteName::Pad5x0 => (),
            crate::button_map::NoteName::Pad6x0 => (),
            crate::button_map::NoteName::Pad7x0 => (),
        }

        Ok(super::LightAction::None)
    }

    fn control_press(
        &mut self,
        _control_name: crate::button_map::ControlName,
    ) -> Result<super::LightAction, MyError> {
        // Do nothing for now
        Ok(super::LightAction::None)
    }

    fn apply_button_lights(
        &mut self,
        midiconn: &std::sync::Arc<std::sync::Mutex<crate::midi::MidiConnection>>,
        button_values: &std::collections::HashMap<u8, crate::button_map::ButtonType>,
    ) -> Result<(), MyError> {
        // Do nothing for now

        let mut midi = midiconn.try_lock().expect("Could not lock midi-conn.");

        for (address, name) in button_values {
            match name {
                crate::button_map::ButtonType::ControlChange(_) => (),
                crate::button_map::ButtonType::Note(note_name) => match note_name {
                    crate::button_map::NoteName::Pad0x7 => (),
                    crate::button_map::NoteName::Pad1x7 => (),
                    crate::button_map::NoteName::Pad2x7 => (),
                    crate::button_map::NoteName::Pad3x7 => (),
                    crate::button_map::NoteName::Pad4x7 => (),
                    crate::button_map::NoteName::Pad5x7 => (),
                    crate::button_map::NoteName::Pad6x7 => (),
                    crate::button_map::NoteName::Pad7x7 => (),
                    crate::button_map::NoteName::Pad0x6 => (),
                    crate::button_map::NoteName::Pad1x6 => (),
                    crate::button_map::NoteName::Pad2x6 => (),
                    crate::button_map::NoteName::Pad3x6 => (),
                    crate::button_map::NoteName::Pad4x6 => (),
                    crate::button_map::NoteName::Pad5x6 => (),
                    crate::button_map::NoteName::Pad6x6 => (),
                    crate::button_map::NoteName::Pad7x6 => (),
                    crate::button_map::NoteName::Pad0x5 => (),
                    crate::button_map::NoteName::Pad1x5 => (),
                    crate::button_map::NoteName::Pad2x5 => (),
                    crate::button_map::NoteName::Pad3x5 => (),
                    crate::button_map::NoteName::Pad4x5 => (),
                    crate::button_map::NoteName::Pad5x5 => (),
                    crate::button_map::NoteName::Pad6x5 => (),
                    crate::button_map::NoteName::Pad7x5 => (),
                    crate::button_map::NoteName::Pad0x4 => (),
                    crate::button_map::NoteName::Pad1x4 => (),
                    crate::button_map::NoteName::Pad2x4 => (),
                    crate::button_map::NoteName::Pad3x4 => (),
                    crate::button_map::NoteName::Pad4x4 => (),
                    crate::button_map::NoteName::Pad5x4 => (),
                    crate::button_map::NoteName::Pad6x4 => (),
                    crate::button_map::NoteName::Pad7x4 => (),
                    crate::button_map::NoteName::Pad0x3 => (),
                    crate::button_map::NoteName::Pad1x3 => (),
                    crate::button_map::NoteName::Pad2x3 => (),
                    crate::button_map::NoteName::Pad3x3 => (),
                    crate::button_map::NoteName::Pad4x3 => (),
                    crate::button_map::NoteName::Pad5x3 => (),
                    crate::button_map::NoteName::Pad6x3 => (),
                    crate::button_map::NoteName::Pad7x3 => (),
                    crate::button_map::NoteName::Pad0x2 => (),
                    crate::button_map::NoteName::Pad1x2 => (),
                    crate::button_map::NoteName::Pad2x2 => (),
                    crate::button_map::NoteName::Pad3x2 => (),
                    crate::button_map::NoteName::Pad4x2 => (),
                    crate::button_map::NoteName::Pad5x2 => (),
                    crate::button_map::NoteName::Pad6x2 => (),
                    crate::button_map::NoteName::Pad7x2 => (),
                    crate::button_map::NoteName::Pad0x1 => (),
                    crate::button_map::NoteName::Pad1x1 => (),
                    crate::button_map::NoteName::Pad2x1 => (),
                    crate::button_map::NoteName::Pad3x1 => (),
                    crate::button_map::NoteName::Pad4x1 => (),
                    crate::button_map::NoteName::Pad5x1 => (),
                    crate::button_map::NoteName::Pad6x1 => (),
                    crate::button_map::NoteName::Pad7x1 => (),
                    crate::button_map::NoteName::Pad0x0 => midi
                        .send_to_device(&[0b10010000, *address, 124u8])
                        .expect("Could not send button color to device."),
                    crate::button_map::NoteName::Pad1x0 => midi
                        .send_to_device(&[0b10010000, *address, 124u8])
                        .expect("Could not send button color to device."),
                    crate::button_map::NoteName::Pad2x0 => midi
                        .send_to_device(&[0b10010000, *address, 124u8])
                        .expect("Could not send button color to device."),
                    crate::button_map::NoteName::Pad3x0 => {
                        midi.send_to_device(&[0b10010000, *address, 124u8])
                            .expect("Could not send button color to device.");
                    }
                    crate::button_map::NoteName::Pad4x0 => (),
                    crate::button_map::NoteName::Pad5x0 => (),
                    crate::button_map::NoteName::Pad6x0 => (),
                    crate::button_map::NoteName::Pad7x0 => (),
                },
                crate::button_map::ButtonType::Encoder(encoder_name) => {
                    if *encoder_name == EncoderName::Control71 {
                        midi.send_to_device(&[0b10110000, *address, 122u8])
                            .expect("Could not send button color to device.");
                    }
                }
            }
        }

        Ok(())
    }

    fn update(&mut self) -> Result<super::LightAction, MyError> {
        for msg in self.receiver.try_iter() {
            match msg {
                Query::CurrentSong(song) => self.playing_song = song,
                Query::Play => (),
                Query::Pause => (),
                Query::Skip => (),
                Query::CurrentUserPlaylists(playlists) => self.playlists = playlists,
                Query::PlayPlaylist(_) => (),
            }
        }

        if (std::time::Instant::now() - self.last_updated) > std::time::Duration::from_secs(5) {
            self.sender
                .send(Query::CurrentSong(None))
                .expect("Could not send Query to thread.");

            self.sender
                .send(Query::CurrentUserPlaylists(None))
                .expect("Could not send Query to thread.");

            self.last_updated = std::time::Instant::now();
        }

        Ok(super::LightAction::None)
    }

    fn display(&self, display: &mut push2_display::Push2Display) -> Result<(), MyError> {
        if let Some(song) = &self.playing_song {
            Text::new(
                song,
                Point { x: 10, y: 15 },
                MonoTextStyle::new(&FONT_10X20, Bgr565::WHITE),
            )
            .draw(display)
            .expect("Infallible");
        } else {
            Text::new(
                "No song updated",
                Point { x: 10, y: 15 },
                MonoTextStyle::new(&FONT_10X20, Bgr565::WHITE),
            )
            .draw(display)
            .expect("Infallible");
        }

        if let Some(playlists) = &self.playlists {
            for (index, playlist) in playlists.iter().enumerate() {
                Text::new(
                    &playlist.name,
                    Point {
                        x: 10,
                        y: ((index + 1) as i32 * 15) + 20,
                    },
                    MonoTextStyle::new(
                        &FONT_10X20,
                        if self.selected_playlist == index {
                            Bgr565::GREEN
                        } else {
                            Bgr565::YELLOW
                        },
                    ),
                )
                .draw(display)
                .expect("Infallible");
            }
        }

        Ok(())
    }

    fn encoder_change(
        &mut self,
        encoder_name: EncoderName,
        change: i16,
    ) -> Result<super::LightAction, MyError> {

        if let Some(playlists) = &self.playlists {

            if encoder_name == EncoderName::Control71 {
                let num_playlists = playlists.len();


                let change_mod = (change as isize + num_playlists as isize).rem_euclid(num_playlists as isize);

                self.selected_playlist = 
                    (self.selected_playlist as isize + change_mod).rem_euclid(num_playlists as isize) as usize;
    
                Ok(super::LightAction::None)
            } else {
                Ok(super::LightAction::None)
            }
        } else {
         Ok(super::LightAction::None)
        }

       
    }
}
