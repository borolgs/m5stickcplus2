use app::{App, events};
use embassy_executor::Spawner;
use embedded_graphics::{pixelcolor::Rgb565, prelude::*};
use embedded_graphics_simulator::{
    OutputSettingsBuilder, SimulatorDisplay, SimulatorEvent, Window, sdl2,
};
use log::debug;
use mousefood::prelude::*;
use ratatui::Terminal;

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    env_logger::builder()
        .filter_level(log::LevelFilter::Debug)
        .format_timestamp_nanos()
        .init();

    let output_settings = OutputSettingsBuilder::new().scale(3).build();
    let mut window = Window::new(
        "M5StickC PLUS2 Simulator. Buttons: 1=A 2=B 3=C",
        &output_settings,
    );

    let mut display: SimulatorDisplay<Rgb565> = SimulatorDisplay::new(Size::new(240, 135));

    let channel = Box::leak(Box::new(app::events::channel()));

    let btn_sender = channel.publisher().unwrap();

    let backend_config = EmbeddedBackendConfig {
        flush_callback: Box::new(move |display| {
            window.update(display);

            for event in window.events() {
                match event {
                    SimulatorEvent::Quit => panic!("simulator window closed"),
                    SimulatorEvent::KeyDown { keycode, .. } => {
                        if let Some(button) = match keycode {
                            sdl2::Keycode::Num1 => Some(events::Button::A),
                            sdl2::Keycode::Num2 => Some(events::Button::B),
                            sdl2::Keycode::Num3 => Some(events::Button::C),
                            _ => None,
                        } {
                            debug!("Key {:?} pressed -> Button {:?}", keycode, button);
                            btn_sender.publish_immediate(events::Event::ButtonDown(button));
                        }
                    }
                    SimulatorEvent::KeyUp { keycode, .. } => {
                        if let Some(button) = match keycode {
                            sdl2::Keycode::Num1 => Some(events::Button::A),
                            sdl2::Keycode::Num2 => Some(events::Button::B),
                            sdl2::Keycode::Num3 => Some(events::Button::C),
                            _ => None,
                        } {
                            debug!("Key {:?} released -> Button {:?}", keycode, button);
                            btn_sender.publish_immediate(events::Event::ButtonUp(button));
                        }
                    }
                    _ => {}
                }
            }
        }),
        ..Default::default()
    };
    let backend: EmbeddedBackend<SimulatorDisplay<_>, _> =
        EmbeddedBackend::new(&mut display, backend_config);

    let mut terminal = Terminal::new(backend).unwrap();

    App::new(channel.publisher().unwrap(), channel.subscriber().unwrap())
        .run(&mut terminal)
        .await
        .unwrap();
}
