use std::sync::{Arc, Mutex};

use embedded_graphics::{
    mono_font::{iso_8859_13::FONT_10X20, MonoTextStyle},
    pixelcolor::Bgr565,
    prelude::{Point, RgbColor},
    text::Text,
    Drawable,
};
use tokio::runtime::Handle;

use crate::{
    spotify::{self, Spotify},
    MyError,
};

pub struct SpotifyMode {
    spotify: Arc<Spotify>,
    last_updated: std::time::Instant,
    running_query: Option<tokio::task::JoinHandle<String>>,

    playing_song: Option<String>,
}

impl SpotifyMode {
    pub async fn new() -> Result<SpotifyMode, MyError> {
        let spotify = Arc::new(spotify::Spotify::new().await?);

        Ok(
        SpotifyMode {
            spotify,
            last_updated: std::time::Instant::now(),
            running_query: None,
            playing_song: None,
        })
    }
}

impl super::DeviceMode for SpotifyMode {
    fn button_press(
        &mut self,
        _note_name: crate::button_map::NoteName,
    ) -> Result<super::LightAction, MyError> {
        // Do nothing for now
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
        _midiconn: &std::sync::Arc<std::sync::Mutex<crate::midi::MidiConnection>>,
        _button_values: &std::collections::HashMap<u8, crate::button_map::ButtonType>,
    ) -> Result<(), MyError> {
        // Do nothing for now
        Ok(())
    }

    fn update(&mut self) -> Result<super::LightAction, MyError> {
        if let Some(true) = self
            .running_query
            .as_ref()
            .and_then(|handle| Some(handle.is_finished()))
        {
            let handle_inner = self
                .running_query
                .take()
                .expect("Could not take option, despite previous check.");

            let playing_song = Arc::new(Mutex::new(String::new()));
            let playing_song_clone = Arc::clone(&playing_song);

            tokio::task::block_in_place(|| {
                Handle::current().block_on(async move {
                    // do something async
                    *playing_song_clone
                        .lock()
                        .expect("Thread could not lock playing song.") = handle_inner
                        .await
                        .expect("Could not join spotify query thread.");
                });
            });

            self.playing_song = Some(
                playing_song
                    .lock()
                    .expect("Could not lock playing song.")
                    .clone(),
            );
        }

        if let Some(handle) = &self.running_query {
            if handle.is_finished() {}
        }

        if (std::time::Instant::now() - self.last_updated) > std::time::Duration::from_secs(5) {
            let async_spotify = Arc::clone(&self.spotify);

            let handle = tokio::spawn(async move { async_spotify.get_current_song().await });

            self.running_query = Some(handle);

            self.last_updated = std::time::Instant::now();
        }

        Ok(super::LightAction::None)
    }

    fn display(&self, display: &mut push2_display::Push2Display) -> Result<(), MyError> {
        if let Some(song) = &self.playing_song {
            Text::new(
                song,
                Point { x: 50, y: 15 },
                MonoTextStyle::new(&FONT_10X20, Bgr565::WHITE),
            )
            .draw(display)
            .expect("Infallible");
        } else {
            Text::new(
                "No song updated",
                Point { x: 50, y: 15 },
                MonoTextStyle::new(&FONT_10X20, Bgr565::WHITE),
            )
            .draw(display)
            .expect("Infallible");
        }

        Ok(())
    }
}
