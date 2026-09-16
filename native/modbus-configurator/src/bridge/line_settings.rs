//! Downstream RS-485 settings, parsed only from a complete Modbus Bridge menu.
use super::*;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LineSettings {
    pub baud: u32,
    pub data_bits: u32,
    pub parity: String,
    pub stop_bits: String,
    pub retries: u32,
    pub timeout_ms: u32,
    pub delay_ms: u32,
}

impl LineSettings {
    /// Reject missing, duplicate or malformed fields rather than supplying defaults.
    pub fn parse(text: &str) -> Option<Self> {
        let field = |label: &str| -> Option<String> {
            let values: Vec<_> = text
                .lines()
                .filter_map(|line| line.trim().strip_prefix(label).map(str::trim))
                .collect();
            (values.len() == 1).then(|| values[0].to_owned())
        };
        let number = |label: &str| field(label)?.split_whitespace().next()?.parse().ok();
        Some(Self {
            baud: number("B  - Baud Rate")?,
            data_bits: number("D  - Data Bits")?,
            parity: field("P  - Parity")?,
            stop_bits: field("S  - Stop Bits")?,
            retries: number("R  - Retries")?,
            timeout_ms: number("T  - Timeout")?,
            delay_ms: number("I  - Inter Message Delay")?,
        })
    }

    pub fn summary(&self) -> String {
        format!(
            "{} baud · {} data bits · {} parity · {} stop bits",
            self.baud, self.data_bits, self.parity, self.stop_bits
        )
    }

    fn fields(&self) -> [(char, String); 7] {
        [
            ('B', self.baud.to_string()),
            ('D', self.data_bits.to_string()),
            ('P', self.parity.clone()),
            ('S', self.stop_bits.to_string()),
            ('R', self.retries.to_string()),
            ('T', self.timeout_ms.to_string()),
            ('I', self.delay_ms.to_string()),
        ]
    }

    pub fn valid(&self) -> bool {
        [2400, 4800, 9600, 14400, 19200, 38400, 56000, 57600].contains(&self.baud)
            && [7, 8].contains(&self.data_bits)
            && ["None", "Odd", "Even"].contains(&self.parity.as_str())
            && ["1", "1.5", "2"].contains(&self.stop_bits.as_str())
            && self.retries <= 10
            && (10..=20000).contains(&self.timeout_ms)
            && (5..=10000).contains(&self.delay_ms)
    }
}

/// Resolve only options actually offered by the firmware, not guessed menu indexes.
fn entry_command(text: &str, key: char, value: &str) -> Option<String> {
    let label = match key {
        'B' => "Baud Rate",
        'D' => "Data Bits",
        'P' => "Parity",
        'S' => "Stop Bits",
        'R' => "Retries",
        'T' => "Timeout",
        'I' => "Inter Message Delay",
        _ => return None,
    };
    if !text.contains(&format!("Current Setting: {label} =")) {
        return None;
    }
    if matches!(key, 'B' | 'D' | 'P' | 'S') {
        let choices: Vec<_> = text
            .lines()
            .filter_map(|line| {
                let (index, label) = line.trim().split_once(" - ")?;
                index.parse::<u8>().ok()?;
                (label.split_whitespace().next()? == value).then(|| index.to_owned())
            })
            .collect();
        return (choices.len() == 1).then(|| choices[0].clone());
    }
    let (min, max) = match key {
        'R' => (0, 10),
        'T' => (10, 20000),
        'I' => (5, 10000),
        _ => return None,
    };
    let number = value.parse::<u32>().ok()?;
    ((min..=max).contains(&number)
        && text.contains(&format!("Enter a number between {min} and {max}")))
    .then(|| value.into())
}

impl BridgeSession {
    /// Expose only settings observed on the current menu, never USB console settings.
    pub fn line_settings(&self) -> Option<LineSettings> {
        (self.parser.state() == PromptState::ModbusMenu)
            .then(|| LineSettings::parse(self.parser.text()))
            .flatten()
    }

