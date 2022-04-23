use rodio::{
    cpal::{traits::HostTrait, Host},
    Device, DeviceTrait, OutputStream, OutputStreamHandle, Sink,
};

#[derive(Clone, Copy)]
pub enum RepressMode {
    End,
    Interrupt,
}

#[allow(dead_code)]
pub struct SoundSystem {
    host: Host,
    device: Device,

    stream: OutputStream,
    stream_handle: OutputStreamHandle,

    pub repress_mode: RepressMode,

    volume: u32,
}

impl SoundSystem {
    pub fn new<S: AsRef<str>>(device: S) -> SoundSystem {
        let default_host = rodio::cpal::default_host();
        let mut device_list = default_host.output_devices().unwrap();

        let opt_device = device_list.find(|x| x.name().unwrap() == device.as_ref());

        let device = if opt_device.is_some() {
            opt_device.unwrap()
        } else {
            default_host.default_output_device().unwrap()
        };

        for device in device_list {
            println!("{}", device.name().unwrap());
        }

        let (stream, stream_handle) = OutputStream::try_from_device(&device).unwrap();

        SoundSystem {
            host: default_host,
            device,
            stream,
            stream_handle,
            repress_mode: RepressMode::End,
            volume: crate::DEFAULT_VOLUME,
        }
    }

    pub fn get_sink(&mut self) -> Sink {
        Sink::try_new(&self.stream_handle).unwrap()
    }

    pub fn change_volume(&mut self, change: i8) {
        if change.is_negative() {
            self.volume = self.volume.saturating_sub(change.abs() as u32);
        } else {
            self.volume = u32::min(crate::MAX_VOLUME, self.volume.saturating_add(change as u32));
        }
    }

    pub fn get_volume_factor(&self) -> f32 {
        self.volume as f32 / (crate::DEFAULT_VOLUME as f32)
    }
}
