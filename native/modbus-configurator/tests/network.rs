//! Network composition preserves addressing and encoding across repeated sensors.
use modbus_configurator::{
    catalog::{bundled, table_rows},
    network::{Device, compose, recognize},
};

#[test]
fn thirty_two_partial_devices_preserve_nonsequential_slave_mapping() {
    let profiles = bundled().unwrap();
    let devices: Vec<_> = (1..=32)
        .rev()
        .map(|slave| {
            let mut device = Device::new(0, slave, &profiles);
            device.points = [1].into();
            device
        })
        .collect();
    let table = compose(&devices, &profiles).unwrap();
    assert_eq!(recognize(&table, &profiles), Some(devices.clone()));
    for (row, device) in table_rows(&table).unwrap().values().zip(&devices) {
        assert_eq!(
            row.split('\t').nth(1),
            Some(device.slave.to_string().as_str())
        );
    }
    let mut excess = devices;
    excess.push(Device::new(0, 33, &profiles));
    assert!(compose(&excess, &profiles).is_err());
}

#[test]
fn four_identical_sensors_round_trip_with_unique_points_and_slaves() {
    let profiles = bundled().unwrap();
    let index = profiles
        .iter()
        .position(|p| p.info.id.contains("dpt146"))
        .unwrap();
    let devices: Vec<_> = (1..=4)
        .map(|slave| Device::new(index, slave, &profiles))
        .collect();
    let table = compose(&devices, &profiles).unwrap();
    let rows = table_rows(&table).unwrap();
    assert_eq!(rows.len(), 32);
    assert_eq!(
        rows.keys().copied().collect::<Vec<_>>(),
        (1..=32).collect::<Vec<_>>()
    );
    for (index, row) in rows.values().enumerate() {
        let original = profiles[devices[index / 8].profile]
            .rows
            .values()
            .nth(index % 8)
            .unwrap();
        assert_eq!(
            row.split('\t').nth(1).unwrap().parse::<usize>().unwrap(),
            index / 8 + 1
        );
        assert!(row.split('\t').skip(2).eq(original.split('\t').skip(2)));
    }
    assert_eq!(recognize(&table, &profiles), Some(devices));
}

#[test]
fn subsets_keep_encoding_and_reject_invalid_networks() {
    let profiles = bundled().unwrap();
    let mut devices: Vec<_> = (0..4)
        .map(|index| Device::new(index, index as u8 + 1, &profiles))
        .collect();
    for device in &mut devices {
        device.points = device.points.iter().take(2).copied().collect();
    }
    let table = compose(&devices, &profiles).unwrap();
    assert_eq!(table_rows(&table).unwrap().len(), 8);
    assert_eq!(recognize(&table, &profiles), Some(devices.clone()));
    devices[1].slave = devices[0].slave;
    assert!(compose(&devices, &profiles).is_err());
    devices[1].slave = 248;
    assert!(compose(&devices, &profiles).is_err());
    devices[1].slave = 0;
    assert!(compose(&devices, &profiles).is_err());
    devices[1].slave = 2;
    devices[1].points.clear();
    assert!(compose(&devices, &profiles).is_err());
    devices[1].points.insert(255);
    assert!(compose(&devices, &profiles).is_err());
    assert!(compose(&[], &profiles).is_err());
    devices.push(devices[0].clone());
    assert!(compose(&devices, &profiles).is_err());
}

#[test]
fn capacity_and_custom_imports_fail_without_truncation() {
    let profiles = bundled().unwrap();
    let index = profiles.iter().position(|p| p.rows.len() > 8).unwrap();
    let devices: Vec<_> = (1..=4)
        .map(|slave| Device::new(index, slave, &profiles))
        .collect();
    assert!(compose(&devices, &profiles).unwrap_err().contains("32"));
    let table = compose(&devices[..1], &profiles).unwrap();
    let mut lines: Vec<String> = table.lines().map(str::to_owned).collect();
    let mut fields: Vec<String> = lines[1].split('\t').map(str::to_owned).collect();
    fields[6] = "2".into();
    lines[1] = fields.join("\t");
    assert!(recognize(&lines.join("\n"), &profiles).is_none());
    assert!(recognize("invalid", &profiles).is_none());
}

#[test]
fn editable_custom_rows_round_trip_alongside_known_and_repeated_models() {
    use modbus_configurator::network::editable;
    let profiles = bundled().unwrap();
    let devices = [Device::new(0, 3, &profiles), Device::new(0, 1, &profiles)];
    let original = compose(&devices, &profiles).unwrap();
    let custom = original.replacen("\t4\tF32", "\t1234\tF32", 1);
    let mut entries = editable(&custom, &profiles).unwrap();
    assert_eq!(entries.len(), 2);
    assert_eq!(entries[0].slave, 3);
    assert!(!entries[0].custom_rows.is_empty());
    assert!(entries[1].custom_rows.is_empty());
    assert_eq!(compose(&entries, &profiles).unwrap(), custom);
    entries[0].slave = 9;
    entries[0].points.remove(&2);
    let edited = compose(&entries, &profiles).unwrap();
    assert!(edited.contains("1\t9\tHold\t1234\tF32"));
    assert_eq!(table_rows(&edited).unwrap().len(), 15);
    assert!(editable("invalid", &profiles).is_err());
}
