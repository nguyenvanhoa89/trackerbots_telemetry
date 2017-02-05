Telemetry host
====

This tool provides an abstraction layer on top of the raw Mavlink telemetry stream, and pulse stream
from pulse server.

`telemetry_host` listens to the Mavlink telemetry stream on `udp:127.0.0.1:14552` and to the pulse
stream on `tcp:127.0.0.1:11000`. Currently it cannot be configured to use different addresses/ports
without code changes.

Both the Mavlink telemetry stream, and the pulse stream must be active before starting this tool.

    - See: `simulator_instructions.md` for details about how to start the simulator Mavlink stream.
    - See: the `pulse_server` subdirectory for details about how to start the pulse server stream.

Once the `telemetry_host` has been started, the following functionality is supported:

 - Sending `GET /`: Returns the latest telemetry from the UAV.
 - `PUT /` - Sends a `MAV_DO_REPOSITION` command to the UAV
 - `GET /pulses/<index>`  - Returns the list of pulses that have occurred since the `<index>` pulse
 (`GET /pulses/0` will return all pulses).