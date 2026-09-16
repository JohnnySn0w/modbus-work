//! Read-only evidence checks; never identify hardware or repair a table by inference.
use modbus_configurator::{
    bridge::{BridgeResult, LineSettings},
    catalog::{Profile, Register, table_rows},
};
use std::collections::BTreeSet;
#[cfg(test)]
#[path = "tests/diagnostic_checks.rs"]
mod tests;

#[derive(Debug, PartialEq, Eq)]
pub(super) struct Finding {
    pub warning: bool,
    pub subject: String,
    pub detail: String,
}

fn add(
    out: &mut Vec<Finding>,
    warning: bool,
    subject: impl Into<String>,
    detail: impl Into<String>,
) {
    out.push(Finding {
        warning,
        subject: subject.into(),
        detail: detail.into(),
    });
}

/// Match all encoding fields and an explicit address difference, ignoring point/slave IDs.
fn matches(actual: &str, expected: &str, difference: i32) -> bool {
    let a: Vec<_> = actual.split('\t').collect();
    let b: Vec<_> = expected.split('\t').collect();
    a.len() == 8
        && b.len() == 8
        && a[2] == b[2]
        && a[4..] == b[4..]
        && a[3]
            .parse::<i32>()
            .ok()
            .zip(b[3].parse::<i32>().ok())
            .is_some_and(|(a, b)| a - b == difference)
}

/// Broad physical checks, not sensor accuracy or operating-range certification.
fn value_check(value: f64, register: Option<&Register>, float: bool) -> Option<String> {
    if !value.is_finite() {
        return Some(
            "Non-finite value. Check register address, data type, word order, and sensor status."
                .into(),
        );
    }
    if float && value != 0.0 && value.abs() < f64::from(f32::MIN_POSITIVE) {
        return Some(format!(
            "Value {value:e} is a subnormal 32-bit float. An address or word-order mismatch is possible; a very small value alone is not proof."
        ));
    }
    let r = register?;
    let invalid = match r.units.as_str() {
        "%RH" => !(0.0..=100.0).contains(&value),
        "deg C" => value < -273.15,
        "deg F" => value < -459.67,
        "bara" | "ppmv" | "g/m3" | "g/kg" => value < 0.0,
        _ => false,
    };
    if invalid {
        return Some(format!(
            "{} = {value} {} is outside broad physical bounds. Check encoding and sensor status; this does not establish an indexing error.",
            r.name, r.units
        ));
    }
    if (r.name.to_ascii_lowercase().contains("status") || r.name == "Error code")
        && ((r.decode.contains("1 = no errors") || r.decode.contains("1 = online data available"))
            && value != 1.0
            || r.defaults.starts_with("0 =") && value != 0.0)
    {
        return Some(format!(
            "{} reports {value}. Register definition: {} {}",
            r.name, r.decode, r.defaults
        ));
    }
    None
}

