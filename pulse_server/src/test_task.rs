use std::cmp;
use std::error::Error;
use std::thread;
use std::time::Duration;

use std::sync::mpsc::{channel, sync_channel, Sender, SyncSender, TrySendError};

use std::fs::File;
use std::io::{Read, BufReader};

use common::{Config, Command};
use common::signal::*;

use task::{init_task, Task, TaskHandle};

use animal_detector::Detectors;

pub fn start_task(config: Config) -> TaskHandle<Pulse, Command> {
    let (mut task, task_handle) = init_task();

    info!(target: "hackrf_task", "Starting test task");
    thread::spawn(move|| {
        loop {
            info!(target: "hackrf_task", "Running test");

            if let Err(e) = run_test(&mut task, config.clone()) {
                error!(target: "hackrf_task", "Test task failure: {}", e);
                thread::sleep(Duration::from_secs(10));
            }
        }
    });

    task_handle
}

fn run_test(task: &mut Task<Pulse, Command>, config: Config) -> Result<(), Box<Error>> {
    let mut test_task = TestTask {
        task: task,
        detectors: Detectors::new(&config),
    };

    let mut command = try!(test_task.task.command_receiver.recv());
    loop {
        command = match command {
            Command::Start(config) => try!(test_task.receiver_loop(config)),
            Command::Stop => try!(test_task.task.command_receiver.recv()),
            Command::Exit => break,
        };
    }
    Ok(())
}

struct TestTask<'a> {
    task: &'a mut Task<Pulse, Command>,
    detectors: Detectors,
}

impl<'a> TestTask<'a> {
    fn receiver_loop(&mut self, config: Option<Config>) -> Result<Command, Box<Error>> {
        if let Some(config) = config {
            self.detectors = Detectors::new(&config);
        }

        let command_receiver = &mut self.task.command_receiver;

        if let Ok(file) = File::open("signal.bin") {
            let (data_sender, data_receiver) = sync_channel(5);

            thread::spawn(move|| file_source(file, data_sender));

            loop {
                select! {
                    command = command_receiver.recv() => { return Ok(try!(command)); },

                    data = data_receiver.recv() => {
                        let data = try!(data);
                        for pulse in self.detectors.next(&data) {
                            try!(self.task.data_sender.send(pulse));
                        }
                    }
                }
            }
        }
        else {
            let (pulse_sender, pulse_receiver) = channel();
            thread::spawn(move|| no_source(pulse_sender));

            loop {
                select! {
                    command = command_receiver.recv() => { return Ok(try!(command)); },

                    pulse = pulse_receiver.recv() => {
                        try!(self.task.data_sender.send(try!(pulse)));
                    }
                }
            }
        }
    }
}

const FRAME_SIZE: usize = 4_000_000;

fn file_source(file: File, sender: SyncSender<Vec<u8>>) {
    let mut reader = BufReader::new(file);
    let data = {
        let mut buffer = vec![];
        reader.read_to_end(&mut buffer).unwrap();
        buffer
    };

    let mut index = 0;

    let mut frame = vec![0; FRAME_SIZE];

    loop {
        let read_size = cmp::min(data.len() - index, FRAME_SIZE);
        frame[0..read_size].copy_from_slice(&data[index..(index + read_size)]);

        if read_size < FRAME_SIZE {
            index = FRAME_SIZE - read_size;
            frame[read_size..FRAME_SIZE].copy_from_slice(&data[0..index]);
        }
        else {
            index += read_size;
        }

        match sender.try_send(frame.clone()) {
            Ok(()) => {},
            Err(TrySendError::Full(_)) => warn!(target: "hackrf_task", "Sample dropped"),
            Err(_) => break,
        }

        thread::sleep(Duration::from_secs(1));
    }
}

fn no_source(pulse_sender: Sender<Pulse>) {
    loop {
        let now = Timestamp::now();
        let pulse = Pulse { freq: 1.0, signal_strength: 1.0, gain: 0, timestamp: now };
        if pulse_sender.send(pulse).is_err() {
            break;
        }
        thread::sleep(Duration::from_secs(1));
    }
}
