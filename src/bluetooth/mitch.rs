use std::cmp::max;

use btleplug::{
    api::{Peripheral as _, WriteType},
    platform::Peripheral,
};
use color_eyre::eyre::eyre;
use futures::{StreamExt, executor::block_on};
use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Direction, Flex, Layout, Rect},
    style::{Color, Style},
    widgets::{Block, Borders, Paragraph, Widget, WidgetRef},
};
use std::fmt;
use uuid::{Uuid, uuid};

pub const COMMAND_CHAR: Uuid = uuid!("d5913036-2d8a-41ee-85b9-4e361aa5c8a7");
pub const DATA_CHAR: Uuid = uuid!("09bf2c52-d1d9-c0b7-4145-475964544307");

#[derive(Clone)]
pub struct Mitch {
    name: String,
    per: Peripheral,
    connected: bool,
    state: Option<MitchState>,
}

impl Drop for Mitch {
    fn drop(&mut self) {
        if self.connected {
            let _ = block_on(self.per.disconnect());
        }
    }
}

impl fmt::Debug for Mitch {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        #[derive(Debug)]
        #[allow(dead_code)]
        struct DebugMitch<'a> {
            name: &'a String,
            connected: bool,
            state: Option<MitchState>,
        }
        let dbg = DebugMitch {
            name: &self.name,
            connected: self.connected,
            state: self.state,
        };
        fmt::Debug::fmt(&dbg, f)
    }
}

#[derive(Clone, Copy, Debug)]
#[repr(u8)]
pub enum MitchState {
    SysStartup = 0x01,
    SysIdle = 0x02,
    SysStandby = 0x03,
    SysLog = 0x04,
    SysReadout = 0x05,
    SysTx = 0xF8,
    SysError = 0xFF,
    BootStartup = 0xf0,
    BootIdle = 0xf1,
    BootDownload = 0xf2,
}

impl TryFrom<u8> for MitchState {
    type Error = color_eyre::eyre::Error;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        if (1_u8..=5_u8).contains(&value)
            || value == 0xf8
            || value == 0xff
            || value == 0xf0
            || value == 0xf1
            || value == 0xf2
        {
            return Ok(unsafe { *(&value as *const _ as *const MitchState) });
        }
        Err(eyre!("Unknown state: {value}"))
    }
}

enum Commands {
    GetState,
    StartStream,
    StopStream,
}

impl AsRef<[u8]> for Commands {
    fn as_ref(&self) -> &[u8] {
        match self {
            Commands::GetState => &[130, 0],
            Commands::StartStream => &[0x02, 0x03, 0xF8, 0x04, 0x04],
            Commands::StopStream => &[0x02, 0x01, 0x02],
        }
    }
}

impl Mitch {
    pub async fn new(name: String, per: Peripheral) -> color_eyre::Result<Self> {
        let mut s = per.notifications().await?;
        tokio::spawn(async move {
            while let Some(b) = s.next().await {
                if b.uuid != DATA_CHAR {
                    continue;
                }
                println!("{b:?}")
            }
        });
        Ok(Self {
            name,
            per,
            connected: false,
            state: None,
        })
    }

    pub fn name_with_state(&self) -> String {
        format!("{} - {:?}", self.name, self.state)
    }

    pub(crate) async fn start_recording(&mut self) -> color_eyre::Result<()> {
        let c = self.per.characteristics();
        let data_char = c.iter().find(|c| c.uuid == DATA_CHAR).unwrap();
        self.per.subscribe(data_char).await?;
        let cmd_char = c.iter().find(|c| c.uuid == COMMAND_CHAR).unwrap();
        self.per
            .write(
                cmd_char,
                Commands::StartStream.as_ref(),
                WriteType::WithResponse,
            )
            .await?;
        self.per.read(cmd_char).await?;
        Ok(())
    }

    pub(crate) async fn stop_recording(&mut self) -> color_eyre::Result<()> {
        let characteristics = self.per.characteristics();
        let cmd_char = characteristics
            .iter()
            .find(|c| c.uuid == COMMAND_CHAR)
            .unwrap();
        self.per
            .write(
                cmd_char,
                Commands::StopStream.as_ref(),
                WriteType::WithResponse,
            )
            .await?;
        self.per.read(cmd_char).await?;
        Ok(())
    }

    pub(crate) async fn update_state(&mut self) -> color_eyre::Result<()> {
        if !self.connected {
            self.state = None;
            return Ok(());
        }
        let c = self.per.characteristics();
        let cmd_char = c.iter().find(|c| c.uuid == COMMAND_CHAR).unwrap();
        self.per
            .write(
                cmd_char,
                Commands::GetState.as_ref(),
                WriteType::WithResponse,
            )
            .await?;
        let state = MitchState::try_from(self.per.read(cmd_char).await?[4])?;
        self.state = Some(state);
        Ok(())
    }

    pub(crate) async fn connect(&mut self) -> color_eyre::Result<()> {
        if self.connected {
            return Ok(());
        }
        self.per.connect().await?;
        self.per.discover_services().await?;
        self.connected = true;
        Ok(())
    }

    pub(crate) async fn disconnect(&mut self) -> color_eyre::Result<()> {
        if !self.connected {
            return Ok(());
        }
        self.per.disconnect().await?;
        self.connected = false;
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

impl Default for MitchList {
    fn default() -> Self {
        Self::new()
    }
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

    // Update state for all mitches if the update fails we disconnect the mitch
    // TODO: maybe we should mark it
    pub async fn update(&mut self) -> color_eyre::Result<()> {
        for i in (0..self.inner.len()).rev() {
            if self.inner[i].update_state().await.is_err() {
                // we ignore the error here since it is very likely that the connection has gone
                // away and therefore the function would error and that is fine
                let _ = self.inner[i].disconnect().await;
            }
        }
        Ok(())
    }

    pub fn len(&self) -> usize {
        self.inner.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl WidgetRef for MitchList {
    fn render_ref(&self, area: Rect, buf: &mut Buffer) {
        let info: Vec<String> = self.inner.iter().map(|m| m.name_with_state()).collect();
        let len = info.iter().map(|m| m.len()).max().unwrap_or(0);
        // Define a layout for the list items. Each item gets 3 rows.
        let item_height = 3;
        let constraints: Vec<Constraint> = self
            .inner
            .iter()
            .map(|_| Constraint::Length(item_height as u16))
            .collect();
        let a = center(
            area,
            Constraint::Length((len + 5) as u16),
            Constraint::Length((item_height * self.len()) as u16),
        );

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints(constraints)
            .split(a);

        // Iterate over items and their corresponding chunks
        for (i, _) in self.inner.iter().enumerate() {
            // Determine the style of the block's border
            let border_style = if i == self.active {
                Style::default().fg(Color::Cyan) // Highlighted border
            } else {
                Style::default().fg(Color::DarkGray) // Normal border
            };

            let block = Block::default().borders(Borders::ALL).style(border_style);

            let paragraph = Paragraph::new(info[i].as_str())
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
