use app::{Receiver, Sender};
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, mutex::Mutex};
use esp_radio::esp_now::{
    BROADCAST_ADDRESS, EspNowManager, EspNowReceiver, EspNowSender, PeerInfo,
};

#[embassy_executor::task]
pub async fn sender(
    mut app_receiver: Receiver,
    #[allow(unused)] now_sender: &'static Mutex<CriticalSectionRawMutex, EspNowSender<'static>>,
) {
    #[allow(unused)]
    let mut buf = [0u8; 8];
    loop {
        #[allow(unused)]
        let evt = app_receiver.next_message_pure().await;

        #[cfg(feature = "camera")]
        {
            use app::{Camera, Event};

            use crate::camera;

            match evt {
                Event::Camera(Camera::ConnectToCamera { address }) => {
                    let mut sender = now_sender.lock().await;

                    _ = sender
                        .send_async(&address, &camera::protocol::encode_connect())
                        .await;
                }
                Event::Camera(Camera::DisconnectFromCamera { address }) => {
                    let mut sender = now_sender.lock().await;

                    _ = sender
                        .send_async(&address, &camera::protocol::encode_disconnect())
                        .await;
                }
                _ => {}
            }
        }

        #[cfg(feature = "controller")]
        {
            use app::Event;

            let msg = match evt {
                Event::Controller(controller) => Some(postcard::to_slice(&controller, &mut buf)),
                _ => None,
            };

            let Some(msg) = msg else {
                continue;
            };

            match msg {
                Ok(msg) => {
                    let mut sender = now_sender.lock().await;
                    let status = sender.send_async(&BROADCAST_ADDRESS, msg).await;
                    log::debug!("Broadcast {:?}: {:?}", evt, status);
                }
                Err(_) => {}
            }
        }
    }
}

#[embassy_executor::task]
pub async fn listener(
    #[allow(unused)] app_sender: Sender,
    manager: &'static EspNowManager<'static>,
    mut receiver: EspNowReceiver<'static>,
) {
    loop {
        log::trace!("Received message");
        let msg = receiver.receive_async().await;
        if msg.info.dst_address == BROADCAST_ADDRESS {
            if !manager.peer_exists(&msg.info.src_address) {
                manager
                    .add_peer(PeerInfo {
                        interface: esp_radio::esp_now::EspNowWifiInterface::Sta,
                        peer_address: msg.info.src_address,
                        lmk: None,
                        channel: None,
                        encrypt: false,
                    })
                    .unwrap();
                log::debug!("Added peer {:?}", msg.info.src_address);
            }
        }

        #[allow(unused)]
        let data = msg.data();

        #[cfg(feature = "camera")]
        {
            use app::{Camera, Event};

            // log::debug!("Camera msg: {:?}", data);

            if let Some(camera_msg) = crate::camera::protocol::decode(data) {
                match camera_msg {
                    crate::camera::protocol::Message::CameraReady => {
                        app_sender
                            .publish(Event::Camera(Camera::CameraReady {
                                address: msg.info.src_address,
                            }))
                            .await
                    }
                    crate::camera::protocol::Message::FrameChunk {
                        frame_id,
                        chunk_idx,
                        total_chunks,
                        data,
                    } => {
                        let mut chunk = crate::camera::FrameChunk {
                            frame_id,
                            chunk_idx,
                            total_chunks,
                            len: data.len(),
                            data: [0u8; crate::camera::MAX_CHUNK_DATA],
                        };
                        chunk.data[..data.len()].copy_from_slice(data);

                        if crate::camera::FRAME_CHUNKS.try_send(chunk).is_err() {
                            log::warn!("Frame chunk channel full, dropping");
                        }
                    }
                    _ => {}
                }
            }
        }

        #[cfg(feature = "vehicle")]
        {
            use app::{Controller, Event, Vehicle};

            let controller_msg = postcard::from_bytes::<Controller>(data);
            match controller_msg {
                Ok(Controller::Move(left, right)) => {
                    app_sender
                        .publish(Event::Vehicle(Vehicle::Move(left, right)))
                        .await;
                }
                Err(err) => {
                    log::error!("Parse error: {:?}", err);
                }
            }
        }
    }
}
