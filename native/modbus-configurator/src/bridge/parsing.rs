//! Validate exported point tables and decode console measurement reports.
use super::*;

/// Classify malformed console output as an invalid response.
fn invalid(message: &str) -> BridgeError {
    BridgeError::new(ErrorCode::InvalidResponse, message)
}

/// Read the configured point count from the export menu, bounded by the 32-point limit.
pub(super) fn export_count(text: &str) -> Result<usize> {
    for line in text.lines() {
        if line.trim().starts_with("E - Export")
            && let Some(count) = line
                .split_whitespace()
                .last()
                .and_then(|s| s.strip_suffix("/32"))
                .and_then(|s| s.parse::<usize>().ok())
            && count <= 32
        {
            return Ok(count);
        }
    }
    Err(invalid(
        "Modbus Bridge menu did not report a valid point count.",
    ))
}

/// Validate native TSV rows and require the advertised number of distinct points.
pub fn parse_export(text: &str, expected_count: usize) -> Result<BTreeMap<u8, String>> {
    let mut rows = BTreeMap::new();
    for line in text.lines().map(str::trim) {
        if !line.as_bytes().first().is_some_and(u8::is_ascii_digit) {
            continue;
        }
        let fields: Vec<_> = line.split('\t').collect();
        if fields.len() != 8 {
            return Err(invalid("Malformed native export row."));
        }
        let item = fields[0]
            .parse::<u8>()
            .map_err(|_| invalid("Invalid point index."))?;
        let slave = fields[1]
            .parse::<u8>()
            .map_err(|_| invalid("Invalid slave address."))?;
        fields[3]
            .parse::<u16>()
            .map_err(|_| invalid("Invalid transmitted register address."))?;
        let mult = fields[6]
            .parse::<f64>()
            .map_err(|_| invalid("Invalid multiplier."))?;
        if !(1..=32).contains(&item)
            || !(1..=247).contains(&slave)
            || !matches!(fields[2], "Hold" | "Input")
            || !matches!(fields[4], "U16" | "S16" | "U32" | "S32" | "F32")
            || !matches!(fields[5], "HH" | "HL" | "LH" | "LL")
            || !mult.is_finite()
            || fields[7] != "Int"
        {
            return Err(invalid("Unsupported native point-table field."));
        }
        if rows.insert(item, line.into()).is_some() {
            return Err(invalid("Duplicate exported point index."));
        }
    }
    if rows.len() != expected_count || expected_count > 32 {
        return Err(invalid(
            "Export row count does not match the Modbus Bridge menu.",
        ));
    }
    Ok(rows)
}

/// Each configured point must have a final value or an explicit device exception.
/// Missing output is a transport failure, never a measurement or a zero value.
/// Decode point readings and exceptions against the configured table.
pub fn parse_point_report(
    text: &str,
    rows: &BTreeMap<u8, String>,
) -> Result<(Vec<Reading>, Vec<PointException>)> {
    point_report(text, rows, false)
}

