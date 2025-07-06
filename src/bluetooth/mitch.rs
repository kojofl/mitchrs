use std::cmp::max;

use btleplug::{api::Peripheral as _, platform::Peripheral};
use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Constraint, Direction, Flex, Layout, Rect},
    style::{Color, Style, Stylize as _},
    text::{Span, Text},
    widgets::{
        Block, BorderType, Borders, List, ListItem, ListState, Paragraph, StatefulWidget as _,
        Widget, WidgetRef,
    },
};

#[derive(Clone, Debug)]
pub struct Mitch {
    name: String,
    per: Peripheral,
    connected: bool,
}

impl Mitch {
    pub fn new(name: String, per: Peripheral) -> Self {
        Self {
            name,
            per,
            connected: false,
        }
    }

    pub async fn connect(&mut self) -> color_eyre::Result<()> {
        if self.connected {
            return Ok(());
        }
        self.per.connect().await?;
        self.connected = true;
        Ok(())
    }
}

impl WidgetRef for Mitch {
    fn render_ref(&self, area: Rect, buf: &mut Buffer) {
        let t = format!("{:#?}", self);
        let (h, l) = t
            .lines()
            .fold((2, 0), |acc, l| (acc.0 + 1, max(acc.1, l.len())));
        let a = center(
            area,
            Constraint::Length(l as u16),
            Constraint::Length(h as u16),
        );
        let block = Block::default().borders(Borders::ALL);

        let paragraph = Paragraph::new(t)
            .style(Style::default().fg(Color::White))
            .centered()
            .block(block);

        paragraph.render(a, buf);
    }
}

#[derive(Clone, Debug)]
pub struct MitchList {
    inner: Vec<Mitch>,
    pub active: usize,
}

impl MitchList {
    pub fn new() -> Self {
        Self {
            inner: Vec::new(),
            active: 0,
        }
    }

    pub fn insert(&mut self, mitch: Mitch) {
        self.inner.push(mitch);
    }

    pub fn get_active(&self) -> &Mitch {
        &self.inner[self.active]
    }

    pub fn get_active_mut(&mut self) -> &mut Mitch {
        &mut self.inner[self.active]
    }

    pub fn len(&self) -> usize {
        return self.inner.len();
    }
}

impl WidgetRef for MitchList {
    fn render_ref(&self, area: Rect, buf: &mut Buffer) {
        let len = self.inner.iter().map(|m| m.name.len()).max().unwrap_or(0);
        // Define a layout for the list items. Each item gets 3 rows.
        let item_height = 3;
        let constraints: Vec<Constraint> = self
            .inner
            .iter()
            .map(|_| Constraint::Length(item_height as u16))
            .collect();
        let a = center(
            area,
            Constraint::Length((len + 10) as u16),
            Constraint::Length((item_height * self.len()) as u16),
        );

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints(constraints)
            .split(a);

        // Iterate over items and their corresponding chunks
        for (i, mitch) in self.inner.iter().enumerate() {
            // Determine the style of the block's border
            let border_style = if i == self.active {
                Style::default().fg(Color::Cyan) // Highlighted border
            } else {
                Style::default().fg(Color::DarkGray) // Normal border
            };

            let block = Block::default().borders(Borders::ALL).style(border_style);

            let paragraph = Paragraph::new(mitch.name.as_str())
                .style(Style::default().fg(Color::White))
                .centered()
                .block(block); // Center the text inside the block

            // We render the paragraph and tell it to be contained within the block.
            // The block is rendered into the chunk area we calculated.
            paragraph.render(chunks[i], buf);
        }
    }
}

fn center(area: Rect, horizontal: Constraint, vertical: Constraint) -> Rect {
    let [area] = Layout::horizontal([horizontal])
        .flex(Flex::SpaceAround)
        .areas(area);
    let [area] = Layout::vertical([vertical]).flex(Flex::Center).areas(area);
    area
}
