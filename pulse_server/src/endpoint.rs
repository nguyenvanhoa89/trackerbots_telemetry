use std::collections::{HashMap, VecDeque};
use std::sync::mpsc::{self, TryRecvError};
use std::thread;

use std::io::prelude::*;
use std::io::{self, ErrorKind};

use mio::*;
use mio::channel::{channel, Receiver};
use mio::tcp::{TcpListener, TcpStream};

use common::Command;
use common::signal::Pulse;

use task::TaskHandle;

use serde_json;
use byteorder::{ByteOrder, ReadBytesExt, LittleEndian};

const SERVER_TOKEN: Token = Token(0);
const PULSE_READY_EVENT: Token = Token(1);

pub fn start_endpoint(task_handle: TaskHandle<Pulse, Command>) {
    let TaskHandle { data_receiver, command_sender } = task_handle;
    let poll = Poll::new().unwrap();

    // Spawn a thread to handle events from the task and forward them to the event loop.
    // TODO: Consider using an mio channel in the task instead of a standard channel so that this
    // step can be skipped.
    let (pulse_sender, pulse_receiver) = channel();
    poll.register(&pulse_receiver, PULSE_READY_EVENT, Ready::readable(), PollOpt::edge()).unwrap();

    thread::spawn(move || {
        info!(target: "pulse_manager", "Starting pulse manager");

        while let Ok(pulse) = data_receiver.recv() {
            info!(target: "pulse_manager", "{:?}", pulse);
            pulse_sender.send(pulse).unwrap();
        }

        warn!(target: "pulse_manager", "Pulse manager stopped");
    });

    info!(target: "web_server", "Starting server");

    let mut server = PulseServer::new(pulse_receiver, command_sender);
    server.start_loop(poll);
}


struct PulseServer {
    listener: TcpListener,
    next_client_id: usize,
    clients: HashMap<usize, PulseClient>,
    pulse_receiver: Receiver<Pulse>,
    command_sender: mpsc::Sender<Command>,
    buffer: Vec<u8>,
}

impl PulseServer {
    /// Create a new instance of the pulse server
    fn new(pulse_receiver: Receiver<Pulse>, command_sender: mpsc::Sender<Command>) -> PulseServer {
        let addr = "0.0.0.0:11000".parse().unwrap();
        let listener = TcpListener::bind(&addr).unwrap();

        PulseServer {
            listener: listener,
            next_client_id: 2,
            clients: HashMap::new(),
            pulse_receiver: pulse_receiver,
            command_sender: command_sender,
            buffer: vec![],
        }
    }

    /// Start the server, polling for new events
    fn start_loop(&mut self, poll: Poll) {
        poll.register(&self.listener, SERVER_TOKEN, Ready::readable(), PollOpt::edge()).unwrap();

        let mut events = Events::with_capacity(1024);
        loop {
            poll.poll(&mut events, None).unwrap();

            for event in events.iter() {
                self.handle_event(event, &poll);
            }
        }
    }

    /// Handle a received event
    fn handle_event(&mut self, event: Event, poll: &Poll) {
        match event.token() {
            SERVER_TOKEN => self.accept_clients(poll),

            PULSE_READY_EVENT => {
                loop {
                    match self.pulse_receiver.try_recv() {
                        Ok(pulse) => self.prepare_pulse(pulse, poll),

                        Err(TryRecvError::Empty) => break,
                        Err(e) => warn!(target: "web_server", "Error receiving pulse: {}", e),
                    }
                }
            },

            client_token => {
                let id = client_token.0;

                if event.kind().is_hup() {
                    info!(target: "web_server", "Dropped Client: [{}]", id);
                    self.clients.remove(&id);
                    return;
                }
                else if event.kind().is_readable() {
                    self.read_from_client(id, poll);
                }
                else if event.kind().is_writable() {
                    self.write_to_client(id, poll);
                }
            }
        }
    }

    /// Accept new clients
    fn accept_clients(&mut self, poll: &Poll) {
        loop {
            match self.listener.accept() {
                Ok((socket, address)) => {
                    info!(target: "web_server", "Registering client with address: {}", address);
                    self.register_client(socket, poll);
                },

                Err(ref e) if e.kind() == ErrorKind::WouldBlock => break,
                Err(e) => {
                    error!(target: "web_server", "listener.accept() failed: {}", e);
                }
            }
        }
    }

    /// Register a client in the event loop.
    fn register_client(&mut self, socket: TcpStream, poll: &Poll) {
        let id = self.next_client_id;
        self.next_client_id += 1;

        self.clients.insert(id, PulseClient::new(socket, Token(id)));

        let result = poll.register(&self.clients[&id].connection, Token(id),
            Ready::readable() | Ready::hup(), PollOpt::edge());

        if let Err(e) = result {
            error!(target: "web_server", "Failed to register client: {}", e);
        }
        else {
            info!(target: "web_server", "New client connected (assigned id: [{}])", id);
        }
    }

    /// Handle a command sent by a client
    fn handle_command(&mut self, command: Command) {
        let result = self.command_sender.send(command);

        if let Err(e) = result {
            error!(target: "web_server", "Failed to send client command: {}", e);
        }
        else {
            info!(target: "web_server", "Received command from client");
        }
    }

