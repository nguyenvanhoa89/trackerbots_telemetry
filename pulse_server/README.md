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

## Edison autostart configuration

See `edison_autostart_installation.md` for details about how to configure the pulse server to
automatically run and restart on crash when installed on an Intel Edison.