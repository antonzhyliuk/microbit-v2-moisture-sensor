// See the "macOS permissions note" in README.md before running this on macOS
// Big Sur or later.

use btleplug::api::{
    bleuuid::uuid_from_u16, Central, CharPropFlags, Manager as _, Peripheral, ScanFilter,
};
use btleplug::platform::{Adapter, Manager};
use futures::StreamExt;
use prometheus_exporter::{
    self,
    prometheus::core::{AtomicF64, GenericGauge},
    prometheus::register_gauge,
};
use std::error::Error;
use std::time::Duration;
use tokio::time;
use tokio::time::timeout;
use uuid::Uuid;

/// Only devices whose name contains this string will be tried.
const PERIPHERAL_NAME_MATCH_FILTER: &str = "MicroBit";
/// UUID of the characteristic for which we should subscribe to notifications.
const NOTIFY_CHARACTERISTIC_UUID: Uuid = uuid_from_u16(0xbabe);

#[tokio::main]
async fn main() {
    pretty_env_logger::init();
    let binding = "127.0.0.1:3737".parse().unwrap();
    prometheus_exporter::start(binding).unwrap();
    let gauge = register_gauge!("soil_moisture", "help").unwrap();

    let manager = Manager::new().await.unwrap();
    let adapter_list: Vec<btleplug::platform::Adapter> = manager.adapters().await.unwrap();

    if adapter_list.is_empty() {
        eprintln!("No Bluetooth adapters found");
    }

    loop {
        let _ = scan_and_subscribe(&adapter_list, &gauge).await; // poor-man's supervision
    }
}

async fn scan_and_subscribe(
    adapter_list: &Vec<Adapter>,
    gauge: &GenericGauge<AtomicF64>,
) -> Result<(), Box<dyn Error>> {
    for adapter in adapter_list.iter() {
        println!("Starting scan...");
        adapter
            .start_scan(ScanFilter::default())
            .await
            .expect("Can't scan BLE adapter for connected devices...");

        time::sleep(Duration::from_secs(2)).await;

        let peripherals = adapter.peripherals().await?;

        if peripherals.is_empty() {
            eprintln!("->>> BLE peripheral devices were not found, sorry. Exiting...");
        } else {
            // All peripheral devices in range.
            for peripheral in peripherals.iter() {
                let properties = peripheral.properties().await?;
                let is_connected = peripheral.is_connected().await?;
                let local_name = properties
                    .unwrap()
                    .local_name
                    .unwrap_or(String::from("(peripheral name unknown)"));
                println!(
                    "Peripheral {:?} is connected: {:?}",
                    &local_name, is_connected
                );
                // Check if it's the peripheral we want.
                if local_name.contains(PERIPHERAL_NAME_MATCH_FILTER) {
                    println!("Found matching peripheral {:?}...", &local_name);
                    if !is_connected {
                        // Connect if we aren't already connected.
                        if let Err(err) =
                            timeout(Duration::from_secs(25), peripheral.connect()).await?
                        {
                            eprintln!("Error connecting to peripheral, skipping: {}", err);
                            continue;
                        }
                    }
                    let is_connected =
                        timeout(Duration::from_secs(25), peripheral.is_connected()).await??;
                    println!(
                        "Now connected ({:?}) to peripheral {:?}.",
                        is_connected, &local_name
                    );
                    if is_connected {
                        println!("Discover peripheral {:?} services...", local_name);
                        peripheral.discover_services().await?;
                        for characteristic in peripheral.characteristics() {
                            println!("Checking characteristic {:?}", characteristic);
                            // Subscribe to notifications from the characteristic with the selected
                            // UUID.
                            if characteristic.uuid == NOTIFY_CHARACTERISTIC_UUID
                                && characteristic.properties.contains(CharPropFlags::NOTIFY)
                            {
                                println!("Subscribing to characteristic {:?}", characteristic.uuid);
                                let _ = timeout(
                                    Duration::from_secs(25),
                                    peripheral.subscribe(&characteristic),
                                )
                                .await?;

                                let mut notification_stream = peripheral.notifications().await?;
                                // Process while the BLE connection is not broken or stopped.
                                while let Ok(Some(data)) =
                                    timeout(Duration::from_secs(25), notification_stream.next())
                                        .await
                                {
                                    let metric =
                                        ((data.value[1] as u16) << 8) | data.value[0] as u16;
                                    println!(
                                        "Received data from {:?} [{:?}]: {:?}",
                                        local_name, data.uuid, metric
                                    );
                                    gauge.set(metric.into());
                                }
                            }
                        }
                        println!("Disconnecting from peripheral {:?}...", local_name);
                        let _ = timeout(Duration::from_secs(25), peripheral.disconnect()).await?;
                    }
                } else {
                    println!("Skipping unknown peripheral {:?}", local_name);
                }
            }
        }
    }
    Ok(())
}
