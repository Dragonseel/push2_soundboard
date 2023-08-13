use embedded_graphics::{text::Text, prelude::{Point, RgbColor}, mono_font::{MonoTextStyle, iso_8859_13::FONT_10X20}, pixelcolor::Bgr565, Drawable};


pub struct SpotifyMode {

}

impl SpotifyMode {
    pub fn new() -> SpotifyMode {
        SpotifyMode {  }
    }
}

impl super::DeviceMode for SpotifyMode {
    fn button_press(&mut self, note_name: crate::button_map::NoteName) -> super::LightAction {
        // Do nothing for now
        super::LightAction::None
    }

    fn control_press(&mut self, control_name: crate::button_map::ControlName) -> super::LightAction {
        // Do nothing for now
        super::LightAction::None
    }

    fn apply_button_lights(
        &mut self,
        midiconn: &std::sync::Arc<std::sync::Mutex<crate::midi::MidiConnection>>,
        button_values: &std::collections::HashMap<u8, crate::button_map::ButtonType>,
    ) {
        // Do nothing for now
    }

    fn update(&mut self) -> super::LightAction {
        super::LightAction::None
    }

    fn display(&self, display: &mut push2_display::Push2Display) {
        Text::new(
            "Spotify",
            Point { x: 50, y: 15 },
            MonoTextStyle::new(&FONT_10X20, Bgr565::WHITE),
        )
        .draw(display).unwrap();
    }
}