use core::cell::RefCell;

use app::{Event, Sender};
use embassy_sync::{
    blocking_mutex::{Mutex, raw::CriticalSectionRawMutex},
    channel::Channel,
};
use embedded_graphics::{
    Drawable,
    image::{Image, ImageRawBE},
    prelude::Point,
};
use mousefood::prelude::Rgb565;
use zune_jpeg::JpegDecoder;

pub const MAX_CHUNK_DATA: usize = 241;
pub const MAX_FRAME_SIZE: usize = 12 * 1024; // 12KB for 160x120
pub const MAX_CHUNKS: usize = 64;

pub const FRAME_WIDTH: usize = 160;
pub const FRAME_HEIGHT: usize = 120;
pub const RGB565_BUF_SIZE: usize = FRAME_WIDTH * FRAME_HEIGHT * 2;

#[derive(Clone)]
pub struct FrameChunk {
    pub frame_id: u16,
    pub chunk_idx: u16,
    pub total_chunks: u16,
    pub len: usize,
    pub data: [u8; MAX_CHUNK_DATA],
}

impl FrameChunk {
    pub fn data(&self) -> &[u8] {
        &self.data[..self.len]
    }
}

pub static FRAME_CHUNKS: Channel<CriticalSectionRawMutex, FrameChunk, 32> = Channel::new();

pub struct FrameBuffer {
    pub data: [u8; MAX_FRAME_SIZE],
    pub len: usize,
    pub ready_frame_id: u16,
    assembling_frame_id: u16,
    total_chunks: u16,
    received_mask: u64,
}

impl FrameBuffer {
    const fn new() -> Self {
        Self {
            data: [0u8; MAX_FRAME_SIZE],
            len: 0,
            ready_frame_id: 0,
            assembling_frame_id: 0,
            total_chunks: 0,
            received_mask: 0,
        }
    }

    pub fn add_chunk(&mut self, chunk: &FrameChunk) -> bool {
        if chunk.total_chunks as usize > MAX_CHUNKS {
            return false;
        }

        if chunk.frame_id != self.assembling_frame_id {
            self.assembling_frame_id = chunk.frame_id;
            self.total_chunks = chunk.total_chunks;
            self.received_mask = 0;
        }

        if chunk.chunk_idx >= chunk.total_chunks {
            return false;
        }

        let mask = 1u64 << chunk.chunk_idx;
        if self.received_mask & mask != 0 {
            return false;
        }

        let offset = chunk.chunk_idx as usize * MAX_CHUNK_DATA;
        let end = (offset + chunk.len).min(MAX_FRAME_SIZE);
        if offset < MAX_FRAME_SIZE {
            let copy_len = end - offset;
            self.data[offset..end].copy_from_slice(&chunk.data[..copy_len]);
        }

        self.received_mask |= mask;

        let expected_mask = (1u64 << self.total_chunks) - 1;
        if self.received_mask == expected_mask {
            self.len = (self.total_chunks as usize - 1) * MAX_CHUNK_DATA + chunk.len;
            self.ready_frame_id = self.assembling_frame_id;
            return true;
        }

        false
    }
}

pub static FRAME: Mutex<CriticalSectionRawMutex, RefCell<FrameBuffer>> =
    Mutex::new(RefCell::new(FrameBuffer::new()));

pub struct DecodedFrame {
    pub data: [u8; RGB565_BUF_SIZE],
    pub width: u16,
    pub height: u16,
    pub ready: bool,
    pub frame_id: u16,
}

impl DecodedFrame {
    const fn new() -> Self {
        Self {
            data: [0u8; RGB565_BUF_SIZE],
            width: 0,
            height: 0,
            ready: false,
            frame_id: 0,
        }
    }
}

pub static DECODED: Mutex<CriticalSectionRawMutex, RefCell<DecodedFrame>> =
    Mutex::new(RefCell::new(DecodedFrame::new()));