/// Compare each slave independently, including partial sets and repeated device models.
pub(super) fn bridge(
    result: &BridgeResult,
    profiles: &[Profile],
    settings: Option<&LineSettings>,
) -> Vec<Finding> {
    let mut out = Vec::new();
    let rows = match table_rows(&result.native_tsv) {
        Ok(rows) => rows,
        Err(error) => {
            add(&mut out, true, "Point table", error);
            return out;
        }
    };
    let slaves: BTreeSet<_> = rows.values().filter_map(|r| r.split('\t').nth(1)).collect();
    for slave in slaves {
        let actual: Vec<_> = rows
            .iter()
            .filter(|(_, r)| r.split('\t').nth(1) == Some(slave))
            .collect();
        let candidates = |difference| {
            profiles
                .iter()
                .filter(|profile| {
                    actual
                        .iter()
                        .all(|(_, row)| profile.rows.values().any(|r| matches(row, r, difference)))
                })
                .collect::<Vec<_>>()
        };
        let exact = candidates(0);
        let profile = if exact.len() == 1 {
            Some(exact[0])
        } else {
            None
        };
        let subject = format!("Slave {slave} · register layout");
        if let Some(profile) = profile {
            add(
                &mut out,
                false,
                subject,
                format!(
                    "{} entries match the reviewed {} encoding. This identifies a configuration, not the attached device. HMD65 E5 bridge tables use their reviewed one-based convention.",
                    actual.len(),
                    profile.info.model
                ),
            );
        } else if exact.is_empty() {
            let mut shifts = Vec::new();
            for difference in [-1, 1] {
                for p in candidates(difference) {
                    shifts.push(format!(
                        "{} ({difference:+} relative to reviewed addresses)",
                        p.info.model
                    ));
                }
            }
            add(
                &mut out,
                true,
                subject,
                if shifts.is_empty() {
                    "Custom or unsupported register layout. Range and status checks require an unambiguous reviewed encoding; verify address, function, type, word order, and multiplier.".into()
                } else {
                    format!(
                        "Possible zero/one indexing mismatch: all {} entries match {} after an address shift. This is a table comparison, not proof from sensor responses. Verify the device documentation before changing addresses.",
                        actual.len(),
                        shifts.join("; ")
                    )
                },
            );
        } else {
            add(
                &mut out,
                false,
                subject,
                "Partial register set matches multiple profiles. Device-specific checks are unavailable.",
            );
        }
        for (item, row) in actual {
            let register = profile.and_then(|p| {
                p.rows
                    .iter()
                    .find(|(_, r)| matches(row, r, 0))
                    .and_then(|(i, _)| p.point_register(*i))
            });
            let subject = format!("Slave {slave} · point {item}");
            if let Some(error) = result.exceptions.iter().find(|e| e.item == *item) {
                add(
                    &mut out,
                    true,
                    subject,
                    format!(
                        "Exception {}: {}. {}",
                        error.code,
                        error.message,
                        if error.code == 2 {
                            "An illegal address can mean an unsupported register, wrong function, incomplete register pair, or indexing mismatch. It does not identify which cause applies."
                        } else {
                            "Check the device status and the communication settings. A timeout alone cannot distinguish baud rate, parity, slave address, wiring, or power."
                        }
                    ),
                );
            } else if let Some(reading) = result.readings.iter().find(|r| r.item == *item) {
                if let Some(detail) = value_check(
                    reading.value,
                    register,
                    row.split('\t').nth(4) == Some("F32"),
                ) {
                    add(&mut out, true, subject, detail);
                }
            } else if result.successful_reads.is_some() {
                add(
                    &mut out,
                    true,
                    subject,
                    "No value or exception returned in this read. A retained display value is not evidence of a successful current read.",
                );
            }
        }
    }
    if result.successful_reads.is_none() {
        add(
            &mut out,
            false,
            "Measurement checks",
            "Configuration-only result. Read now to assess sensor responses.",
        );
    }
    if let Some(s) = settings {
        add(
            &mut out,
            !s.valid() || s.data_bits != 8,
            "E5 bridge serial settings",
            format!(
                "{}; timeout {} milliseconds; retries {}; delay {} milliseconds. {}",
                s.summary(),
                s.timeout_ms,
                s.retries,
                s.delay_ms,
                if s.data_bits != 8 {
                    "Modbus RTU requires eight data bits."
                } else {
                    "The instrument settings must match. USB console communication does not verify downstream baud rate or parity."
                }
            ),
        );
        // Eight request bytes plus thirteen response bytes for a four-register value.
        let bits = 1.0
            + f64::from(s.data_bits)
            + if s.parity == "None" { 0.0 } else { 1.0 }
            + s.stop_bits.parse::<f64>().unwrap_or(2.0);
        let wire_ms = 21.0 * bits * 1000.0 / f64::from(s.baud.max(1));
        if f64::from(s.timeout_ms) < wire_ms {
            add(
                &mut out,
                true,
                "Response timeout",
                format!(
                    "{} milliseconds is below the approximately {wire_ms:.1} milliseconds needed for an eight-byte request and thirteen-byte response at this line speed. Device processing and turnaround add time; this is an estimate, not a measured timeout requirement.",
                    s.timeout_ms
                ),
            );
        }
    } else {
        add(
            &mut out,
            false,
            "Serial settings unavailable",
            "Read the E5 bridge line settings before assessing baud rate, parity, stop bits, or timeout. No settings have been inferred from measurement values.",
        );
    }
    out
}

/// Assess direct-adapter data using its own address convention and reported settings.
pub(super) fn adapter(
    result: &modbus_configurator::adapter::AdapterResult,
    profiles: &[Profile],
) -> Vec<Finding> {
    let mut out = Vec::new();
    add(
        &mut out,
        false,
        "USB adapter settings",
        format!(
            "{} · 8 data bits · {} parity · {} stop bits. Responses do not establish sensor accuracy.",
            result.settings.label(),
            if result.settings.even { "even" } else { "none" },
            if result.settings.two_stops { 2 } else { 1 }
        ),
    );
    let profile = profiles.iter().find(|p| p.info.id == result.key);
    for (address, value) in &result.values {
        let register = profile.and_then(|p| {
            p.registers
                .iter()
                .find(|r| r.range().is_ok_and(|(first, _)| first == *address))
        });
        if let Some(detail) = value_check(
            *value,
            register,
            register.is_some_and(|r| r.interpretation.contains("float32")),
        ) {
            add(
                &mut out,
                true,
                format!(
                    "Slave {} · transmitted address {address}",
                    result.settings.slave
                ),
                detail,
            );
        }
    }
    for (address, error) in &result.errors {
        add(
            &mut out,
            true,
            format!("Transmitted address {address}"),
            format!(
                "{error}. Verify slave address, 19200 baud, parity, stop bits, wiring, and power. This error alone does not determine the cause."
            ),
        );
    }
    out
}
