#[cfg(not(feature = "std"))]
use alloc::{
    string::{String, ToString},
    vec::Vec,
};

use embassy_time::Instant;
use log::Level;
use ratatui::{
    Frame, Terminal,
    buffer::Buffer,
    layout::{Constraint, Layout, Margin, Rect},
    prelude::{Backend, Widget},
    style::{Color, Style, Stylize},
    text::Line,
    widgets::{Block, Padding, Paragraph, Tabs, Wrap},
};
#[cfg(feature = "std")]
use std::{string::String, vec::Vec};

use strum::IntoEnumIterator;

use crate::{
    Stats, StickHat,
    events::{self, EVENTS, Event, Receiver, Sender},
    layout::AppLayout,
    logger,
};

#[cfg(feature = "tv")]
use crate::remote::{TVRemote, TVState};

pub struct App {
    #[allow(dead_code)]
    sender: Sender,
    receiver: Receiver,
    exit: bool,
    layout: AppLayout,
    c_start: Option<Instant>,
    b_start: Option<Instant>,
    selected_tab: SelectedTab,
    tab_touched: bool,
    stats: events::Stats,
    #[cfg(feature = "tv")]
    tv: TVState,
    hat: Option<StickHat>,
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
            c_start: None,
            b_start: None,
            selected_tab: SelectedTab::Info,
            tab_touched: false,
            #[cfg(feature = "tv")]
            tv: TVState {
                current_btn: events::Remote::OnOff,
            },
            stats: Stats::default(),
            hat: None,
        }
    }

    pub async fn run<B: Backend>(&mut self, terminal: &mut Terminal<B>) -> Result<(), B::Error> {
        log::info!("Application started");
        self.layout = AppLayout::new(terminal.get_frame().area());

        while !self.exit {
            terminal.draw(|frame| self.draw(frame))?;

            let msg = self.receiver.next_message_pure().await;

            self.handle_events(msg).await;
        }

        Ok(())
    }

    fn next_tab(&mut self) {
        #[cfg(feature = "tv")]
        {
            self.tv.current_btn = events::Remote::OnOff;
        }
        self.selected_tab = self.selected_tab.next();
        self.tab_touched = false;
    }

    #[allow(unused)]
    fn prev_tab(&mut self) {
        self.selected_tab = self.selected_tab.prev();
        self.tab_touched = false;
    }

    fn touch_tab(&mut self) {
        self.tab_touched = true;
    }

    fn c_held_time(&self) -> u64 {
        if let Some(start) = self.c_start {
            let elapsed = start.elapsed();
            let held_ms = elapsed.as_millis();
            return held_ms;
        }

        0
    }

    #[allow(unused)]
    fn b_held_time(&self) -> u64 {
        if let Some(start) = self.b_start {
            let elapsed = start.elapsed();
            let held_ms = elapsed.as_millis();
            return held_ms;
        }

        0
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
            #[cfg(feature = "controller")]
            SelectedTab::Controller => self.draw_controller(main, buf),
            #[cfg(feature = "vehicle")]
            SelectedTab::Vehicle => self.draw_vehicle(main, buf),
            #[cfg(feature = "tv")]
            SelectedTab::Remote => self.draw_remote(main, buf),
            SelectedTab::Info => self.draw_info(main, buf),
            SelectedTab::Dev => self.draw_dev(main, buf),
        }

        self.draw_footer(footer, buf);
    }

    fn draw_tabs(&self, area: Rect, buf: &mut Buffer) {
        let titles = SelectedTab::titles();
        let selected_tab_index = self.selected_tab as usize;

        let bg_color = Color::Rgb(
            if self.c_held_time() < 500 {
                50
            } else {
                ms_to_red(self.c_held_time())
            },
            50,
            50,
        );

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

    #[cfg(feature = "tv")]
    fn draw_remote(&self, area: Rect, buf: &mut Buffer) {
        use ratatui::prelude::StatefulWidget;
        TVRemote::new().render(area, buf, &mut self.tv.clone());
    }

    #[cfg(feature = "controller")]
    fn draw_controller(&self, area: Rect, buf: &mut Buffer) {
        // TODO
        Paragraph::new("Vehicle Controller")
            .centered()
            .block(Block::new().padding(Padding::top(1)))
            .render(area, buf);
    }

    #[cfg(feature = "vehicle")]
    fn draw_vehicle(&self, area: Rect, buf: &mut Buffer) {
        // TODO
        Paragraph::new("Vehicle")
            .centered()
            .block(Block::new().padding(Padding::top(1)))
            .render(area, buf);
    }

    fn draw_dev(&self, area: Rect, buf: &mut Buffer) {
        let log_lines = logger::latest_log_lines(area.height as usize);
        let log_lines = log_lines
            .iter()
            .rev()
            .map(|(level, msg)| {
                let color = match level {
                    Level::Error => Color::Red,
                    Level::Warn => Color::Yellow,
                    Level::Info => Color::White,
                    Level::Debug => Color::DarkGray,
                    Level::Trace => Color::Magenta,
                };

                Line::styled(msg, Style::new().fg(color))
            })
            .collect::<Vec<_>>();

        Paragraph::new(log_lines)
            .wrap(Wrap { trim: true })
            .render(area, buf);
    }

    fn draw_info(&self, area: Rect, buf: &mut Buffer) {
        let info = [
            ("battery", format!("{}%", self.stats.battery_level)),
            (
                "hat",
                self.hat
                    .map(|h| format!("{:?}", h))
                    .unwrap_or("-".to_string()),
            ),
            (
                "heap",
                format!(
                    "{}/{}",
                    self.stats.heap_used / 1024,
                    (self.stats.heap_used + self.stats.heap_free) / 1024
                ),
            ),
        ];
        let horizontal = Layout::horizontal([Constraint::Max(10), Constraint::Fill(1)]);
        let vertical = Layout::vertical((0..info.len()).map(|_| Constraint::Length(1)));

        let rows = vertical.split(area.inner(Margin::new(1, 1)));

        let info_style = Style::new().fg(Color::DarkGray);

        for (row, (title, value)) in rows.iter().cloned().zip(info) {
            let [title_area, value_area] = horizontal.areas(row);

            Paragraph::new(title)
                .style(info_style)
                .render(title_area, buf);
            Paragraph::new(value).render(value_area, buf);
        }
    }

    fn draw_footer(&self, area: Rect, buf: &mut Buffer) {
        if self.c_held_time() > 1000 {
            buf.set_string(
                0,
                area.bottom().saturating_sub(1),
                " hold to shut down...",
                Style::new().fg(Color::Gray),
            );
            return;
        }

        match self.selected_tab {
            #[cfg(feature = "tv")]
            SelectedTab::Remote if !matches!(self.hat, Some(events::StickHat::MiniJoyC)) => {
                let c_mode = {
                    let mut mode = "c - next tab";

                    if self.tab_touched {
                        if self.c_held_time() < 500 {
                            mode = "c - prev btn";
                        }
                    }

                    mode
                };

                let b_mode = {
                    let mut mode = "b - next btn";

                    if self.b_held_time() > 300 {
                        mode = "b - next row"
                    }

                    mode
                };

                let btns = [c_mode, b_mode]
                    .iter()
                    .map(|s| s.to_string())
                    .filter(|s| !s.is_empty())
                    .collect::<Vec<_>>()
                    .join(", ");

                let info = format!(" {}", btns);

                buf.set_string(
                    0,
                    area.bottom().saturating_sub(1),
                    &info,
                    Style::new().fg(Color::Gray),
                );
            }
            _ => {}
        }
    }

    async fn handle_events(&mut self, event: Event) {
        match event {
            Event::InitHat(hat) => {
                self.hat = Some(hat);
            }
            #[allow(unused)]
            Event::JoyC(joyc_event) => {
                self.touch_tab();
                match self.selected_tab {
                    #[cfg(feature = "controller")]
                    SelectedTab::Controller => match joyc_event {
                        crate::JoyC::Pos((x, y)) => {
                            use crate::Controller;

                            let x = x as i16;
                            let y = y as i16;

                            let left = (y + x).clamp(-100, 100) as i8;
                            let right = (y - x).clamp(-100, 100) as i8;

                            self.sender
                                .publish(Event::Controller(Controller::Move(left, right)))
                                .await;
                        }
                        _ => {}
                    },
                    #[cfg(feature = "tv")]
                    SelectedTab::Remote => match joyc_event {
                        crate::JoyC::Button => {
                            self.sender
                                .publish(Event::Remote(self.tv.current_btn))
                                .await
                        }
                        crate::JoyC::Arrow(dir) => match dir {
                            crate::JoycDirection::Up => {
                                self.tv.prev_row();
                            }
                            crate::JoycDirection::Right => {
                                self.tv.next_btn();
                            }
                            crate::JoycDirection::Down => {
                                self.tv.next_row();
                            }
                            crate::JoycDirection::Left => {
                                self.tv.prev_btn();
                            }
                            crate::JoycDirection::Center => {}
                        },
                        _ => {}
                    },
                    _ => {}
                };
            }
            Event::ButtonUp(events::Button::A) => match self.selected_tab {
                #[cfg(feature = "tv")]
                SelectedTab::Remote => {
                    self.touch_tab();
                    self.sender
                        .publish(Event::Remote(self.tv.current_btn))
                        .await
                }
                _ => {}
            },
            Event::ButtonDown(events::Button::B) => {
                if self.b_start.is_none() {
                    self.b_start = Some(Instant::now());
                }
            }
            Event::ButtonUp(events::Button::B) => {
                match self.selected_tab {
                    #[cfg(feature = "tv")]
                    SelectedTab::Remote => {
                        self.touch_tab();
                        if self.b_held_time() > 300 {
                            self.tv.next_row();
                        } else {
                            self.tv.next_btn();
                        }
                    }
                    _ => {}
                };
                self.b_start = None;
            }
            Event::ButtonDown(events::Button::C) => {
                if self.c_start.is_none() {
                    self.c_start = Some(Instant::now());
                }
            }
            Event::ButtonUp(events::Button::C) => {
                if matches!(self.hat, Some(events::StickHat::MiniJoyC)) {
                    self.next_tab();
                } else if self.tab_touched {
                    if self.c_held_time() > 500 {
                        self.next_tab();
                    } else {
                        match self.selected_tab {
                            #[cfg(feature = "tv")]
                            SelectedTab::Remote => {
                                self.tv.prev_btn();
                            }
                            _ => {}
                        }
                    }
                } else {
                    self.next_tab();
                }
                self.c_start = None;
            }
            Event::StatsUpdated(stats) => {
                self.stats = stats;
            }
            _ => {}
        }
    }
}