    /// Recheck the reviewed snapshot, back it up, then verify each acknowledged change.
    pub fn configure_line(
        &mut self,
        expected: &Identity,
        target: &LineSettings,
        reviewed: &LineSettings,
        cancel: &AtomicBool,
        mut progress: impl FnMut(&str),
        backup: impl FnOnce(&LineSettings) -> io::Result<()>,
    ) -> Result<BridgeResult> {
        if !target.valid() {
            return Err(BridgeError::new(
                ErrorCode::InvalidRequest,
                "Unsupported Modbus Bridge line settings.",
            ));
        }
        let result = self.run_with_export(expected, false, cancel, &mut progress, |_| {})?;
        let current = self.line_settings().ok_or_else(|| {
            BridgeError::new(
                ErrorCode::InvalidResponse,
                "Complete Modbus Bridge line settings were not received.",
            )
        })?;
        if &current != reviewed {
            return Err(BridgeError::new(
                ErrorCode::UnsafeState,
                "Modbus Bridge line settings changed since review. Refresh and review again.",
            ));
        }
        if current == *target {
            return Ok(result);
        }
        backup(&current)?;
        let deadline = Instant::now() + self.timing.operation;
        let change = (|| {
            let mut expected_fields = current.fields();
            for (index, ((key, value), (_, old))) in target
                .fields()
                .into_iter()
                .zip(current.fields())
                .enumerate()
            {
                if value == old {
                    continue;
                }
                progress("Applying Modbus Bridge RS-485 line settings");
                self.require(PromptState::ModbusMenu)?;
                self.send(&key.to_string(), cancel, deadline, self.timing.response)?;
                self.require(PromptState::LineSettingInput)?;
                let command = entry_command(self.parser.text(), key, &value).ok_or_else(|| {
                    BridgeError::new(
                        ErrorCode::InvalidResponse,
                        "Modbus Bridge did not offer the selected line setting.",
                    )
                })?;
                self.send(&command, cancel, deadline, self.timing.response)?;
                if self.parser.state() == PromptState::Continue {
                    self.send("", cancel, deadline, self.timing.response)?;
                }
                self.require(PromptState::ModbusMenu)?;
                let actual = self.line_settings().ok_or_else(|| {
                    BridgeError::new(
                        ErrorCode::InvalidResponse,
                        "Line-setting readback was incomplete.",
                    )
                })?;
                expected_fields[index].1 = value;
                if actual.fields() != expected_fields {
                    return Err(BridgeError::new(
                        ErrorCode::InvalidResponse,
                        "Modbus Bridge line-setting verification failed.",
                    ));
                }
            }
            Ok(())
        })();
        self.cached_table = None;
        if let Err(error) = change {
            return Err(BridgeError::new(
                ErrorCode::ProgrammingUncertain,
                &format!(
                    "Line settings may be partially applied. Polling is paused; refresh the Modbus Bridge settings before retrying. {}",
                    error.message
                ),
            ));
        }
        progress("Modbus Bridge line settings applied and verified");
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn menu_indexes_and_numeric_limits_are_checked() {
        let baud = "Current Setting: Baud Rate = 19200\n5 - 19200 <==\n6 - 38400\nEnter a number between 1 and 8:";
        assert_eq!(entry_command(baud, 'B', "38400"), Some("6".into()));
        assert_eq!(entry_command(baud, 'B', "115200"), None);
        assert_eq!(entry_command(baud, 'P', "38400"), None);
        let stop = "Current Setting: Stop Bits = 2\n1 - 1 bit\n2 - 1.5 bits\n3 - 2 bits <==\nEnter a number between 1 and 3:";
        assert_eq!(entry_command(stop, 'S', "2"), Some("3".into()));
        assert_eq!(entry_command(stop, 'S', "1.5"), Some("2".into()));
        let delay =
            "Current Setting: Inter Message Delay = 150 ms\nEnter a number between 5 and 10000 ms:";
        assert_eq!(entry_command(delay, 'I', "5"), Some("5".into()));
        assert_eq!(entry_command(delay, 'I', "0"), None);
    }
    #[test]
    fn complete_menu_required_and_units_preserved() {
        let menu = "B  - Baud Rate 19200\nD  - Data Bits 8\nP  - Parity None\nS  - Stop Bits 2\nR  - Retries 1\nT  - Timeout 500 ms\nI  - Inter Message Delay 150 ms";
        let settings = LineSettings::parse(menu).unwrap();
        assert_eq!(
            (settings.baud, settings.timeout_ms, settings.delay_ms),
            (19200, 500, 150)
        );
        assert!(LineSettings::parse(&menu.replace("D  - Data Bits 8\n", "")).is_none());
        assert!(LineSettings::parse(&format!("{menu}\nB  - Baud Rate 9600")).is_none());
    }
}
