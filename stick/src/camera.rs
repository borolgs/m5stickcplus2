use alloc::vec::Vec;
use esp_hal::peripherals::UART1;
use esp_hal::uart::{Config, Uart, UartRx, UartTx};
use esp_hal::Async;
use zune_jpeg::JpegDecoder;

const HEADER: [u8; 2] = [0xAA, 0x55];
const BAUD_RATE: u32 = 1_500_000;

const CMD_FRAMESIZE: u8 = 0x01;

#[repr(u8)]
#[derive(Clone, Copy)]
pub enum FrameSize {
    Qqvga = 1,   // 160x120
    Hqvga = 4,   // 240x176
    Qvga = 6,    // 320x240
    Vga = 10,    // 640x480
}

pub struct Frame {
    pub cmd: u8,
    pub data: Vec<u8>,
}

pub struct Image {
    pub width: usize,
    pub height: usize,
    pub rgb: Vec<u8>,
}

pub struct Camera<'d> {
    rx: UartRx<'d, Async>,
    tx: UartTx<'d, Async>,
}

impl<'d> Camera<'d> {
    pub fn new(
        uart: UART1<'d>,
        rx_pin: impl esp_hal::gpio::InputPin + 'd,
        tx_pin: impl esp_hal::gpio::OutputPin + 'd,
    ) -> Self {
        let config = Config::default().with_baudrate(BAUD_RATE);
        let uart = Uart::new(uart, config)
            .unwrap()
            .with_rx(rx_pin)
            .with_tx(tx_pin)
            .into_async();
        let (rx, tx) = uart.split();
        Self { rx, tx }
    }

    fn build_frame(cmd: u8, data: &[u8]) -> Vec<u8> {
        let payload_len = (data.len() + 2) as u32; // +2 for cmd and crc
        let mut frame = Vec::with_capacity(8 + data.len() + 1);

        frame.push(HEADER[0]);
        frame.push(HEADER[1]);

        let len_bytes = payload_len.to_be_bytes();
        frame.extend_from_slice(&len_bytes);

        let len_crc = len_bytes[0] ^ len_bytes[1] ^ len_bytes[2] ^ len_bytes[3];
        frame.push(len_crc);

        frame.push(cmd);
        frame.extend_from_slice(data);

        let frame_crc = frame.iter().fold(0u8, |acc, &b| acc ^ b);
        frame.push(frame_crc);

        frame
    }

    pub async fn set_framesize(&mut self, size: FrameSize) -> Result<(), ()> {
        let value = size as u16;
        let data = value.to_le_bytes();
        let frame = Self::build_frame(CMD_FRAMESIZE, &data);

        self.tx.write_async(&frame).await.map_err(|_| ())?;
        log::info!("Sent framesize command: {:?}", size as u8);
        Ok(())
    }

    async fn read_header(&mut self) -> Result<(u32, u8), ()> {
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

            let len_crc = buf[2] ^ buf[3] ^ buf[4] ^ buf[5];
            if len_crc != buf[6] {
                continue;
            }

            let mut cmd = [0u8; 1];
            if self.rx.read_async(&mut cmd).await.is_err() {
                continue;
            }

            return Ok((len, cmd[0]));
        }
    }

    pub async fn read_frame(&mut self) -> Result<Frame, ()> {
        let (len, cmd) = self.read_header().await?;
        if len < 2 {
            return Err(());
        }

        let data_len = len as usize - 2;
        let mut data = alloc::vec![0u8; data_len];

        let mut offset = 0;
        while offset < data_len {
            match self.rx.read_async(&mut data[offset..]).await {
                Ok(n) => offset += n,
                Err(_) => return Err(()),
            }
        }

        let mut _crc = [0u8; 1];
        let _ = self.rx.read_async(&mut _crc).await;

        Ok(Frame { cmd, data })
    }

    pub async fn read_image(&mut self) -> Result<Image, ()> {
        let frame = self.read_frame().await?;
        decode_jpeg(&frame.data)
    }

    pub async fn detect(&mut self) -> bool {
        match self.read_frame().await {
            Ok(frame) => {
                log::info!("Camera detected, {} bytes", frame.data.len());
                true
            }
            Err(_) => {
                log::warn!("Camera not detected");
                false
            }
        }
    }
}

pub fn decode_jpeg(data: &[u8]) -> Result<Image, ()> {
    let mut decoder = JpegDecoder::new(data);
    decoder.decode_headers().map_err(|_| ())?;
    let (width, height) = decoder.dimensions().ok_or(())?;
    let rgb = decoder.decode().map_err(|_| ())?;
    Ok(Image { width, height, rgb })
}

impl Image {
    pub fn to_rgb565_raw(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(self.width * self.height * 2);

        for i in 0..(self.width * self.height) {
            let idx = i * 3;
            let r = self.rgb[idx] as u16;
            let g = self.rgb[idx + 1] as u16;
            let b = self.rgb[idx + 2] as u16;

            let raw = ((r >> 3) << 11) | ((g >> 2) << 5) | (b >> 3);
            out.extend_from_slice(&raw.to_be_bytes());
        }

        out
    }
}
