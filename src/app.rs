use alloc::{string::String, vec::Vec};
use esp_hal::time::Instant;
use log::info;
use ratatui::{
    Frame, Terminal,
    buffer::Buffer,
    layout::Rect,
    prelude::{Backend, Widget},
    style::{Color, Style, Stylize},
    widgets::{Block, Padding, Paragraph, Tabs},
};

use crate::{button::Buttons, layout::AppLayout};

pub struct App {
    exit: bool,
    pub layout: AppLayout,
    buttons: Buttons,
    exit_start: Option<Instant>,
    selected_tab: SelectedTab,
}

impl App {
    pub fn new(buttons: Buttons) -> Self {
        Self {
            exit: false,
            buttons: buttons,
            layout: AppLayout::new(Rect::default()),
            exit_start: None,
            selected_tab: SelectedTab::Tab1,
        }
    }

    pub fn run<B: Backend>(&mut self, terminal: &mut Terminal<B>) -> Result<(), B::Error> {
        self.layout = AppLayout::new(terminal.get_frame().area());

        while !self.exit {
            terminal.draw(|frame| self.draw(frame))?;
            self.handle_events::<B>()?;
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

        let bg_color = self
            .exit_start
            .and_then(|start| {
                let elapsed = start.elapsed();
                let held_ms = elapsed.as_millis();
                if held_ms < 300 {
                    return None;
                }

                Some(Color::Rgb(ms_to_red(held_ms), 10, 10))
            })
            .unwrap_or(Color::Rgb(10, 10, 10));

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
        let heap = {
            let free = esp_alloc::HEAP.free();
            let used = esp_alloc::HEAP.used();
            let total = free + used;

            format!("h:{}/{}", used / 1024, total / 1024)
        };

        let mut info = format!(" {}", heap);

        if let Some(start) = self.exit_start {
            let elapsed = start.elapsed();
            let held_ms = elapsed.as_millis();
            if held_ms > 300 {
                info = format!(" exit:{}ms", held_ms);
            }
        }

        buf.set_string(
            0,
            area.bottom().saturating_sub(1),
            &info,
            Style::new().fg(Color::Gray),
        );
    }

    fn handle_events<B: Backend>(&mut self) -> Result<(), B::Error> {
        self.buttons.update();

        if self.buttons.a.just_pressed() {
            info!("Button A pressed");
        }

        if self.buttons.b.just_pressed() {
            info!("Button B pressed");
        }

        if self.buttons.c.just_pressed() {
            self.exit_start = Some(Instant::now());
            self.next_tab();
            info!("Button C pressed");
        }
        if !self.buttons.c.is_pressed() {
            self.exit_start = None
        }

        Ok(())
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
