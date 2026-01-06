use app::{
    App, Sender,
    events::{self, EVENTS, Receiver},
    logger,
};
use embassy_executor::Spawner;
use embassy_time::{Duration, Timer};
use embedded_graphics::{pixelcolor::Rgb565, prelude::*};
use embedded_graphics_simulator::{
    OutputSettingsBuilder, SimulatorDisplay, SimulatorEvent, Window, sdl2,
};
use log::{debug, info};
use mousefood::prelude::*;
use ratatui::Terminal;

#[embassy_executor::task]
async fn draw_task(sender: Sender) {
    loop {
        sender.publish(app::Event::Draw).await;
        Timer::after(Duration::from_millis(50)).await;
    }
}

#[embassy_executor::task]
async fn event_handler(mut receiver: Receiver) {
    loop {
        let event = receiver.next_message_pure().await;
        if !matches!(event, app::Event::Draw | app::Event::LogAdded) {
            info!("Message from app: {:?}", event);
        }
    }
}

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    // env_logger::builder()
    //     .filter_level(log::LevelFilter::Debug)
    //     .format_timestamp_nanos()
    //     .init();
    logger::init();

    let output_settings = OutputSettingsBuilder::new().scale(3).build();
    let mut window = Window::new(
        "M5StickC PLUS2 Simulator. Buttons: 1=C 2=A 3=B, Joystick: arrows + space",
        &output_settings,
    );

    let mut display: SimulatorDisplay<Rgb565> = SimulatorDisplay::new(Size::new(240, 135));

    let btn_sender = EVENTS.publisher().unwrap();

    let backend_config = EmbeddedBackendConfig {
        flush_callback: Box::new(move |display| {
            window.update(display);

            for event in window.events() {
                match event {
                    SimulatorEvent::Quit => panic!("simulator window closed"),
                    SimulatorEvent::KeyDown { keycode, .. } => {
                        if let Some(button) = match keycode {
                            sdl2::Keycode::Num1 => Some(events::Button::C),
                            sdl2::Keycode::Num2 => Some(events::Button::A),
                            sdl2::Keycode::Num3 => Some(events::Button::B),
                            _ => None,
                        } {
                            debug!("Key {:?} pressed -> Button {:?}", keycode, button);
                            btn_sender.publish_immediate(events::Event::ButtonDown(button));
                        }

                        if let Some(joyc_evt) = match keycode {
                            sdl2::Keycode::DOWN => {
                                Some(events::JoyC::Arrow(events::JoycDirection::Down))
                            }
                            sdl2::Keycode::Up => {
                                Some(events::JoyC::Arrow(events::JoycDirection::Up))
                            }
                            sdl2::Keycode::LEFT => {
                                Some(events::JoyC::Arrow(events::JoycDirection::Left))
                            }
                            sdl2::Keycode::RIGHT => {
                                Some(events::JoyC::Arrow(events::JoycDirection::Right))
                            }
                            sdl2::Keycode::SPACE => Some(events::JoyC::Button),
                            _ => None,
                        } {
                            debug!("Joystick {:?} -> {:?}", keycode, joyc_evt);
                            btn_sender.publish_immediate(events::Event::JoyC(joyc_evt));
                        }
                    }
                    SimulatorEvent::KeyUp { keycode, .. } => {
                        if let Some(button) = match keycode {
                            sdl2::Keycode::Num1 => Some(events::Button::C),
                            sdl2::Keycode::Num2 => Some(events::Button::A),
                            sdl2::Keycode::Num3 => Some(events::Button::B),
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

    let mut app = App::new();

    spawner
        .spawn(event_handler(EVENTS.subscriber().unwrap()))
        .unwrap();

    spawner
        .spawn(draw_task(EVENTS.publisher().unwrap()))
        .unwrap();

    EVENTS
        .publisher()
        .unwrap()
        .publish(app::Event::InitHat(app::StickHat::MiniJoyC))
        .await;

    app.run(&mut terminal).await.unwrap();
}
