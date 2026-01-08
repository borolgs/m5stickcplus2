use embassy_time::{Duration, Timer};
use esp_hal::peripherals::UART1;
use esp_hal::uart::{Config, Uart, UartRx};
use esp_hal::Async;

const HEADER: [u8; 2] = [0xAA, 0x55];
const BAUD_RATE: u32 = 1_500_000;

pub struct Camera<'d> {
    rx: UartRx<'d, Async>,
}

impl<'d> Camera<'d> {
    pub fn new(
        uart: UART1<'d>,
        rx_pin: impl esp_hal::gpio::InputPin + 'd,
    ) -> Self {
        let config = Config::default().with_baudrate(BAUD_RATE);
        let uart = Uart::new(uart, config)
            .unwrap()
            .with_rx(rx_pin)
            .into_async();
        let (rx, _tx) = uart.split();
        Self { rx }
    }

    pub async fn wait_for_frame(&mut self) -> Result<u32, ()> {
        let mut buf = [0u8; 7];

        loop {
            if self.rx.read_async(&mut buf[0..1]).await.is_err() {
                continue;
            }

            if buf[0] != HEADER[0] {
                continue;
            }

            if self.rx.read_async(&mut buf[1..2]).await.is_err() {
                continue;
            }

            if buf[1] != HEADER[1] {
                continue;
            }

            if self.rx.read_async(&mut buf[2..7]).await.is_err() {
                continue;
            }

            let len = ((buf[2] as u32) << 24)
                | ((buf[3] as u32) << 16)
                | ((buf[4] as u32) << 8)
                | (buf[5] as u32);

            return Ok(len);
        }
    }

    pub async fn detect(&mut self) -> bool {
        for _ in 0..3 {
            let result = embassy_futures::select::select(
                self.wait_for_frame(),
                Timer::after(Duration::from_secs(2)),
            )
            .await;

            match result {
                embassy_futures::select::Either::First(Ok(len)) => {
                    log::info!("Camera detected, frame size: {} bytes", len);
                    return true;
                }
                embassy_futures::select::Either::First(Err(_)) => {
                    continue;
                }
                embassy_futures::select::Either::Second(_) => {
                    continue;
                }
            }
        }
        log::warn!("Camera not detected");
        false
    }
}
