Trackerbots telemetry
====

The pulse server application is designed to run on an Intel Edison connected to a HackRF. It exposes
a TCP endpoint that streams detected pulses to a connected client. See the `pulse_server`
subdirectory for more details.

The `telemetry_host` tool provides a simple REST API on top of the raw data streams for easy
consumption in other applications. See the `telemetry_host` subdirectory for more details.
