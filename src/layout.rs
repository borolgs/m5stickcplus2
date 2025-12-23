use ratatui::layout::{Constraint, Direction, Layout, Rect};

#[derive(Debug, Default, Clone)]
pub struct AppLayout {
    pub header: Rect,
    pub main: Rect,
    pub footer: Rect,
}

impl AppLayout {
    pub fn new(area: Rect) -> Self {
        let header_height = 1;
        let footer_height = 1;
        let [header, main, footer] = Layout::default()
            .direction(Direction::Vertical)
            .constraints(vec![
                Constraint::Max(header_height),
                Constraint::Fill(1),
                Constraint::Max(footer_height),
            ])
            .areas(area);

        Self {
            header,
            main,
            footer,
        }
    }
}
