## MicroBit v2 (nRF52833) BLE soil moisture sensor

This project contains my first attempts in embedded programming with Rust. After finishing [discovery book](https://docs.rust-embedded.org/discovery/microbit/) I started looking for applications for this board and decided to build a soil moisture sensor for my Zamioculcas. I thought that it would be fun to monitor plants' soil moisture and plot metrics in Grafana.

#### Firmware design:
Firmware uses Nordic's "softdevice" s140 for accessing BLE peripherals. It should be flashed on the chip at the first segment of Flash and then it will boot the user's code placed after. The user's code will enable s140 and start advertising with a local name `MicroBit`. Advertising will contain service `0xCAFE` with `READ|NOTIFY` characteristic `0xBABE`. This characteristic will notify every 10 seconds about soil moisture measurement. Firmware measures soil moisture by wiring pin `p0.03` to ADC.
The solution is heavily influenced by [this example](https://github.com/embassy-rs/nrf-softdevice/blob/master/examples/src/bin/ble_bas_peripheral_notify.rs).

#### Exporter design:
The exporter is a Rust binary that scans for BLE peripheral named `MicroBit`. It subscribes to characteristic `0xBABE` and then exposes it in Prometheus format on the `/metrics` path using [prometheus_exporter library](https://docs.rs/prometheus_exporter/latest/prometheus_exporter/). The solution is heavily influenced by [this example](https://github.com/deviceplug/btleplug/blob/master/examples/subscribe_notify_characteristic.rs).

#### Monitoring design:
Prometheus scrapes metrics from the exporter on port `3737` and then Grafana queries Prometheus for plotting and alerts.


#### Getting started:
1. Download and extract the [s140 softdevice](https://www.nordicsemi.com/Products/Development-software/s140) to the 'embedded-sensor/s140_nrf52_7.3.0' catalog

2. Flash softdevice to the chip.
```
cd embedded-sensor/s140_nrf52_7.3.0
sh flash.sh
```

3. Flash sensor binary to the chip
```
cd ../microbit-v2-moisture-sensor
cargo run --release
```

4. Install sensor-exporter bin to your host
```
cd ../sensor-exporter
cargo install --path .
```

5. Register sensor-exporter daemon using your host's process manager. Example for MacOs 13.4 (22F66):
```
cat > ~/Library/LaunchAgents/microbit.exporter.daemon.plist <<EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
  <dict>

    <key>Label</key>
    <string>microbit.exporter.daemon.plist</string>

    <key>RunAtLoad</key>
    <true/>

    <key>StartInterval</key>
    <integer>20</integer>

    <key>StandardErrorPath</key>
    <string>/Users/antonzhyliuk/work/rust/microbit/sensor-exporter/stderr.log</string>

    <key>StandardOutPath</key>
    <string>/Users/antonzhyliuk/work/rust/microbit/sensor-exporter/stdout.log</string>

    <key>WorkingDirectory</key>
    <string>/Users/antonzhyliuk/work/rust/microbit/sensor-exporter</string>

    <key>ProgramArguments</key>
    <array>
      <string>/Users/antonzhyliuk/.cargo/bin/sensor-exporter</string>
    </array>

    <key>KeepAlive</key>
    <true/>

  </dict>
</plist>

EOF

launchctl load -w ~/Library/LaunchAgents/microbit.exporter.daemon.plist
```

6. Run docker images of monitoring systems (Prometheus and Grafana)
```
cd ../monitoring
sh start-prometheus.sh
sh start-grafana.sh
```

7. Go to `localhost:3000` (Grafana UI) and configure Prometheus data-source on URL `http://host.docker.internal:9090`

8. Create a dashboard with a panel displaying `soil_moisture` metric
