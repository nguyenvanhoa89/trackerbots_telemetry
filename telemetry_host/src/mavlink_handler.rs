use std::f32;
use std::thread;
use std::sync::Mutex;
use std::sync::atomic::{ATOMIC_BOOL_INIT, AtomicBool, Ordering};

use mavlink;
use mavlink::common::*;

#[derive(Copy, Clone, Default)]
pub struct SharedData {
    pub position: [f32; 3],
    pub velocity: [f32; 3],
    pub heading: f32,
    pub next_target: Option<[f32; 3]>
}

lazy_static! {
    pub static ref SHARED_DATA: Mutex<SharedData> = Mutex::new(SharedData::default());
}

static STOPPED: AtomicBool = ATOMIC_BOOL_INIT;

pub struct MavlinkHandle {}

impl Drop for MavlinkHandle {
    fn drop(&mut self) {
        STOPPED.store(true, Ordering::Relaxed);
    }
}

impl MavlinkHandle {
    pub fn new() -> MavlinkHandle {
        STOPPED.store(false, Ordering::Relaxed);
        *SHARED_DATA.lock().unwrap() = SharedData::default();
        thread::spawn(|| mavlink_background_process());
        MavlinkHandle { }
    }
}

const EARTH_RADIUS_METERS: f64 = 6371e3;

#[derive(Default)]
struct GpsBase {
    base: Option<Coordinate>,
}

impl GpsBase {
    fn next(&mut self, lon: i32, lat: i32) -> Option<[f32; 2]> {
        let new = Coordinate { lat: lat as f64 / 1e7, lon: lon as f64 / 1e7 };

        match self.base {
            Some(base) => {
                let x_offset = (new.lon - base.lon).to_radians() * EARTH_RADIUS_METERS;
                let y_offset = (new.lat - base.lat).to_radians() * EARTH_RADIUS_METERS;
                Some([x_offset as f32, y_offset as f32])
            },
            None => {
                self.base = Some(new);
                None
            }
        }
    }

    fn invert(&self, x: f32, y: f32) -> Coordinate {
        let base = self.base.expect("Tried to invert offset without base");

        Coordinate {
            lat: (y as f64 / EARTH_RADIUS_METERS).to_degrees() + base.lat,
            lon: (x as f64 / EARTH_RADIUS_METERS).to_degrees() + base.lon
        }
    }
}

#[derive(Copy, Clone)]
struct Coordinate {
    lat: f64,
    lon: f64,
}

fn mavlink_background_process() {
    println!("Attempting to connect to Mavlink (udp:127.0.0.1:14550)");

    let connection = mavlink::connect("udpin:127.0.0.1:14550").unwrap();

    let mut gps_base = GpsBase::default();
    while STOPPED.load(Ordering::Relaxed) == false {
        match connection.recv().unwrap() {
            MavMessage::GLOBAL_POSITION_INT(data) => {
                if let Some(message) = handle_gps_data(&mut gps_base, data) {
                    println!("Sending message: {:?}", message);
                    if let Err(e) = connection.send(&message) {
                        println!("Failed to send message: {}", e);
                    }
                }
            },

            _ => {}
        }
    }
}

fn handle_gps_data(gps_base: &mut GpsBase, data: GLOBAL_POSITION_INT_DATA) -> Option<MavMessage> {
    let (dx, dy) = match gps_base.next(data.lon, data.lat) {
        Some(position) => (position[0], position[1]),
        None => return None,
    };

    let alt_meters = data.alt as f32 / 1e3;
    let new_position = [dx, dy, alt_meters];
    let velocity = [data.vx as f32 / 100.0, data.vy as f32 / 100.0, data.vz as f32 / 100.0];

    let mut shared_data_lock = SHARED_DATA.lock().unwrap();
    shared_data_lock.position = new_position;
    shared_data_lock.velocity = velocity;
    shared_data_lock.heading = data.hdg as f32 / 100.0;
    // shared_data_lock.orientation = orientation;

    let target = shared_data_lock.next_target.take();
    drop(shared_data_lock);

    if let Some(target) = target {
        println!("Attempting to set new target");

        let dest_coordinate = gps_base.invert(target[0], target[1]);
        let alt = if target[2] != 0.0 { target[2] } else { alt_meters };

        return Some(generate_navigation_message(dest_coordinate.lon as f32,
            dest_coordinate.lat as f32, alt));
    }

    None
}

const MAV_CMD_DO_REPOSITION: u16 = 192;

fn generate_navigation_message(lon: f32, lat: f32, alt: f32) -> MavMessage {
    MavMessage::COMMAND_LONG(COMMAND_LONG_DATA {
        param1: -1.0,
        param2: 1.0,
        param3: 0.0,
        param4: -f32::NAN,
        param5: lat,
        param6: lon,
        param7: alt,
        command: MAV_CMD_DO_REPOSITION,
        target_system: 1,
        target_component: 0,
        confirmation: 0,
    })
}