//! Transport-independent recognition only. This module never sends console input.
pub const MAX_RESPONSE_BYTES: usize = 65536;
use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PromptState {
    #[default]
    Unknown,
    Password,
    MainMenu,
    ModbusMenu,
    ImportExport,
    RadioMenu,
    ImportInput,
    Continue,
    ReadComplete,
    ReadOptions,
}

#[derive(Default)]
enum Escape {
    #[default]
    None,
    Start,
    Csi,
    Osc,
    OscEnd,
}

/// Bounded ASCII console window; ANSI state survives arbitrary read boundaries.
#[derive(Default)]
pub struct ConsoleParser {
    text: String,
    escape: Escape,
}

impl ConsoleParser {
    pub fn text(&self) -> &str {
        &self.text
    }
    pub fn feed(&mut self, bytes: &[u8]) -> PromptState {
        for &byte in bytes {
            match self.escape {
                Escape::Start => {
                    self.escape = match byte {
                        b'[' => Escape::Csi,
                        b']' => Escape::Osc,
                        _ => Escape::None,
                    };
                }
                Escape::Csi => {
                    if (0x40..=0x7e).contains(&byte) {
                        self.escape = Escape::None;
                    }
                }
                Escape::Osc => {
                    if byte == 7 {
                        self.escape = Escape::None;
                    } else if byte == 27 {
                        self.escape = Escape::OscEnd;
                    }
                }
                Escape::OscEnd => {
                    self.escape = if byte == b'\\' {
                        Escape::None
                    } else {
                        Escape::Osc
                    };
                }
                Escape::None => match byte {
                    27 => self.escape = Escape::Start,
                    b'\r' => self.text.push('\n'),
                    b'\n' if self.text.ends_with('\n') => {}
                    b'\n' | b'\t' | 32..=126 => self.text.push(byte as char),
                    _ => {}
                },
            }
        }
        if self.text.len() > MAX_RESPONSE_BYTES {
            self.text.drain(..self.text.len() - MAX_RESPONSE_BYTES);
        }
        self.state()
    }

    pub fn state(&self) -> PromptState {
        let text = self.text.to_ascii_lowercase();
        // Last recognized prompt wins: an old main menu must not mask a newer submenu.
        [
            ("password:", PromptState::Password),
            ("enlink main menu:", PromptState::MainMenu),
            ("modbus configuration menu:", PromptState::ModbusMenu),
            ("modbus import/export menu:", PromptState::ImportExport),
            ("radio configuration menu:", PromptState::RadioMenu),
            (
                "finish the import with an empty line",
                PromptState::ImportInput,
            ),
            ("press a key to continue", PromptState::Continue),
            ("modbus read completed", PromptState::ReadComplete),
            ("read all data points options:", PromptState::ReadOptions),
        ]
        .into_iter()
        .filter_map(|(pattern, state)| text.rfind(pattern).map(|index| (index, state)))
        .max_by_key(|(index, _)| *index)
        .map_or(PromptState::Unknown, |(_, state)| state)
    }

    /// Call when a session disconnects; never carry a prompt across reconnects.
    pub fn reset(&mut self) {
        *self = Self::default();
    }
}