fn decode_frame_to_rgb565(jpeg_data: &[u8], frame_id: u16) -> bool {
    let mut decoder = JpegDecoder::new(jpeg_data);
    if decoder.decode_headers().is_err() {
        return false;
    }
    let Some((width, height)) = decoder.dimensions() else {
        return false;
    };
    let Ok(rgb) = decoder.decode() else {
        return false;
    };

    DECODED.lock(|d| {
        let mut d = d.borrow_mut();
        let pixel_count = width * height;

        if pixel_count * 2 > RGB565_BUF_SIZE {
            return;
        }

        for i in 0..pixel_count {
            let idx = i * 3;
            let r = rgb[idx] as u16;
            let g = rgb[idx + 1] as u16;
            let b = rgb[idx + 2] as u16;

            let raw = ((r >> 3) << 11) | ((g >> 2) << 5) | (b >> 3);
            let bytes = raw.to_be_bytes();
            d.data[i * 2] = bytes[0];
            d.data[i * 2 + 1] = bytes[1];
        }

        d.width = width as u16;
        d.height = height as u16;
        d.frame_id = frame_id;
        d.ready = true;
    });

    true
}

#[embassy_executor::task]
pub async fn frame_assembler(sender: Sender) {
    log::info!("frame_assembler started");

    loop {
        let chunk = FRAME_CHUNKS.receive().await;

        let decoded = FRAME.lock(|f| {
            let mut f = f.borrow_mut();
            if f.add_chunk(&chunk) {
                decode_frame_to_rgb565(&f.data[..f.len], f.ready_frame_id)
            } else {
                false
            }
        });

        if decoded {
            sender.publish_immediate(Event::Draw);
        }
    }
}

pub fn draw_decoded(buf: &mut mousefood::framebuffer::HeapBuffer<Rgb565>) {
    static LAST_DRAWN: Mutex<CriticalSectionRawMutex, RefCell<u16>> = Mutex::new(RefCell::new(0));

    DECODED.lock(|d| {
        let d = d.borrow();
        if !d.ready || d.width == 0 || d.height == 0 {
            return;
        }

        let already_drawn = LAST_DRAWN.lock(|last| {
            let mut last = last.borrow_mut();
            if *last == d.frame_id {
                return true;
            }
            *last = d.frame_id;
            false
        });

        if already_drawn {
            return;
        }

        let pixel_count = d.width as usize * d.height as usize;
        let byte_count = pixel_count * 2;

        let raw_image: ImageRawBE<Rgb565> = ImageRawBE::new(&d.data[..byte_count], d.width as u32);
        let image = Image::new(&raw_image, Point::new(0, 10));
        let _ = image.draw(buf);
    });
}

#[allow(dead_code)]
pub mod protocol {
    pub const PREFIX: [u8; 2] = [0xCA, 0x3E];

    pub const MSG_CAMERA_READY: u8 = 0x01;
    pub const MSG_CONNECT: u8 = 0x02;
    pub const MSG_FRAME_CHUNK: u8 = 0x03;
    pub const MSG_DISCONNECT: u8 = 0x04;

    // header: magic(2) + msg_type(1) + frame_id(2) + chunk_idx(2) + total_chunks(2) = 9 bytes
    pub const CHUNK_HEADER_SIZE: usize = 9;
    pub const CHUNK_DATA_SIZE: usize = 250 - CHUNK_HEADER_SIZE; // 241 bytes

    #[derive(Debug)]
    pub enum Message<'a> {
        CameraReady,
        Connect,
        FrameChunk {
            frame_id: u16,
            chunk_idx: u16,
            total_chunks: u16,
            data: &'a [u8],
        },
    }

    pub fn decode(data: &[u8]) -> Option<Message<'_>> {
        if data.len() < 3 || data[0..2] != PREFIX {
            return None;
        }
        let msg_type = data[2];
        match msg_type {
            MSG_CAMERA_READY => Some(Message::CameraReady),
            MSG_FRAME_CHUNK if data.len() >= CHUNK_HEADER_SIZE => {
                let frame_id = u16::from_le_bytes([data[3], data[4]]);
                let chunk_idx = u16::from_le_bytes([data[5], data[6]]);
                let total_chunks = u16::from_le_bytes([data[7], data[8]]);
                let payload = &data[CHUNK_HEADER_SIZE..];
                Some(Message::FrameChunk {
                    frame_id,
                    chunk_idx,
                    total_chunks,
                    data: payload,
                })
            }
            _ => None,
        }
    }

    pub fn encode_connect() -> [u8; 3] {
        [PREFIX[0], PREFIX[1], MSG_CONNECT]
    }

    pub fn encode_disconnect() -> [u8; 3] {
        [PREFIX[0], PREFIX[1], MSG_DISCONNECT]
    }
}