    /// Prepares a pulse to be sent to each of the connected clients
    fn prepare_pulse(&mut self, pulse: Pulse, poll: &Poll) {
        if self.clients.len() == 0 {
            return;
        }

        // Write pulse to internal buffer, reserving the first 8 bytes for the length of the
        // encoded pulse.
        self.buffer.resize(8, 0);
        serde_json::to_writer(&mut self.buffer, &pulse).unwrap();

        let length = self.buffer.len() as u64 - 8;
        LittleEndian::write_u64(&mut self.buffer[..8], length);

        // Add the pulse to the backlog of each of the clients and register them for write events.
        for (_, client) in &mut self.clients {
            client.backlog.push_back(self.buffer.clone());

            let result = poll.reregister(&client.connection, client.token,
                Ready::writable() | Ready::readable() | Ready::hup(), PollOpt::edge());

            if let Err(e) = result {
                error!(target: "web_server", "Failed to reregister client for events: {}", e);
            }
        }
    }

    /// Read something from the client
    fn read_from_client(&mut self, id: usize, poll: &Poll) {
        loop {
            let command = match self.clients.get_mut(&id) {
                Some(client) => client.read_event(),
                None => {
                    error!(target: "web_server", "Tried to read from missing client: [{}]", id);
                    break;
                }
            };

            match command {
                Ok(Some(command)) => self.handle_command(command),
                Ok(None) => {},

                Err(ref e) if e.kind() == ErrorKind::WouldBlock => break,
                Err(e) => {
                    error!(target: "web_server", "Error reading from client: {}", e);

                    // Kill bad client
                    warn!(target: "web_server", "Dropped Client: [{}]", id);
                    let client = self.clients.remove(&id).unwrap();
                    let _ = poll.deregister(&client.connection);
                    drop(client);
                }
            }
        }
    }

    /// Write something to the client
    fn write_to_client(&mut self, id: usize, poll: &Poll) {
        match self.clients.get_mut(&id) {
            Some(client) => client.write_event(poll),
            None => {
                error!(target: "web_server", "Tried to write event to missing client: [{}]", id);
            }
        }
    }
}

struct PulseClient {
    connection: TcpStream,
    token: Token,
    backlog: VecDeque<Vec<u8>>,
    buffer: Vec<u8>,
    bytes_read: usize,
}

impl PulseClient {
    fn new(connection: TcpStream, token: Token) -> PulseClient {
        PulseClient {
            connection: connection,
            token: token,
            backlog: VecDeque::new(),
            buffer: vec![],
            bytes_read: 0,
        }
    }

    /// Reads a command from the current buffer as a json encoded string
    fn read_buffer_json(&mut self) -> Option<Command> {
        let id = self.token.0;

        match serde_json::from_slice(&self.buffer) {
            Ok(command) => {
                info!(target: "web_server", "Read command from client [{}]: {:?}", id, command);
                Some(command)
            },
            Err(e) => {
                error!(target: "web_server", "Failed to parse command from client [{}]: {}", id, e);
                None
            }
        }
    }

    /// Handles read events for the client
    fn read_event(&mut self) -> io::Result<Option<Command>> {
        if self.buffer.len() == 0 {
            // The first u64 in the stream contains the length of the encoded command
            let length = try!(self.connection.read_u64::<LittleEndian>()) as usize;

            if length > 2000 {
                // Limit maximum message size to 2000 bytes
                return Err(io::Error::new(io::ErrorKind::InvalidInput,
                    "Bad Request (Invalid Length)"));
            }

            self.buffer.resize(length, 0);
            self.bytes_read = 0;
        }

        // Read the encoded data
        self.bytes_read += try!(self.connection.read(&mut self.buffer[self.bytes_read..]));

        // If all the data has been read, then parse the result
        if self.bytes_read == self.buffer.len() {
            let result = Ok(self.read_buffer_json());
            self.buffer.clear();
            result
        }
        else {
            Ok(None)
        }
    }

    /// Handles write events for the client
    fn write_event(&mut self, poll: &Poll) {
        while self.backlog.len() > 0 {
            match self.connection.write(&self.backlog.front().unwrap()) {
                // Successfully wrote the message to the client stream, so we can remove it from our
                // backlog.
                Ok(_) => { self.backlog.pop_front(); }

                // The stream is not read to be written to yet, so stop any subsequent writes
                Err(ref e) if e.kind() == ErrorKind::WouldBlock => { break },

                // Some other error occured.
                Err(e) => error!("An unhandled error occured when writing to a client [{}]: {}",
                    self.token.0, e),
            }
        }

        if self.backlog.len() > 0 {
            // We have finished writing our backlog, so unregister for write events.
            let result = poll.reregister(&self.connection, self.token, Ready::readable() |
                Ready::hup(), PollOpt::edge());

            if let Err(e) = result {
                error!(target: "web_server", "Failed to reregister client [{}]: {}",
                    self.token.0, e);
            }
        }
    }
}