/// In-flight output may omit unfinished points; all reported headers still require validation.
pub(super) fn point_report(
    text: &str,
    rows: &BTreeMap<u8, String>,
    partial: bool,
) -> Result<(Vec<Reading>, Vec<PointException>)> {
    if !text.contains("--- [") && !text.contains("--- Reading:") {
        return parse_readings(text, rows).map(|readings| (readings, vec![]));
    }
    let mut outcomes: BTreeMap<u8, std::result::Result<f64, PointException>> = BTreeMap::new();
    let mut current = None;
    for line in text.lines().map(str::trim) {
        if let Some(header) = line.strip_prefix("--- [") {
            if let Some(item) = current
                && !outcomes.contains_key(&item)
            {
                return Err(invalid("Detailed read omitted a point result."));
            }
            let (index, fields) = header
                .split_once(']')
                .ok_or_else(|| invalid("Malformed detailed point header."))?;
            let item = index
                .trim()
                .parse::<u8>()
                .map_err(|_| invalid("Invalid detailed point index."))?;
            let row = rows
                .get(&item)
                .ok_or_else(|| invalid("Unexpected detailed point index."))?;
            let columns: Vec<_> = row.split('\t').collect();
            let mut fields: Vec<_> = fields.split_whitespace().collect();
            let retry = fields.last().is_some_and(|f| f.starts_with("Retry:"));
            if retry {
                fields.pop();
            }
            let expected = [
                format!("ID:{}", columns[1]),
                format!("Reg:{}", columns[2]),
                format!("Addr:{}", columns[3]),
                format!("Data:{}", columns[4]),
                columns[5].into(),
            ];
            if fields != expected.iter().map(String::as_str).collect::<Vec<_>>() {
                return Err(invalid(
                    "Detailed point settings differ from the exported table.",
                ));
            }
            if retry {
                if !matches!(outcomes.get(&item), Some(Err(_))) {
                    return Err(invalid("Unexpected detailed retry."));
                }
                outcomes.remove(&item);
            } else if outcomes.contains_key(&item) {
                return Err(invalid("Duplicate detailed point."));
            }
            current = Some(item);
        } else if let Some(value) = line.strip_prefix("--- Reading:") {
            let item = current.ok_or_else(|| {
                invalid("Incomplete detailed output: reading without a point header.")
            })?;
            let value = value
                .trim()
                .parse::<f64>()
                .map_err(|_| invalid("Invalid detailed reading."))?;
            if !value.is_finite() || outcomes.insert(item, Ok(value)).is_some() {
                return Err(invalid("Duplicate or non-finite detailed reading."));
            }
        } else if let Some(exception) = line.strip_prefix("--- Exception:") {
            let item = current.ok_or_else(|| {
                invalid("Incomplete detailed output: exception without a point header.")
            })?;
            let (code, message) = exception
                .trim()
                .strip_prefix('[')
                .and_then(|s| s.split_once(']'))
                .ok_or_else(|| invalid("Malformed Modbus exception."))?;
            let code = code
                .parse::<u8>()
                .map_err(|_| invalid("Invalid Modbus exception code."))?;
            if code == 0
                || outcomes
                    .insert(
                        item,
                        Err(PointException {
                            item,
                            code,
                            message: message.trim().trim_matches('\'').into(),
                        }),
                    )
                    .is_some()
            {
                return Err(invalid("Duplicate or invalid Modbus exception."));
            }
        }
    }
    if !partial && outcomes.len() != rows.len() {
        return Err(invalid(
            "Detailed read did not return every configured point.",
        ));
    }
    let mut readings = vec![];
    let mut exceptions = vec![];
    for (item, outcome) in outcomes {
        match outcome {
            Ok(value) => readings.push(Reading { item, value }),
            Err(exception) => exceptions.push(exception),
        }
    }
    Ok((readings, exceptions))
}

/// Parse legacy read output and reject unknown points or invalid values.
fn parse_readings(text: &str, rows: &BTreeMap<u8, String>) -> Result<Vec<Reading>> {
    if text.to_ascii_lowercase().contains("--- exception:") {
        return Err(invalid(
            "The Modbus Bridge reported Modbus exceptions. Check the configured register addresses against the connected instrument.",
        ));
    }
    let mut readings = BTreeMap::new();
    for line in text.lines() {
        let line = line.trim();
        let line = line
            .strip_prefix("Item ")
            .or_else(|| line.strip_prefix("item "))
            .unwrap_or(line);
        if !line.as_bytes().first().is_some_and(u8::is_ascii_digit) {
            continue;
        }
        let tokens: Vec<_> = line
            .split([' ', '\t', ':', '='])
            .filter(|s| !s.is_empty())
            .collect();
        if tokens.len() != 2 {
            return Err(invalid("Malformed Modbus Bridge reading."));
        }
        let item = tokens[0]
            .parse::<u8>()
            .map_err(|_| invalid("Invalid reading index."))?;
        let value = tokens[1]
            .parse::<f64>()
            .map_err(|_| invalid("Invalid reading value."))?;
        if !value.is_finite() || !rows.contains_key(&item) || readings.insert(item, value).is_some()
        {
            return Err(invalid(
                "Duplicate, unexpected, or non-finite Modbus Bridge reading.",
            ));
        }
    }
    if readings.len() != rows.len() {
        return Err(invalid("Read All did not return every configured point."));
    }
    Ok(readings
        .into_iter()
        .map(|(item, value)| Reading { item, value })
        .collect())
}

/// Check that the console summary accounts for the expected readings and exceptions.
pub(super) fn verify_summary(text: &str, expected: usize, exceptions: usize) -> Result<()> {
    let mut summaries = vec![];
    for line in text.lines() {
        if let Some((before, _)) = line.split_once("(OK/Exceptions)") {
            summaries.push(before.split_whitespace().last().unwrap_or(""));
        }
    }
    if summaries.last().copied() != Some(format!("{expected}/{exceptions}").as_str()) {
        return Err(invalid(
            "Read All summary does not match the received point results.",
        ));
    }
    Ok(())
}
