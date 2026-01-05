use core::net::{Ipv4Addr, SocketAddr, SocketAddrV4};
use core::str::FromStr;

use edge_nal::UdpBind;
use edge_nal_embassy::{Udp, UdpBuffers};
use embassy_net::{Runner, Stack};
use embassy_time::{Duration, Timer};

use esp_radio::wifi::{WifiApState, WifiController, WifiDevice, WifiEvent};

pub const RPC_PORT: u16 = 9000;
pub const SERVER_IP: Ipv4Addr = Ipv4Addr::new(192, 168, 2, 1);

#[embassy_executor::task]
pub async fn echo_server(stack: Stack<'static>) {
    let buffers: UdpBuffers<3, 512, 512, 3> = UdpBuffers::new();
    let udp = Udp::new(stack, &buffers);

    let mut socket = match udp
        .bind(SocketAddr::V4(SocketAddrV4::new(
            Ipv4Addr::UNSPECIFIED,
            RPC_PORT,
        )))
        .await
    {
        Ok(s) => s,
        Err(e) => {
            log::error!("UDP bind failed: {:?}", e);
            return;
        }
    };

    log::info!("UDP echo server listening on port {}", RPC_PORT);

    let mut buf = [0u8; 256];

    loop {
        match edge_nal::UdpReceive::receive(&mut socket, &mut buf).await {
            Ok((len, remote)) => {
                log::debug!("recv {} bytes from {}", len, remote);

                if let Err(e) = edge_nal::UdpSend::send(&mut socket, remote, &buf[..len]).await {
                    log::warn!("send error: {:?}", e);
                }
            }
            Err(e) => {
                log::warn!("recv error: {:?}", e);
                Timer::after(Duration::from_millis(100)).await;
            }
        }
    }
}

#[embassy_executor::task]
pub async fn net_task(mut runner: Runner<'static, WifiDevice<'static>>) {
    runner.run().await
}

#[embassy_executor::task]
pub async fn connection(mut controller: WifiController<'static>) {
    log::debug!(
        "Starting wifi. Device capabilities: {:?}",
        controller.capabilities()
    );
    loop {
        match esp_radio::wifi::ap_state() {
            WifiApState::Started => {
                controller.wait_for_event(WifiEvent::ApStop).await;
                Timer::after(Duration::from_millis(5000)).await
            }
            _ => {}
        }
    }
}

#[embassy_executor::task]
pub async fn run_dhcp(stack: Stack<'static>, gw_ip_addr: &'static str) {
    use core::net::{Ipv4Addr, SocketAddrV4};

    use edge_dhcp::{
        io::{self, DEFAULT_SERVER_PORT},
        server::{Server, ServerOptions},
    };
    use edge_nal::UdpBind;
    use edge_nal_embassy::{Udp, UdpBuffers};

    let ip = Ipv4Addr::from_str(gw_ip_addr).expect("dhcp task failed to parse gw ip");

    let mut buf = [0u8; 1500];

    let mut gw_buf = [Ipv4Addr::UNSPECIFIED];

    let buffers = UdpBuffers::<3, 1024, 1024, 10>::new();
    let unbound_socket = Udp::new(stack, &buffers);
    let mut bound_socket = unbound_socket
        .bind(core::net::SocketAddr::V4(SocketAddrV4::new(
            Ipv4Addr::UNSPECIFIED,
            DEFAULT_SERVER_PORT,
        )))
        .await
        .unwrap();

    loop {
        _ = io::server::run(
            &mut Server::<_, 64>::new_with_et(ip),
            &ServerOptions::new(ip, Some(&mut gw_buf)),
            &mut bound_socket,
            &mut buf,
        )
        .await
        .inspect_err(|e| log::warn!("DHCP server error: {e:?}"));
        Timer::after(Duration::from_millis(500)).await;
    }
}
