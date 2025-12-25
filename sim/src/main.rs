use app::App;
use embassy_executor::Spawner;
use embassy_time::Timer;
use embedded_graphics::{pixelcolor::Rgb565, prelude::*};
use embedded_graphics_simulator::{
    OutputSettingsBuilder, SimulatorDisplay, SimulatorEvent, Window,
};
use log::info;
use mousefood::prelude::*;
use ratatui::Terminal;

#[embassy_executor::task]
async fn run() {
    loop {
        info!("tick");
        Timer::after_secs(1).await;
    }
}

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    env_logger::builder()
        .filter_level(log::LevelFilter::Debug)
        .format_timestamp_nanos()
        .init();

    let output_settings = OutputSettingsBuilder::new().scale(3).build();
    let mut window = Window::new("M5StickC PLUS2 Simulator", &output_settings);

    let mut display: SimulatorDisplay<Rgb565> = SimulatorDisplay::new(Size::new(240, 135));

    let backend_config = EmbeddedBackendConfig {
        flush_callback: Box::new(move |display| {
            window.update(display);
            if window.events().any(|e| e == SimulatorEvent::Quit) {
                panic!("simulator window closed");
            }
        }),
        ..Default::default()
    };
    let backend: EmbeddedBackend<SimulatorDisplay<_>, _> =
        EmbeddedBackend::new(&mut display, backend_config);

    let mut terminal = Terminal::new(backend).unwrap();

    let channel = Box::leak(Box::new(app::events::channel()));

    spawner.spawn(run()).unwrap();

    App::new(channel.publisher().unwrap(), channel.subscriber().unwrap())
        .run(&mut terminal)
        .await
        .unwrap();
}
