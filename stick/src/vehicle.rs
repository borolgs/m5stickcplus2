use app::{Button, Event, Receiver, Vehicle};
use esp_hal::{
    Async,
    i2c::master::{Error, I2c},
};

const HAT_8SERVOS_ADDR: u8 = 0x36;
const SERVO_ANGLE_REG: u8 = 0x00;
const SERVO_PULSE_REG: u8 = 0x10;
const SERVO_POWER_REG: u8 = 0x30;

const LEFT_CHANNEL: u8 = 1;
const RIGHT_CHANNEL: u8 = 5;

pub fn speed_to_angle(speed: i8) -> u8 {
    // speed: -100..100 -> angle: 0..180
    (90 + (speed as i16) * 90 / 100) as u8
}

#[embassy_executor::task]
pub async fn vehicle_task(mut receiver: Receiver, mut servos: Hat8Servos) {
    servos.enable_power(true).await.ok();

    let mut power_on = true;

    loop {
        let msg = receiver.next_message_pure().await;

        match msg {
            Event::Vehicle(Vehicle::Move(left, right)) => {
                servos
                    .set_servo_angle(LEFT_CHANNEL, speed_to_angle(left))
                    .await
                    .ok();
                servos
                    .set_servo_angle(RIGHT_CHANNEL, speed_to_angle(right))
                    .await
                    .ok();
            }
            Event::ButtonUp(Button::A) => {
                if power_on {
                    servos.enable_power(false).await.ok();
                    power_on = false;
                } else {
                    servos.enable_power(true).await.ok();
                    power_on = true;
                }
            }
            _ => continue,
        }
    }
}

pub struct Hat8Servos {
    i2c: I2c<'static, Async>,
}

impl Hat8Servos {
    pub fn new(i2c: I2c<'static, Async>) -> Self {
        Self { i2c }
    }

    pub async fn is_connected(&mut self) -> bool {
        match self.i2c.write_async(HAT_8SERVOS_ADDR, &[]).await {
            Ok(_) => {
                log::info!("Hat8Servos ACK received");
                true
            }
            Err(e) => {
                log::warn!("Hat8Servos no ACK: {:?}", e);
                false
            }
        }
    }

    pub async fn enable_power(&mut self, enable: bool) -> Result<(), Error> {
        let state = if enable { 1u8 } else { 0u8 };
        log::debug!("Hat8Servos enable_power({})", enable);
        let result = self
            .i2c
            .write_async(HAT_8SERVOS_ADDR, &[SERVO_POWER_REG, state])
            .await;
        if let Err(ref e) = result {
            log::error!("Hat8Servos enable_power failed: {:?}", e);
        }
        result
    }

    pub async fn set_servo_angle(&mut self, channel: u8, angle: u8) -> Result<(), Error> {
        let angle = angle.min(180);
        let reg = SERVO_ANGLE_REG + channel;
        log::info!(
            "set_servo_angle: ch={} angle={} -> I2C write [0x{:02X}, 0x{:02X}]",
            channel,
            angle,
            reg,
            angle
        );
        let result = self.i2c.write_async(HAT_8SERVOS_ADDR, &[reg, angle]).await;
        if let Err(ref e) = result {
            log::error!("set_servo_angle I2C error: {:?}", e);
        }
        result
    }

    pub async fn set_servo_pulse(&mut self, channel: u8, pulse: u16) -> Result<(), Error> {
        let reg = SERVO_PULSE_REG + (channel * 2);
        let high = (pulse >> 8) as u8;
        let low = (pulse & 0xff) as u8;
        log::info!(
            "set_servo_pulse: ch={} pulse={} -> I2C write [0x{:02X}, 0x{:02X}, 0x{:02X}]",
            channel,
            pulse,
            reg,
            high,
            low
        );
        let result = self
            .i2c
            .write_async(HAT_8SERVOS_ADDR, &[reg, high, low])
            .await;
        if let Err(ref e) = result {
            log::error!("set_servo_pulse I2C error: {:?}", e);
        }
        result
    }

    #[allow(dead_code)]
    pub async fn get_servo_angle(&mut self, channel: u8) -> Result<u8, Error> {
        let mut buf = [0u8; 1];
        let reg = SERVO_ANGLE_REG + channel;
        self.i2c
            .write_read_async(HAT_8SERVOS_ADDR, &[reg], &mut buf)
            .await?;
        Ok(buf[0])
    }
}
