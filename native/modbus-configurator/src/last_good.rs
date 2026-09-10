use crate::{adapter::AdapterResult, bridge::BridgeResult};

pub fn timestamp() -> String {
    #[cfg(windows)]
    {
        let mut time = windows_sys::Win32::Foundation::SYSTEMTIME {
            wYear: 0,
            wMonth: 0,
            wDayOfWeek: 0,
            wDay: 0,
            wHour: 0,
            wMinute: 0,
            wSecond: 0,
            wMilliseconds: 0,
        };
        // GetLocalTime initializes this caller-owned SYSTEMTIME and cannot fail.
        unsafe {
            windows_sys::Win32::System::SystemInformation::GetLocalTime(&mut time);
        }
        format!(
            "{:04}-{:02}-{:02} {:02}:{:02}:{:02} (local)",
            time.wYear, time.wMonth, time.wDay, time.wHour, time.wMinute, time.wSecond
        )
    }
    #[cfg(not(windows))]
    {
        "Timestamp unavailable on this platform".into()
    }
}

/// Keep failed points only when their complete native definition is unchanged.
pub fn bridge(previous: Option<&BridgeResult>, incoming: &mut BridgeResult) {
    let Some(old) = previous else {
        return;
    };
    if old.identity != incoming.identity {
        return;
    }
    for reading in &old.readings {
        let row = |table: &str| {
            table
                .lines()
                .find(|row| {
                    row.split('\t').next().and_then(|s| s.parse::<u8>().ok()) == Some(reading.item)
                })
                .map(str::to_owned)
        };
        if row(&old.native_tsv).is_some()
            && row(&old.native_tsv) == row(&incoming.native_tsv)
            && !incoming.readings.iter().any(|r| r.item == reading.item)
        {
            incoming.readings.push(reading.clone());
        }
    }
}
pub fn adapter(previous: Option<&AdapterResult>, incoming: &mut AdapterResult) {
    if let Some(old) = previous
        && crate::adapter::same_device(&old.port, &incoming.port)
        && old.key == incoming.key
        && old.settings.slave == incoming.settings.slave
    {
        for (address, value) in &old.values {
            incoming.values.entry(*address).or_insert(*value);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{bridge::Reading, contract::Identity};
    #[test]
    fn failed_point_retains_value_but_a_real_zero_replaces_it() {
        let old = BridgeResult {
            identity: Identity {
                model: "ENL-MOD-32".into(),
                firmware: "3.6".into(),
            },
            native_tsv:
                "Item\tID\tReg\tAddr\tData\tWord\tMult\tRead\n1\t1\tHold\t4\tF32\tHL\t1\tInt\n"
                    .into(),
            readings: vec![Reading {
                item: 1,
                value: 23.5,
            }],
            successful_reads: Some(1),
            exceptions: vec![],
        };
        let mut new = old.clone();
        new.readings.clear();
        new.successful_reads = Some(0);
        bridge(Some(&old), &mut new);
        assert_eq!(new.readings[0].value, 23.5);
        assert_eq!(new.successful_reads, Some(0));
        new.readings[0].value = 0.0;
        bridge(Some(&old), &mut new);
        assert_eq!(new.readings[0].value, 0.0);
        new.readings.clear();
        new.native_tsv = new.native_tsv.replace("\t4\t", "\t6\t");
        bridge(Some(&old), &mut new);
        assert!(new.readings.is_empty());
    }
    #[test]
    #[cfg(windows)]
    fn local_timestamp_is_readable() {
        let text = timestamp();
        assert_eq!(text.len(), 27);
        assert_eq!(&text[10..11], " ");
        assert!(text.ends_with(" (local)"));
    }
}