#[derive(
    Clone, Copy, PartialEq, strum::EnumIter, strum::EnumCount, strum::FromRepr, strum::Display,
)]
pub enum SelectedTab {
    #[strum(to_string = "info")]
    Info,
    #[cfg(feature = "controller")]
    #[strum(to_string = "ctrl")]
    Controller,
    #[cfg(feature = "vehicle")]
    #[strum(to_string = "vehicle")]
    Vehicle,
    #[cfg(feature = "tv")]
    #[strum(to_string = "tv")]
    Remote,
    #[strum(to_string = "dev")]
    Dev,
}

impl SelectedTab {
    pub fn next(self) -> Self {
        Self::from_repr((self as usize + 1) % Self::iter().len()).unwrap()
    }

    pub fn prev(self) -> Self {
        let len = Self::iter().len();
        Self::from_repr((self as usize + len - 1) % len).unwrap()
    }

    pub fn titles() -> Vec<String> {
        Self::iter().map(|t| t.to_string()).collect::<Vec<_>>()
    }
}

/// 0 -> 30, 3000 -> 255
pub fn ms_to_red(ms: u64) -> u8 {
    const MAX_MS: u64 = 3000;
    const MIN_RED: u64 = 50;
    const MAX_RED: u64 = 255;
    const RED_RANGE: u64 = MAX_RED - MIN_RED; // 225

    let clamped_ms = ms.min(MAX_MS);
    (MIN_RED + (clamped_ms * RED_RANGE) / MAX_MS) as u8
}
