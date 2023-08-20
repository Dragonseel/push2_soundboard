use std::sync::mpsc::{Receiver, Sender};

use midir::{
    Ignore, MidiInput, MidiInputConnection, MidiInputPort, MidiOutput, MidiOutputConnection,
    MidiOutputPort,
};

use crate::MyError;
use std::sync::mpsc::channel;

pub enum MidiMessage {
    Btn(u8, u8),
    Volume(i8),
}

const CONTROL_CHANGE: u8 = 0xB0;
const NOTE_ON: u8 = 0x90;

#[allow(dead_code)]
pub struct MidiConnection {
    in_port: MidiInputPort,
    in_conn: MidiInputConnection<Sender<MidiMessage>>,

    out_port: MidiOutputPort,
    out_conn: MidiOutputConnection,
}

impl MidiConnection {
    pub fn new(
        in_name: &str,
        out_name: &str,
    ) -> Result<(MidiConnection, Receiver<MidiMessage>), MyError> {
        let mut midi_in = MidiInput::new("Push2_Soundboard-IN")?;
        midi_in.ignore(Ignore::None);
        let in_port = MidiConnection::get_midi_in_port(&midi_in, in_name)?;

        let midi_out = MidiOutput::new("Push2_Soundboard-OUT")?;
        let out_port = MidiConnection::get_midi_out_port(&midi_out, out_name)?;
        let conn_out = midi_out.connect(&out_port, "Push2_Soundboard-OutPort")?;

        let (tx1, rx1) = channel::<MidiMessage>();
        // _conn_in needs to be a named parameter, because it needs to be kept alive until the end of the scope
        let conn_in = midi_in.connect(
            &in_port,
            "Push2_Soundboard-InPort",
            move |_, message, tx| {
                match message {
                    [CONTROL_CHANGE, 0x4E, value] => tx
                        .send(MidiMessage::Volume(MidiConnection::get_endcoder_value(
                            value,
                        )))
                        .expect("Could not send volume message through channel"),
                    [CONTROL_CHANGE, address, value] => {
                        tx.send(MidiMessage::Btn(*address, *value))
                            .expect("Could not send control change message through channel.");
                    }
                    [NOTE_ON, address, value] => {
                        tx.send(MidiMessage::Btn(*address, *value))
                            .expect("Could not send note on message through channel");
                    }
                    _ => {
                        // println!("{}: {:X?} (len = {})", stamp, message, message.len());
                    }
                }
            },
            tx1,
        )?;

        Ok((
            MidiConnection {
                in_port,
                in_conn: conn_in,
                out_port,
                out_conn: conn_out,
            },
            rx1,
        ))
    }

    pub fn send_to_device(&mut self, data: &[u8]) -> Result<(), MyError> {
        self.out_conn.send(data)?;
        Ok(())
    }

    fn get_endcoder_value(value: &u8) -> i8 {
        let is_right: bool = (value & 0xC0) == 0;
        if is_right {
            (value & 0x3F) as i8
        } else {
            (64 - ((value & 0x3F) as i8)) * -1
        }
    }

    fn get_midi_in_port(midi_in: &MidiInput, port_name: &str) -> Result<MidiInputPort, MyError> {
        // Get an input port (read from console if multiple are available)
        let in_ports = midi_in.ports();

        let ip = in_ports
            .iter()
            .find(|&x| midi_in.port_name(x).unwrap_or_default() == port_name.to_string())
            .ok_or(MyError::NoMidiInFound)?;
        Ok(ip.clone())
    }

    fn get_midi_out_port(
        midi_out: &MidiOutput,
        port_name: &str,
    ) -> Result<MidiOutputPort, MyError> {
        let out_ports = midi_out.ports();
        let p = out_ports
            .iter()
            .find(|&x| midi_out.port_name(x).unwrap_or_default() == port_name.to_string())
            .ok_or(MyError::NoMidiOutFound)?;
        Ok(p.clone())
    }
}
