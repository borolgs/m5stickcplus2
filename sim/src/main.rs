use embedded_graphics::{pixelcolor::Rgb565, prelude::*};
use embedded_graphics_simulator::{
    OutputSettingsBuilder, SimulatorDisplay, SimulatorEvent, Window,
};
use std::thread;
use std::time::Duration;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create a simulated display (240x135 to match M5StickC PLUS2 in landscape)
    let mut display: SimulatorDisplay<Rgb565> = SimulatorDisplay::new(Size::new(240, 135));

    let output_settings = OutputSettingsBuilder::new().scale(3).build();
    let mut window = Window::new("M5StickC PLUS2 Simulator", &output_settings);

    'running: loop {
        // Clear display
        display.clear(Rgb565::BLACK)?;

        // Update window
        window.update(&display);

        // Handle events
        for event in window.events() {
            match event {
                SimulatorEvent::Quit => break 'running,
                _ => {}
            }
        }

        thread::sleep(Duration::from_millis(16)); // ~60 FPS
    }

    Ok(())
}
