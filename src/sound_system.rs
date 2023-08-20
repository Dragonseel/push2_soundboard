use rodio::{
    cpal::{traits::HostTrait, Host},
    Device, DeviceTrait, OutputStream, OutputStreamHandle, Sink,
};

use crate::MyError;

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
    pub fn new<S: AsRef<str>>(device: S) -> Result<SoundSystem, MyError> {
        let default_host = rodio::cpal::default_host();
        let mut device_list = default_host.output_devices();

        let mut device_list = match device_list {
            Ok(list) => list,
            Err(_) => return Err(MyError::SoundSystemError("Device List not found.")),
        };

        let opt_device = device_list
            .find(|x| x.name().expect("Device name could not be parsed.") == device.as_ref());

        let device = match opt_device {
            Some(value) => value,
            None => {
                let default_device = default_host.default_output_device();
                match default_device {
                    Some(value) => value,
                    None => {
                        return Err(MyError::SoundSystemError("Could not create output device."))
                    }
                }
            }
        };

        let stream_res = OutputStream::try_from_device(&device);

        let (stream, stream_handle) = match stream_res {
            Ok(value) => value,
            Err(_) => return Err(MyError::SoundSystemError("Could not create output stream.")),
        };

        Ok(SoundSystem {
            host: default_host,
            device,
            stream,
            stream_handle,
            repress_mode: RepressMode::End,
            volume: crate::DEFAULT_VOLUME,
        })
    }

    pub fn get_sink(&mut self) -> Result<Sink, MyError> {
        match Sink::try_new(&self.stream_handle) {
            Ok(value) => Ok(value),
            Err(_) => Err(MyError::SoundSystemError("Could not create a sink.")),
        }
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
