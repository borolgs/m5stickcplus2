use esp_hal::gpio::Input;

pub struct Buttons {
    pub a: Button<'static>,
    pub b: Button<'static>,
    pub c: Button<'static>,
}

impl Buttons {
    pub fn new(a: Input<'static>, b: Input<'static>, c: Input<'static>) -> Self {
        Self {
            a: Button::new(a),
            b: Button::new(b),
            c: Button::new(c),
        }
    }
    pub fn update(&mut self) {
        self.a.update();
        self.b.update();
        self.c.update();
    }
}

pub struct Button<'a> {
    input: Input<'a>,
    prev_state: bool,
    just_pressed: bool,
    changed: bool,
}

impl<'a> Button<'a> {
    pub fn new(input: Input<'a>) -> Self {
        Self {
            input,
            prev_state: false,
            just_pressed: false,
            changed: false,
        }
    }

    pub fn update(&mut self) {
        let pressed = self.input.is_low();
        self.just_pressed = pressed && !self.prev_state;
        self.changed = pressed != self.prev_state;
        self.prev_state = pressed;
    }

    pub fn just_pressed(&self) -> bool {
        self.just_pressed
    }

    pub fn changed(&self) -> bool {
        self.changed
    }

    pub fn is_pressed(&self) -> bool {
        self.input.is_low()
    }
}
