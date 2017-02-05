use std::io::prelude::*;
use std::net::TcpStream;
use std::str;
use std::sync::Mutex;
use std::thread;

use byteorder::{ReadBytesExt, WriteBytesExt, LittleEndian};
use common::Command;
use common::signal::Pulse;
use serde::{Serialize, Deserialize};
use serde_json;

use mavlink_handler::{self, Telemetry};

#[derive(Debug, Clone, Serialize)]
pub struct PulseWithTelemetry {
    pub telemetry: Telemetry,
    pub pulse: Pulse,
}

lazy_static! {
    pub static ref PULSE_DATA: Mutex<Vec<PulseWithTelemetry>> = Mutex::new(vec![]);
}

/// Returns the number of pulses that have occured since the specified index
pub fn get_pulses_since(index: usize) -> Vec<PulseWithTelemetry> {
    let pulse_data = PULSE_DATA.lock().unwrap();

    if index < pulse_data.len() {
        pulse_data[index..].into()
    }
    else {
        vec![]
    }
}

pub struct PulseHandle {}

impl PulseHandle {
    pub fn new() -> PulseHandle {
        thread::spawn(|| run_pulse_client());
        PulseHandle {}
    }
}

fn read_json<R: Read, T: Deserialize>(reader: &mut R, buffer: &mut Vec<u8>) -> T {
    let size = reader.read_u64::<LittleEndian>().unwrap() as usize;

    buffer.clear();
    buffer.resize(size, 0);

    reader.read_exact(buffer).unwrap();
    serde_json::from_slice(buffer).unwrap()
}

fn write_json<W: Write, T: Serialize>(writer: &mut W, buffer: &mut Vec<u8>, value: &T) {
    buffer.clear();
    serde_json::to_writer(buffer, value).unwrap();
    writer.write_u64::<LittleEndian>(buffer.len() as u64).unwrap();
    writer.write(buffer).unwrap();
}

fn run_pulse_client() {
    let mut buffer = vec![];
    let mut connection = TcpStream::connect("127.0.0.1:11000")
        .expect("Failed to connect to Pulse Stream");

    write_json(&mut connection, &mut buffer, &Command::Start(None));
    loop {
        let pulse: Pulse = read_json(&mut connection, &mut buffer);
        println!("{:?}", pulse);

        let value = PulseWithTelemetry {
            pulse: pulse,
            telemetry: mavlink_handler::get_telemetry()
        };

        PULSE_DATA.lock().unwrap().push(value);
    }
}
