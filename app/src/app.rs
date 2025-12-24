#[cfg(not(feature = "std"))]
use alloc::{string::String, vec::Vec};

#[cfg(feature = "std")]
use std::{string::String, vec::Vec};
use ratatui::{
    Frame, Terminal,
    buffer::Buffer,
    layout::Rect,
    prelude::{Backend, Widget},
    style::{Color, Style, Stylize},
    widgets::{Block, Padding, Paragraph, Tabs},
};

use crate::{
    events::{self, Event, Receiver, Sender},
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
    battery_level: i32,
}

impl App {
    pub fn new(sender: Sender, receiver: Receiver, battery_level: i32) -> Self {
        Self {
            sender,
            receiver,
            exit: false,
            layout: AppLayout::new(Rect::default()),
            // exit_start: None,
            selected_tab: SelectedTab::Tab1,
            battery_level,
        }
    }

    pub async fn run<B: Backend>(&mut self, terminal: &mut Terminal<B>) -> Result<(), B::Error> {
        self.layout = AppLayout::new(terminal.get_frame().area());

        while !self.exit {
            terminal.draw(|frame| self.draw(frame))?;
            let msg = self.receiver.next_message_pure().await;
            self.handle_events(msg);
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

        let main_block = Block::new()
            .padding(Padding::left(1))
            .padding(Padding::top(1));

        Paragraph::new("hello").block(main_block).render(main, buf);

        self.draw_footer(footer, buf);
    }

    fn draw_tabs(&self, area: Rect, buf: &mut Buffer) {
        let titles = SelectedTab::titles();
        let selected_tab_index = self.selected_tab as usize;

        let bg_color = Color::Rgb(10, 10, 10);
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

    fn handle_events(&mut self, event: Event) {
        match event {
            Event::ButtonUp(events::Button::A) => {}
            Event::ButtonUp(events::Button::B) => {}
            Event::ButtonUp(events::Button::C) => {
                self.next_tab();
            }
            _ => {}
        }
    }
}

#[derive(Clone, Copy)]
pub enum SelectedTab {
    Tab1,
    Tab2,
    Tab3,
    Tab4,
}

impl SelectedTab {
    pub fn titles() -> Vec<String> {
        let mut titles = Vec::with_capacity(4);
        titles.push(Self::Tab1.title());
        titles.push(Self::Tab2.title());
        titles.push(Self::Tab3.title());
        titles.push(Self::Tab4.title());
        titles
    }

    pub fn title(self) -> String {
        match self {
            SelectedTab::Tab1 => "tab 1",
            SelectedTab::Tab2 => "tab 2",
            SelectedTab::Tab3 => "tab 3",
            SelectedTab::Tab4 => "tab 4",
        }
        .into()
    }

    pub fn next(self) -> Self {
        match self {
            SelectedTab::Tab1 => SelectedTab::Tab2,
            SelectedTab::Tab2 => SelectedTab::Tab3,
            SelectedTab::Tab3 => SelectedTab::Tab4,
            SelectedTab::Tab4 => SelectedTab::Tab1,
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
