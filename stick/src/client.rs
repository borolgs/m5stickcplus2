use core::net::{Ipv4Addr, SocketAddr, SocketAddrV4};

use edge_nal::UdpBind;
use edge_nal_embassy::{Udp, UdpBuffers};
use embassy_net::{Runner, Stack};
use embassy_time::{Duration, Timer};

use esp_radio::wifi::{WifiController, WifiDevice, WifiEvent, WifiStaState};

pub const RPC_PORT: u16 = 9000;
pub const SERVER_IP: Ipv4Addr = Ipv4Addr::new(192, 168, 2, 1);

#[embassy_executor::task]
pub async fn echo_client(stack: Stack<'static>) {
    let buffers: UdpBuffers<3, 512, 512, 3> = UdpBuffers::new();

    let socket = Udp::new(stack, &buffers);
    let mut socket = match socket
        .bind(SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, 0)))
        .await
    {
        Ok(s) => s,
        Err(e) => {
            log::error!("UDP client bind failed: {:?}", e);
            return;
        }
    };

    let server = SocketAddr::V4(SocketAddrV4::new(SERVER_IP, RPC_PORT));
    let mut buf = [0u8; 256];

    loop {
        log::debug!("sending hello...");
        if let Err(e) = edge_nal::UdpSend::send(&mut socket, server, b"hello").await {
            log::warn!("send error: {:?}", e);
        } else {
            log::debug!("sent, waiting for response...");
            match embassy_time::with_timeout(
                Duration::from_secs(2),
                edge_nal::UdpReceive::receive(&mut socket, &mut buf),
            )
            .await
            {
                Ok(Ok((len, from))) => {
                    let msg = core::str::from_utf8(&buf[..len]).unwrap_or("<invalid utf8>");
                    log::info!("recv from {}: {}", from, msg);
                }
                Ok(Err(e)) => {
                    log::warn!("recv error: {:?}", e);
                }
                Err(_) => {
                    log::warn!("recv timeout");
                }
            }
        }

        Timer::after(Duration::from_secs(1)).await;
    }
}

#[embassy_executor::task]
pub async fn net_task(mut runner: Runner<'static, WifiDevice<'static>>) {
    runner.run().await
}

#[embassy_executor::task]
pub async fn connection_task(mut controller: WifiController<'static>) {
    log::debug!("connection_task started");
    loop {
        match esp_radio::wifi::sta_state() {
            WifiStaState::Connected => {
                controller.wait_for_event(WifiEvent::StaDisconnected).await;
                log::warn!("WiFi disconnected, reconnecting...");
                Timer::after(Duration::from_millis(1000)).await;
            }
            _ => {
                if let Err(e) = controller.connect_async().await {
                    log::warn!("reconnect failed: {:?}", e);
                    Timer::after(Duration::from_millis(1000)).await;
                }
            }
        }
    }
}
