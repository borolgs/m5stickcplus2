use app::{Event, Sender, StickHat};
use embassy_time::{Duration, Timer};
use esp_hal::{
    Async,
    i2c::master::{Error, I2c},
};

const JOYC_ADDR: u8 = 0x54;

const POS_VALUE_REG_8_BIT: u8 = 0x20;
const BUTTON_REG: u8 = 0x30;
const RGB_LED_REG: u8 = 0x40;
#[allow(dead_code)]
const CAL_REG: u8 = 0x50;
pub const FIRMWARE_VERSION_REG: u8 = 0xFE;
#[allow(dead_code)]
const I2C_ADDRESS_REG: u8 = 0xFF;

pub struct MiniJoyC {
    i2c: I2c<'static, Async>,
    sender: Sender,
    prev_direction: Option<JoyDirection>,
    prev_button: Option<bool>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum JoyDirection {
    Up,
    Down,
    Left,
    Right,
}

impl MiniJoyC {
    pub fn new(i2c: I2c<'static, Async>, sender: Sender) -> Self {
        Self {
            i2c,
            sender,
            prev_direction: None,
            prev_button: None,
        }
    }

    pub async fn is_connected(&mut self) -> bool {
        self.get_firmware_version()
            .await
            .map(|_| true)
            .unwrap_or(false)
    }

    pub async fn run(&mut self) {
        self.sender
            .publish(app::Event::InitHat(StickHat::MiniJoyC))
            .await;
        loop {
            self.poll().await;
            Timer::after(Duration::from_millis(50)).await;
        }
    }

    async fn poll(&mut self) {
        let Ok((x, y)) = self.read_position().await else {
            return;
        };
        let Ok(button) = self.read_button().await else {
            return;
        };

        let direction = Self::classify_direction(x, y, 35);

        if direction != self.prev_direction {
            let event = match direction {
                Some(JoyDirection::Up) => Event::JoyC(app::JoyC::Pos {
                    dir: app::JoycDirection::Up,
                    val: (x, y),
                }),
                Some(JoyDirection::Down) => Event::JoyC(app::JoyC::Pos {
                    dir: app::JoycDirection::Down,
                    val: (x, y),
                }),
                Some(JoyDirection::Left) => Event::JoyC(app::JoyC::Pos {
                    dir: app::JoycDirection::Left,
                    val: (x, y),
                }),
                Some(JoyDirection::Right) => Event::JoyC(app::JoyC::Pos {
                    dir: app::JoycDirection::Right,
                    val: (x, y),
                }),
                None => Event::JoyC(app::JoyC::Pos {
                    dir: app::JoycDirection::Center,
                    val: (x, y),
                }),
            };
            self.sender.publish(event).await;

            self.prev_direction = direction;
        }

        if self.prev_button.is_some() && button && !self.prev_button.unwrap_or_default() {
            self.sender.publish(Event::JoyC(app::JoyC::Button)).await;
        }
        self.prev_button = Some(button);
    }

    fn classify_direction(x: i8, y: i8, threshold: i8) -> Option<JoyDirection> {
        if x.abs() > y.abs() {
            if x > threshold {
                Some(JoyDirection::Down)
            } else if x < -threshold {
                Some(JoyDirection::Up)
            } else {
                None
            }
        } else if y > threshold {
            Some(JoyDirection::Right)
        } else if y < -threshold {
            Some(JoyDirection::Left)
        } else {
            None
        }
    }

    async fn read_position(&mut self) -> Result<(i8, i8), Error> {
        let mut x_buf = [0u8; 1];
        let mut y_buf = [0u8; 1];

        self.i2c
            .write_read_async(JOYC_ADDR, &[POS_VALUE_REG_8_BIT], &mut x_buf)
            .await?;

        self.i2c
            .write_read_async(JOYC_ADDR, &[POS_VALUE_REG_8_BIT + 1], &mut y_buf)
            .await?;

        Ok((x_buf[0] as i8, y_buf[0] as i8))
    }

    async fn read_button(&mut self) -> Result<bool, Error> {
        let mut buf = [0u8; 1];
        self.i2c
            .write_read_async(JOYC_ADDR, &[BUTTON_REG], &mut buf)
            .await?;
        Ok(buf[0] != 0)
    }

    pub async fn get_firmware_version(&mut self) -> Result<u8, Error> {
        let mut buf = [0u8; 1];
        self.i2c
            .write_read_async(JOYC_ADDR, &[FIRMWARE_VERSION_REG], &mut buf)
            .await?;
        Ok(buf[0])
    }

    #[allow(dead_code)]
    pub async fn set_led_color(&mut self, r: u8, g: u8, b: u8) -> Result<(), Error> {
        self.i2c
            .write_async(JOYC_ADDR, &[RGB_LED_REG, r, g, b])
            .await
    }
}
