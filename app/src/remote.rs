#[cfg(not(feature = "std"))]
use alloc::{string::String, vec::Vec};

use ratatui::{
    buffer::Buffer,
    layout::{Constraint, HorizontalAlignment, Layout, Rect},
    prelude::StatefulWidget,
    style::{Color, Style},
    widgets::{Block, Borders, Padding, Paragraph, Widget},
};

use strum::IntoEnumIterator;

use crate::Remote;

pub enum ActiveRemoteButton {
    OnOff,
}

#[derive(Debug, Clone, Copy)]
pub struct TVState {
    pub current_btn: Remote,
}

impl TVState {
    pub fn new() -> Self {
        Self {
            current_btn: Remote::OnOff,
        }
    }

    pub fn next_row(&mut self) -> Remote {
        let cols = 4;
        let len = Remote::iter().len();
        let col = self.current_btn as usize % cols;
        let new_idx = self.current_btn as usize + cols;

        self.current_btn = if new_idx >= len {
            Remote::from_repr(col).unwrap()
        } else {
            Remote::from_repr(new_idx).unwrap()
        };
        self.current_btn
    }

    pub fn next_btn(&mut self) -> Remote {
        self.current_btn =
            Remote::from_repr((self.current_btn as usize + 1) % Remote::iter().len()).unwrap();
        self.current_btn
    }

    pub fn prev_btn(&mut self) -> Remote {
        let len = Remote::iter().len();
        self.current_btn = Remote::from_repr((self.current_btn as usize + len - 1) % len).unwrap();
        self.current_btn
    }
}

pub struct TVRemote {
    cols: usize,
    rows: usize,
}

impl TVRemote {
    pub fn new() -> Self {
        Self { cols: 4, rows: 3 }
    }
}

impl From<Remote> for String {
    fn from(value: Remote) -> Self {
        match value {
            Remote::OnOff => "on",
            Remote::Home => "home",
            Remote::Back => "back",
            Remote::Ok => "ok",
            Remote::Up => "↑",
            Remote::Right => "→",
            Remote::Down => "↓",
            Remote::Left => "←",
            Remote::Mute => "mute",
            Remote::VolumeUp => "vol ↑",
            Remote::VolumeDown => "vol ↓",
        }
        .into()
    }
}

impl StatefulWidget for TVRemote {
    type State = TVState;
    fn render(self, area: Rect, buf: &mut Buffer, state: &mut TVState) {
        let col_constraints = (0..self.cols).map(|_| Constraint::Min(3));
        let row_constraints = (0..self.rows).map(|_| Constraint::Length(3));
        let horizontal = Layout::horizontal(col_constraints);
        let vertical = Layout::vertical(row_constraints);

        let rows = vertical.split(area);
        let cells = rows
            .iter()
            .flat_map(|&row| horizontal.split(row).to_vec())
            .collect::<Vec<_>>();

        let style = Style::new().fg(Color::DarkGray);
        let active_style = Style::new().fg(Color::White);

        for (i, btn) in Remote::iter().enumerate() {
            let mut block = Block::new().style(if state.current_btn == btn {
                active_style
            } else {
                style
            });
            if state.current_btn == btn {
                block = block.borders(Borders::all());
            } else {
                block = block.padding(Padding::top(1));
            }

            Paragraph::new(String::from(btn))
                .block(block)
                .alignment(HorizontalAlignment::Center)
                .render(cells[i], buf);
        }
    }
}
