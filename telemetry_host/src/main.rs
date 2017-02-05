#![feature(plugin)]
#![plugin(rocket_codegen)]

#[macro_use] extern crate lazy_static;
extern crate mavlink;
extern crate rocket;
extern crate rocket_contrib;
#[macro_use] extern crate serde_derive;

mod mavlink_handler;

use rocket_contrib::JSON;

use mavlink_handler::{SHARED_DATA, MavlinkHandle};

#[derive(Serialize)]
struct Telemetry {
    position: [f32; 3],
    heading: f32,
}

#[derive(Deserialize)]
struct Location {
    pub x: f32,
    pub y: f32,
    pub alt: f32
}

#[get("/")]
fn get_telemetry() -> JSON<Telemetry> {
    let shared_data = SHARED_DATA.lock().unwrap();

    JSON(Telemetry {
        position: shared_data.position,
        heading: shared_data.heading,
    })
}

#[put("/", data = "<location>")]
fn do_reposition(location: JSON<Location>) {
    let target = location.unwrap();
    SHARED_DATA.lock().unwrap().next_target = Some([target.x, target.y, target.alt]);
}

fn main() {
    let _mavlink_handle = MavlinkHandle::new();

    rocket::ignite().mount("/", routes![get_telemetry, do_reposition]).launch();
}
