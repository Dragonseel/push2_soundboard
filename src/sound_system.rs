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

    repress_mode: RepressMode,
}

impl SoundSystem {
    pub fn new<S: AsRef<str>>(device: S) -> SoundSystem {
        let default_host = rodio::cpal::default_host();
        let mut device_list = default_host.output_devices().unwrap();

        let opt_device = device_list
            .find(|x| x.name().unwrap() == device.as_ref());

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
        }
    }

    pub fn get_sink(&mut self) -> Sink {
        Sink::try_new(&self.stream_handle).unwrap()
    }

    pub fn get_repress_mode(&self) -> RepressMode {
        self.repress_mode
    }

    pub fn set_repress_mode(&mut self, mode: RepressMode) {
        self.repress_mode = mode;
    }
}
