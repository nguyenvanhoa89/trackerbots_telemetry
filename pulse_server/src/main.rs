#![feature(mpsc_select)]

extern crate byteorder;
extern crate common;
extern crate animal_detector;
extern crate hackrf;
#[macro_use] extern crate log;
extern crate log4rs;
extern crate mio;
extern crate serde;
#[macro_use] extern crate serde_derive;
extern crate serde_json;

mod endpoint;
mod gain_control;
mod hackrf_task;
mod task;
mod test_task;
mod util;

use std::env;

fn main() {
    let run_test_task = env::args().nth(1) == Some("test".into());

    log4rs::init_file("config/log_config.json", Default::default()).unwrap();
    let config = util::load_json_or_default("config/hackrf_config.json");

    if run_test_task {
        endpoint::start_endpoint(test_task::start_task(config));
    }
    else {
        endpoint::start_endpoint(hackrf_task::start_task(config));
    }
}
