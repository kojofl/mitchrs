use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Rect},
    style::{Color, Style, Stylize},
    text::Span,
    widgets::{Block, BorderType, Paragraph, Widget, WidgetRef as _},
};

use crate::app::{App, AppState};

impl Widget for &App {
    /// Renders the user interface widgets.
    ///
    // This is where you add new widgets.
    // See the following resources:
    // - https://docs.rs/ratatui/latest/ratatui/widgets/index.html
    // - https://github.com/ratatui/ratatui/tree/master/examples
    fn render(self, area: Rect, buf: &mut Buffer) {
        match self.state {
            AppState::Menu => {
                self.render_menu(area, buf);
            }
            AppState::Mitch => {
                self.render_mitch(area, buf);
            }
        }
    }
}

impl App {
    fn render_menu(&self, area: Rect, buf: &mut Buffer) {
        let block = Block::bordered()
            .title("mitchrs")
            .title_alignment(Alignment::Center)
            .border_style(Style::new().white())
            .border_type(BorderType::Rounded);

        let text = "This is a tui template.\n\
                Press `Esc`, `Ctrl-C` or `q` to stop running.\n\
            ";

        let paragraph = Span::styled(text, Style::new().bg(Color::Black));

        let p = Paragraph::new(paragraph).block(block);
        p.render(area, buf);
        self.mitches.render_ref(area, buf);
    }

    fn render_mitch(&self, area: Rect, buf: &mut Buffer) {
        let block = Block::bordered()
            .title("mitchrs")
            .title_alignment(Alignment::Center)
            .border_style(Style::new().white())
            .border_type(BorderType::Rounded);
        let text = "Press `Esc`, `Ctrl-C` or `q` to stop running.\n";

        let paragraph = Span::styled(text, Style::new().bg(Color::Black));
        let p = Paragraph::new(paragraph).block(block);
        p.render(area, buf);

        self.mitches.get_active().render_ref(area, buf);
    }
}
