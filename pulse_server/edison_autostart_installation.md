## Configuring the pulse server to automatically start on an Intel Edison

Configure and copy the `edison_pulse_server.service` file to the systemd director (i.e. the
final path to the file should be `/lib/systemd/system/edison_pulse_server.service`) on the Edison,
then run the following commands:

```bash
chmod 644 /lib/systemd/system/edison_pulse_server.service
chown root:root /lib/systemd/system/edison_pulse_server.service
systemctl daemon-reload
systemctl enable edison_pulse_server.service
```

Optional steps

```bash
systemctl start edison_pulse_server.service             # Enable the service without restarting
journalctl --no-pager -u edison_pulse_server.service    # Show log to ensure the service is working
```