use esp_hal::gpio::Input;
use log::debug;

use app::events::{self, Event, Sender};

pub struct Buttons {
    sender: Sender,
    pub a: Button<'static>,
    pub b: Button<'static>,
    pub c: Button<'static>,
}

impl Buttons {
    pub fn new(sender: Sender, a: Input<'static>, b: Input<'static>, c: Input<'static>) -> Self {
        Self {
            sender,
            a: Button::new(a),
            b: Button::new(b),
            c: Button::new(c),
        }
    }
    pub async fn update(&mut self) {
        self.a.update();
        self.b.update();
        self.c.update();

        if self.a.just_pressed() {
            debug!("Button A pressed");
            self.sender
                .publish(Event::ButtonDown(events::Button::A))
                .await;
        }

        if self.a.just_released() {
            debug!("Button A released");
            self.sender
                .publish(Event::ButtonUp(events::Button::A))
                .await;
        }

        if self.b.just_pressed() {
            debug!("Button B pressed");
            self.sender
                .publish(Event::ButtonDown(events::Button::B))
                .await;
        }

        if self.b.just_released() {
            debug!("Button B released");
            self.sender
                .publish(Event::ButtonUp(events::Button::B))
                .await;
        }

        if self.c.just_pressed() {
            debug!("Button C pressed");
            self.sender
                .publish(Event::ButtonDown(events::Button::C))
                .await;
        }

        if self.c.just_released() {
            debug!("Button C released");
            self.sender
                .publish(Event::ButtonUp(events::Button::C))
                .await;
        }

        if self.a.is_pressed() | self.b.is_pressed() | self.c.is_pressed() {
            self.sender.publish(Event::Draw).await;
        }
    }
}

pub struct Button<'a> {
    input: Input<'a>,
    prev_state: bool,
    just_pressed: bool,
    just_released: bool,
    changed: bool,
}

impl<'a> Button<'a> {
    pub fn new(input: Input<'a>) -> Self {
        Self {
            input,
            prev_state: false,
            just_pressed: false,
            just_released: false,
            changed: false,
        }
    }

    pub fn update(&mut self) {
        let pressed = self.input.is_low();
        self.just_pressed = pressed && !self.prev_state;
        self.just_released = !pressed && self.prev_state;
        self.changed = pressed != self.prev_state;
        self.prev_state = pressed;
    }

    pub fn just_pressed(&self) -> bool {
        self.just_pressed
    }

    pub fn just_released(&self) -> bool {
        self.just_released
    }

    pub fn changed(&self) -> bool {
        self.changed
    }

    pub fn is_pressed(&self) -> bool {
        self.input.is_low()
    }
}
