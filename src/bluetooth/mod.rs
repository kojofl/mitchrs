pub mod mitch;

use std::time::Duration;

use btleplug::{
    api::{Central as _, CentralEvent, CentralState, Manager as _, Peripheral as _, ScanFilter},
    platform::{Adapter, Manager},
};
use futures::StreamExt as _;
use mitch::Mitch;
use tokio::sync::mpsc;

use crate::event::Event;

#[derive(Clone, Debug)]
pub enum BluetoothEvent {
    Discovered(Mitch),
    Lost(String),
    NotActive,
}

pub struct BtleDiscoverTask {
    sender: mpsc::UnboundedSender<Event>,
}

async fn get_central(manager: &Manager) -> Adapter {
    let adapters = manager.adapters().await.unwrap();
    adapters.into_iter().nth(0).unwrap()
}

impl BtleDiscoverTask {
    /// Constructs a new instance of [`EventThread`].
    pub fn new(sender: mpsc::UnboundedSender<Event>) -> Self {
        Self { sender }
    }

    /// Runs the blte discovery thread.
    ///
    /// This function emits mitch discovered events
    pub async fn run(self) -> color_eyre::Result<()> {
        let manager = Manager::new().await?;

        let central = get_central(&manager).await;

        let central_state = central.adapter_state().await.unwrap();

        if central_state != CentralState::PoweredOn {
            self.send(Event::Bluetooth(BluetoothEvent::NotActive));
            return Ok(());
        }

        // Each adapter has an event stream, we fetch via events(),
        // simplifying the type, this will return what is essentially a
        // Future<Result<Stream<Item=CentralEvent>>>.
        let mut events = central.events().await?;

        central.start_scan(ScanFilter::default()).await?;

        while let Some(event) = events.next().await {
            match event {
                CentralEvent::DeviceDiscovered(id) => {
                    let peripheral = central.peripheral(&id).await?;
                    let properties = peripheral.properties().await?;
                    let name = properties
                        .and_then(|p| p.local_name)
                        .unwrap_or_default()
                        .to_lowercase();
                    if name.starts_with("mitch") {
                        self.send(Event::Bluetooth(BluetoothEvent::Discovered(Mitch::new(
                            name.clone(),
                            peripheral.clone(),
                        ))));
                        tokio::time::sleep(Duration::from_secs(2)).await;
                        self.send(Event::Bluetooth(BluetoothEvent::Discovered(Mitch::new(
                            name.clone(),
                            peripheral.clone(),
                        ))));
                        tokio::time::sleep(Duration::from_secs(2)).await;
                        self.send(Event::Bluetooth(BluetoothEvent::Discovered(Mitch::new(
                            name, peripheral,
                        ))));
                    }
                }
                _ => {}
            }
        }
        Ok(())
    }

    /// Sends an event to the receiver.
    fn send(&self, event: Event) {
        // Ignores the result because shutting down the app drops the receiver, which causes the send
        // operation to fail. This is expected behavior and should not panic.
        let _ = self.sender.send(event);
    }
}
