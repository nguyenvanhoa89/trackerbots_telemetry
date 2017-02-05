//! A task for managing the connection to the HackRF

use std::error::Error;
use std::thread;
use std::time::Duration;

use std::fs::File;
use std::io::{Write, BufWriter};

use common::{Config, Command};
use common::signal::*;

use gain_control::GainControl;
use animal_detector::Detectors;

use hackrf::{self, HackRF, HackRFResult};
use task::{init_task, Task, TaskHandle};

pub fn start_task(config: Config) -> TaskHandle<Pulse, Command> {
    let (mut task, task_handle) = init_task();

    info!(target: "hackrf_task", "Starting HackRF task");
    thread::spawn(move|| {
        loop {
            info!(target: "hackrf_task", "Running HackRF");

            if let Err(e) = run_hackrf(&mut task, config.clone()) {
                error!(target: "hackrf_task", "HackRF task failure: {}", e);
                thread::sleep(Duration::from_secs(10));
            }
        }
    });

    task_handle
}

fn run_hackrf(task: &mut Task<Pulse, Command>, config: Config) -> Result<(), Box<Error>> {
    let context = try!(hackrf::init());

    let mut hackrf_task = HackRFTask {
        task: task,
        device: try!(HackRF::open(&context)),
        detectors: Detectors::new(&config),
        gain_control: GainControl::new(&config.hackrf_config),
        config: config,
    };
    try!(hackrf_task.configure());

    let mut command = try!(hackrf_task.task.command_receiver.recv());
    loop {
        command = match command {
            // Start receiver data from the HackRF.
            Command::Start(config) => try!(hackrf_task.receiver_loop(config)),

            // The hackrf task has been stopped, wait for a new command
            Command::Stop => try!(hackrf_task.task.command_receiver.recv()),

            // Exit the task, this should be called on server shutdown
            Command::Exit => break,
        };
    }

    Err("Exit called".into())
}

struct HackRFTask<'a> {
    task: &'a mut Task<Pulse, Command>,
    device: HackRF,
    detectors: Detectors,
    gain_control: GainControl,
    config: Config,
}

impl<'a> HackRFTask<'a> {
    fn configure(&mut self) -> HackRFResult<()> {
        let hackrf_config = &self.config.hackrf_config;

        try!(self.device.set_samp_rate(hackrf_config.samp_rate as f64));
        try!(self.device.set_freq(hackrf_config.center_freq));
        try!(self.device.set_lna_gain(hackrf_config.lna_gain));
        try!(self.device.set_vga_gain(hackrf_config.vga_gain));
        try!(self.device.set_amp_enable(hackrf_config.amp_enable));
        try!(self.device.set_antenna_enable(hackrf_config.antenna_enable));

        if let Some(baseband_filter) = hackrf_config.baseband_filter {
            try!(self.device.set_baseband_filter_bw(baseband_filter));
        }

        Ok(())
    }

    /// Start the receiver loop with an optional update to the HackRF's configuration
    fn receiver_loop(&mut self, config: Option<Config>) -> Result<Command, Box<Error>> {
        if let Some(config) = config {
            info!(target: "hackrf_task", "Configuring HackRF: {:?}", config);
            self.detectors = Detectors::new(&config);
            self.config = config;
        };

        try!(self.configure());
        let mut rx_stream = try!(self.device.rx_stream(5));

        let mut log_file = None;
        if let Some(ref filename) = self.config.hackrf_config.raw_log {
            log_file = Some(BufWriter::new(try!(File::create(filename))));
        }

        let mut overflow_count = self.device.overflow_count();
        let output_command;
        loop {
            // Ensure that we are still actually streaming
            try!(self.device.is_streaming());

            let data_receiver = rx_stream.receiver();
            let command_receiver = &mut self.task.command_receiver;

            select! {
                command = command_receiver.recv() => {
                    output_command = try!(command);
                    break;
                },

                data = data_receiver.recv() => {
                    let data = try!(data);

                    // Check for missed samples
                    if overflow_count != self.device.overflow_count() {
                        warn!(target: "hackrf_task", "Samples dropped: {}", overflow_count);
                        overflow_count = self.device.overflow_count();
                    }

                    // Write to log file (if log file was specified)
                    if let Some(ref mut file) = log_file {
                        try!(file.write_all(&data));
                    }

                    let pulses = self.detectors.next(&data);
                    for &pulse in &pulses {
                        try!(self.task.data_sender.send(pulse));
                    }

                    if self.config.hackrf_config.auto_gain {
                        self.gain_control.update_sample_count(data.len() as u64);
                        if let Some(gain_update) = self.gain_control.check_gain(&pulses) {
                            println!("Updating gain: {:?}", gain_update);

                            self.config.hackrf_config.lna_gain = gain_update.lna_gain;
                            self.config.hackrf_config.vga_gain = gain_update.vga_gain;

                            // Since we have a new gain update restart the server
                            output_command = Command::Start(None);
                            break;
                        }
                    }
                }
            }
        }

        try!(rx_stream.stop());
        Ok(output_command)
    }
}
