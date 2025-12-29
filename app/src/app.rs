#[cfg(not(feature = "std"))]
use alloc::{string::String, vec::Vec};

use ratatui::{
    Frame, Terminal,
    buffer::Buffer,
    layout::Rect,
    prelude::{Backend, Widget},
    style::{Color, Style, Stylize},
    widgets::{Block, Padding, Paragraph, Tabs},
};
#[cfg(feature = "std")]
use std::{string::String, vec::Vec};

use crate::{
    events::{self, EVENTS, Event, Receiver, Sender},
    layout::AppLayout,
};

pub struct App {
    #[allow(dead_code)]
    sender: Sender,
    receiver: Receiver,
    exit: bool,
    pub layout: AppLayout,
    // exit_start: Option<Instant>,
    selected_tab: SelectedTab,
    battery_level: u8,
}

impl App {
    pub fn new() -> Self {
        let sender = EVENTS.publisher().unwrap();
        let receiver = EVENTS.subscriber().unwrap();
        Self {
            sender,
            receiver,
            exit: false,
            layout: AppLayout::new(Rect::default()),
            // exit_start: None,
            selected_tab: SelectedTab::Main,
            battery_level: 0,
        }
    }

    pub async fn run<B: Backend>(&mut self, terminal: &mut Terminal<B>) -> Result<(), B::Error> {
        self.layout = AppLayout::new(terminal.get_frame().area());

        while !self.exit {
            terminal.draw(|frame| self.draw(frame))?;

            // simulator dev: timeout prevents deadlock (SDL events polled in flush_callback)
            #[cfg(feature = "std")]
            {
                use embassy_time::Duration;
                match embassy_time::with_timeout(
                    Duration::from_millis(16),
                    self.receiver.next_message_pure(),
                )
                .await
                {
                    Ok(msg) => self.handle_events(msg).await,
                    Err(_) => {}
                }
            }

            #[cfg(not(feature = "std"))]
            {
                let msg = self.receiver.next_message_pure().await;
                self.handle_events(msg).await;
            }
        }

        Ok(())
    }

    pub fn next_tab(&mut self) {
        self.selected_tab = self.selected_tab.next();
    }

    fn draw(&self, frame: &mut Frame) {
        let AppLayout {
            header,
            main,
            footer,
        } = self.layout;

        let buf = frame.buffer_mut();

        self.draw_tabs(header, buf);

        match self.selected_tab {
            SelectedTab::Main => {}
            SelectedTab::Telegram => {}
            SelectedTab::Remote => {
                self.draw_remote(main, buf);
            }
            SelectedTab::Info => {}
        }

        self.draw_footer(footer, buf);
    }

    fn draw_tabs(&self, area: Rect, buf: &mut Buffer) {
        let titles = SelectedTab::titles();
        let selected_tab_index = self.selected_tab as usize;

        let bg_color = Color::Rgb(50, 50, 50);
        // TODO: re-enable exit timer
        // self
        //     .exit_start
        //     .and_then(|start| {
        //         let elapsed = start.elapsed();
        //         let held_ms = elapsed.as_millis();
        //         if held_ms < 300 {
        //             return None;
        //         }
        //
        //         Some(Color::Rgb(ms_to_red(held_ms), 10, 10))
        //     })
        //     .unwrap_or(Color::Rgb(10, 10, 10));

        Tabs::new(titles)
            .highlight_style(Style::new().fg(Color::White))
            .select(selected_tab_index)
            .padding("", "")
            .divider(" ")
            .block(
                Block::new()
                    .bg(bg_color)
                    .fg(Color::Black)
                    .padding(Padding::left(1)),
            )
            .render(area, buf);
    }

    fn draw_remote(&self, area: Rect, buf: &mut Buffer) {
        Paragraph::new("On/Off")
            .alignment(ratatui::layout::HorizontalAlignment::Center)
            .block(Block::bordered())
            .render(area, buf);
    }

    fn draw_footer(&self, area: Rect, buf: &mut Buffer) {
        #[cfg(feature = "esp-alloc")]
        let heap = {
            let free = esp_alloc::HEAP.free();
            let used = esp_alloc::HEAP.used();
            let total = free + used;

            format!(
                " b:{}% | h:{}/{}",
                self.battery_level,
                used / 1024,
                total / 1024
            )
        };

        #[cfg(not(feature = "esp-alloc"))]
        let heap = format!(" b:{}%", self.battery_level);

        let info = format!(" {}", heap);

        // TODO: re-enable exit timer
        // if let Some(start) = self.exit_start {
        //     let elapsed = start.elapsed();
        //     let held_ms = elapsed.as_millis();
        //     if held_ms > 300 {
        //         info = format!(" exit:{}ms", held_ms);
        //     }
        // }

        buf.set_string(
            0,
            area.bottom().saturating_sub(1),
            &info,
            Style::new().fg(Color::Gray),
        );
    }

    async fn handle_events(&mut self, event: Event) {
        match event {
            Event::ButtonUp(events::Button::A) => match self.selected_tab {
                SelectedTab::Remote => {
                    self.sender
                        .publish(Event::Remote(events::Remote::OnOff))
                        .await
                }
                _ => {}
            },
            Event::ButtonUp(events::Button::B) => {}
            Event::ButtonUp(events::Button::C) => {
                self.next_tab();
            }
            Event::BatteryLevelUpdated { level } => {
                self.battery_level = level;
            }
            _ => {}
        }
    }
}

#[derive(Clone, Copy)]
pub enum SelectedTab {
    Main,
    Telegram,
    Remote,
    Info,
}

impl SelectedTab {
    pub fn titles() -> Vec<String> {
        let mut titles = Vec::with_capacity(4);
        titles.push(Self::Main.title());
        titles.push(Self::Telegram.title());
        titles.push(Self::Remote.title());
        titles.push(Self::Info.title());
        titles
    }

    pub fn title(self) -> String {
        match self {
            SelectedTab::Main => "main",
            SelectedTab::Telegram => "tg",
            SelectedTab::Remote => "tv",
            SelectedTab::Info => "info",
        }
        .into()
    }

    pub fn next(self) -> Self {
        match self {
            SelectedTab::Main => SelectedTab::Telegram,
            SelectedTab::Telegram => SelectedTab::Remote,
            SelectedTab::Remote => SelectedTab::Info,
            SelectedTab::Info => SelectedTab::Main,
        }
    }
}

/// 0 -> 30, 3000 -> 255
pub fn ms_to_red(ms: u64) -> u8 {
    const MAX_MS: u64 = 3000;
    const MIN_RED: u64 = 10;
    const MAX_RED: u64 = 255;
    const RED_RANGE: u64 = MAX_RED - MIN_RED; // 225

    let clamped_ms = ms.min(MAX_MS);
    (MIN_RED + (clamped_ms * RED_RANGE) / MAX_MS) as u8
}
