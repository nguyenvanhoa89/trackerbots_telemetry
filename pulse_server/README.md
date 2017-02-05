Pulse server
====

## Prerequisites

In order to talk to the HackRF, the pulse server uses the `libusb-1.0` library (which is generally
not available by default on most systems).

### Ubuntu

```
sudo apt-get install libusb-1.0-0-dev
```

## Building

```
cargo build --release
```

## Testing

The pulse server can be configured to run in test mode by providing the `test` argument when running
the server:

```
cargo run --release -- test
```

When running in this mode, the server will attempt to read `signal.bin` and perform pulse detection
on that file. If `signal.bin` does not exist then the server will simply generate a fake pulse every
second.

## Edison autostart configuration

See `edison_autostart_installation.md` for details about how to configure the pulse server to
automatically run and restart on crash when installed on an Intel Edison.