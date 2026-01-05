use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, mutex::Mutex};
use embassy_time::{Duration, Ticker};
use esp_radio::esp_now::{
    BROADCAST_ADDRESS, EspNowManager, EspNowReceiver, EspNowSender, PeerInfo,
};

#[embassy_executor::task]
pub async fn broadcaster(sender: &'static Mutex<CriticalSectionRawMutex, EspNowSender<'static>>) {
    let mut ticker = Ticker::every(Duration::from_secs(1));
    loop {
        ticker.next().await;

        log::info!("Send Broadcast...");
        let mut sender = sender.lock().await;

        let status = sender.send_async(&BROADCAST_ADDRESS, b"Hello.").await;
        log::info!("Send broadcast status: {:?}", status);
    }
}

#[embassy_executor::task]
pub async fn listener(
    manager: &'static EspNowManager<'static>,
    mut receiver: EspNowReceiver<'static>,
) {
    loop {
        let r = receiver.receive_async().await;
        log::info!("Received {:?}", r.data());
        if r.info.dst_address == BROADCAST_ADDRESS {
            if !manager.peer_exists(&r.info.src_address) {
                manager
                    .add_peer(PeerInfo {
                        interface: esp_radio::esp_now::EspNowWifiInterface::Sta,
                        peer_address: r.info.src_address,
                        lmk: None,
                        channel: None,
                        encrypt: false,
                    })
                    .unwrap();
                log::info!("Added peer {:?}", r.info.src_address);
            }
        }
    }
}
