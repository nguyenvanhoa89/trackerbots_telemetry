#![feature(plugin)]
#![plugin(rocket_codegen)]

extern crate byteorder;
extern crate common;
#[macro_use] extern crate lazy_static;
extern crate mavlink;
extern crate rocket;
extern crate rocket_contrib;
extern crate serde;
#[macro_use] extern crate serde_derive;
extern crate serde_json;

mod pulse_handler;
mod mavlink_handler;

use rocket_contrib::JSON;

use mavlink_handler::{Telemetry, Location, MavlinkHandle};
use pulse_handler::{PulseWithTelemetry, PulseHandle};

#[get("/")]
fn get_telemetry() -> JSON<Telemetry> {
    JSON(mavlink_handler::get_telemetry())
}

#[post("/", data = "<location>")]
fn do_reposition(location: JSON<Location>) {
    let target = location.unwrap();
    mavlink_handler::do_reposition(target);
}

#[get("pulses/<index>")]
fn get_pulses(index: usize) -> JSON<Vec<PulseWithTelemetry>> {
    JSON(pulse_handler::get_pulses_since(index))
}

fn main() {
    let _mavlink_handle = MavlinkHandle::new();
    let _pulse_handle = PulseHandle::new();

    rocket::ignite().mount("/", routes![get_telemetry, get_pulses, do_reposition]).launch();
}
