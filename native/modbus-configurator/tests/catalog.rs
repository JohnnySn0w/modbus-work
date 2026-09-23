use modbus_configurator::{
    bridge::BridgeResult,
    catalog::{self, Profile, ProfileInfo, Register, Validation},
    contract::Identity,
};

fn result(table: String) -> BridgeResult {
    BridgeResult {
        dev_eui: None,
        identity: Identity {
            model: "ENL-MOD-32".into(),
            firmware: "3.6".into(),
        },
        native_tsv: table,
        readings: vec![],
        successful_reads: None,
        exceptions: vec![],
    }
}

#[test]
fn slave_remapping_preserves_registers_and_profile_identity() {
    let profiles = catalog::bundled().unwrap();
    let profile = &profiles[0];
    let remapped =
        modbus_configurator::config_file::remap_slave(&profile.native_tsv, 1, 42).unwrap();
    for (old, new) in profile
        .native_tsv
        .lines()
        .skip(1)
        .zip(remapped.lines().skip(1))
    {
        for (column, (a, b)) in old.split('\t').zip(new.split('\t')).enumerate() {
            if column == 1 {
                assert_eq!(b, "42");
            } else {
                assert_eq!(a, b);
            }
        }
    }
    assert!(profile.contains_points(&result(remapped.clone())));
    assert!(!profile.matches(&result(remapped)));
    assert!(modbus_configurator::config_file::remap_slave(&profile.native_tsv, 1, 0).is_err());
    assert!(modbus_configurator::config_file::remap_slave(&profile.native_tsv, 1, 248).is_err());
}

#[test]
fn complete_profile_subset_keeps_labels_but_does_not_claim_a_full_table_match() {
    let profiles = modbus_configurator::catalog::bundled().unwrap();
    let dpt = &profiles[0];
    let mut actual = result(
        include_str!("../../../docs/evidence/bridge-live-12-point-backup-2026-09-09.tsv").into(),
    );
    assert!(dpt.contains_points(&actual));
    assert!(!dpt.matches(&actual));
    for column in 0..8 {
        let mut lines: Vec<_> = actual.native_tsv.lines().map(str::to_string).collect();
        let mut fields: Vec<_> = lines[1].split('\t').map(str::to_string).collect();
        fields[column] = match column {
            0 => "13",
            1 => "2",
            2 => "Input",
            3 => "5",
            4 => "U32",
            5 => "HH",
            6 => "2",
            _ => "Avg",
        }
        .into();
        lines[1] = fields.join("\t");
        assert!(!dpt.contains_points(&result(lines.join("\n"))));
    }
    actual.identity.firmware = "9.9".into();
    assert!(!dpt.contains_points(&actual));
}

#[test]
fn bundled_artifacts_have_expected_lifecycle_and_complete_point_mapping() {
    let profiles = catalog::bundled().unwrap();
    assert_eq!(profiles.len(), 5);
    assert_eq!(
        profiles.iter().map(|p| p.rows.len()).collect::<Vec<_>>(),
        [8, 8, 12, 8, 13]
    );
    assert_eq!(profiles[0].info.status, Validation::Validated);
    assert!(
        profiles[1..]
            .iter()
            .all(|p| p.info.status == Validation::ToTest)
    );
    for profile in &profiles {
        for item in profile.rows.keys() {
            assert!(profile.point_register(*item).is_some());
        }
    }
    assert_eq!(profiles[0].point_register(1).unwrap().name, "Temperature");
    assert_eq!(profiles[0].point_register(5).unwrap().units, "bara");
    assert_eq!(profiles[1].point_register(1).unwrap().units, "%RH");
    assert_eq!(profiles[2].point_register(12).unwrap().units, "A");
}

#[test]
fn historical_off_by_one_table_never_receives_current_profile_labels() {
    let profiles = catalog::bundled().unwrap();
    let historical = include_str!(
        "../../../artifacts/bridge-config/enl-mod-32-firmware-3.6-historical-precorrection-export.tsv"
    );
    assert!(!profiles[0].matches(&result(historical.into())));
    assert!(!profiles[1].matches(&result(profiles[0].native_tsv.clone())));
    assert!(profiles[0].matches(&result(profiles[0].native_tsv.clone())));
}

#[test]
fn every_native_column_and_firmware_are_part_of_label_gate() {
    let profiles = catalog::bundled().unwrap();
    let profile = &profiles[0];
    let first = profile.rows[&1].clone();
    for (column, value) in [
        (0, "9"),
        (1, "2"),
        (2, "Input"),
        (3, "5"),
        (4, "U32"),
        (5, "HH"),
        (6, "0.1"),
        (7, "Other"),
    ] {
        let mut fields: Vec<_> = first.split('\t').collect();
        fields[column] = value;
        let changed = profile.native_tsv.replacen(&first, &fields.join("\t"), 1);
        assert!(!profile.matches(&result(changed)), "column {column}");
    }
    let mut wrong_firmware = result(profile.native_tsv.clone());
    wrong_firmware.identity.firmware = "3.7".into();
    assert!(!profile.matches(&wrong_firmware));
}

#[test]
fn preview_reports_additions_removals_and_changed_rows() {
    let profiles = catalog::bundled().unwrap();
    let profile = &profiles[0];
    let current = profile
        .native_tsv
        .replace(&format!("{}\r\n", profile.rows[&1]), "")
        .replace(
            &profile.rows[&2],
            &profile.rows[&2].replacen("2\t1", "9\t1", 1),
        )
        .replace(
            &profile.rows[&3],
            &profile.rows[&3].replace("\tHL\t", "\tHH\t"),
        );
    let diff = profile.diff(&current).unwrap();
    assert_eq!(
        diff.iter().map(|d| d.item).collect::<Vec<_>>(),
        [1, 2, 3, 9]
    );
    assert!(diff[0].current.is_none());
    assert!(diff[2].current.is_some() && diff[2].proposed.is_some());
    assert!(diff[3].proposed.is_none());
    assert!(profile.diff(&profile.native_tsv).unwrap().is_empty());
}

fn info() -> ProfileInfo {
    serde_json::from_str(r#"{"id":"test","model":"Test","manufacturer":"Test","status":"to-test","summary":"test","serial":"test","help":["test"],"source":"test"}"#).unwrap()
}
const CSV: &str = "register number,name,units,interpretation,decode,defaults\n5 - 6,Temperature,deg C,IEEE-754 float32,,,\n";
const TABLE: &str = "Item\tID\tReg\tAddr\tData\tWord\tMult\tRead\n1\t1\tHold\t4\tF32\tHL\t1\tInt\n";

#[test]
fn confirmed_models_do_not_inherit_documentation_firmware_versions() {
    let profiles = catalog::bundled().unwrap();
    for id in ["hmd65", "hmd65-nonmetric", "wnd-m1-mb", "ati-f12"] {
        let profile = profiles.iter().find(|p| p.info.id == id).unwrap();
        assert_eq!(profile.info.confirmed_variants.len(), 1);
        let variant = &profile.info.confirmed_variants[0];
        if id == "ati-f12" {
            assert_eq!(variant.model, "ATI F12 transmitter");
            assert_eq!(variant.hardware_revision.as_deref(), Some("1.01"));
            assert_eq!(variant.firmware.as_deref(), Some("1.25"));
        } else {
            assert!(variant.hardware_revision.is_none());
            assert!(variant.firmware.is_none());
        }
        assert!(variant.evidence.contains("16 Sep 2026"));
        assert_eq!(
            profile.info.status,
            Validation::ToTest,
            "Model confirmation alone does not qualify every profile register"
        );
    }
    assert!(
        info().confirmed_variants.is_empty(),
        "Older metadata must remain loadable"
    );
}

#[test]
fn malformed_csv_and_mismatched_register_spans_fail_closed() {
    assert!(Profile::load(info(), CSV, TABLE).is_err()); // extra field
    let csv = CSV.replace("float32,,,", "float32,,");
    assert!(Profile::load(info(), &csv, TABLE).is_ok());
    assert!(Profile::load(info(), &csv, &TABLE.replace("\t4\t", "\t5\t")).is_err());
    assert!(Profile::load(info(), &csv, &TABLE.replace("F32", "U16")).is_err());
    assert!(Profile::load(info(), &csv, &TABLE.replace("F32", "U32")).is_err());
    assert!(Profile::load(info(), &csv, &TABLE.replace("\t1\tInt", "\t0.1\tInt")).is_err());
    assert!(Profile::load(info(), &csv.replace("5 - 6", "0 - 1"), TABLE).is_err());
    assert!(Profile::load(info(), &format!("{csv}5 - 6,Duplicate,,,,\n"), TABLE).is_err());
    assert!(catalog::table_rows(&format!("\u{feff}{TABLE}")).is_err());
    assert!(catalog::table_rows(&TABLE.replace("Item", "Index")).is_err());
}

#[test]
fn csv_quoted_commas_and_embedded_newlines_are_preserved() {
    let csv = "register number,name,units,interpretation,decode,defaults\n5 - 6,\"Temperature, gas\",deg C,\"First line\nSecond line\",,\n";
    let profile = Profile::load(info(), csv, TABLE).unwrap();
    assert_eq!(profile.registers[0].name, "Temperature, gas");
    assert_eq!(
        profile.registers[0].interpretation,
        "First line\nSecond line"
    );
}

#[test]
fn manual_to_pdu_conversion_checks_boundaries() {
    let mut register = Register {
        manual: "1".into(),
        name: "First".into(),
        units: String::new(),
        interpretation: String::new(),
        decode: String::new(),
        defaults: String::new(),
    };
    assert_eq!(register.range().unwrap(), (0, 0));
    register.manual = "65535 - 65536".into();
    assert_eq!(register.range().unwrap(), (65534, 65535));
    for invalid in ["0", "65537", "8 - 7", "1-2", "1 - 2 - 3"] {
        register.manual = invalid.into();
        assert!(register.range().is_err());
        assert_eq!(register.pdu_display(), "Invalid");
    }
}

#[test]
fn incomplete_metadata_empty_tables_and_duplicate_points_fail_closed() {
    let csv = CSV.replace("float32,,,", "float32,,");
    for field in ["id", "model", "help"] {
        let mut metadata = info();
        match field {
            "id" => metadata.id.clear(),
            "model" => metadata.model.clear(),
            _ => metadata.help.clear(),
        }
        assert!(
            Profile::load(metadata, &csv, TABLE)
                .unwrap_err()
                .contains("metadata")
        );
    }
    for (bad_csv, bad_table, reason) in [
        (
            csv.replace("register number", "address"),
            TABLE.to_string(),
            "headers",
        ),
        (csv.replace("Temperature", ""), TABLE.to_string(), "name"),
        (
            csv.clone(),
            format!("{}\n", TABLE.lines().next().unwrap()),
            "at least one",
        ),
        (
            csv,
            format!("{TABLE}2\t1\tHold\t4\tF32\tHL\t1\tInt\n"),
            "Multiple points",
        ),
    ] {
        assert!(
            Profile::load(info(), &bad_csv, &bad_table)
                .unwrap_err()
                .contains(reason)
        );
    }
}

#[test]
fn alternate_bank_and_gas_profile_preserve_wire_addresses_and_word_widths() {
    let profiles = catalog::bundled().unwrap();
    let hmd = &profiles[3];
    for (item, address, unit) in [
        (1, 128, "%RH"),
        (2, 130, "deg F"),
        (3, 132, "deg F"),
        (5, 136, "gr/ft3"),
        (6, 138, "gr/lb"),
    ] {
        let r = hmd.point_register(item).unwrap();
        assert_eq!(r.range().unwrap(), (address, address + 1));
        assert_eq!(r.units, unit);
    }
    assert!(!hmd.rows.values().any(|row| row.contains("\tS32\t")));
    let ati = &profiles[4];
    assert_eq!(ati.point_register(8).unwrap().range().unwrap(), (42, 43));
    assert!(ati.rows[&8].contains("\t42\tF32\tHL\t"));
    let reference = modbus_configurator::reference::Reference::bundled().unwrap();
    let fault = &reference.registers["ati-f12"][2];
    assert_eq!(fault.decode(Some(32)), "gas sensor removed");
    assert_eq!(fault.decode(Some(0)), "No flags set");
    assert_eq!(reference.devices["ati-f12"].help_setup, ["TBD"]);
}

#[test]
fn hmd65_bridge_tables_use_manual_numbers_and_exclude_unwanted_status_registers() {
    for profile in catalog::bundled()
        .unwrap()
        .iter()
        .filter(|p| p.info.id.starts_with("hmd65"))
    {
        assert_eq!(profile.rows.len(), 8);
        for (item, row) in &profile.rows {
            let address: u16 = row.split('\t').nth(3).unwrap().parse().unwrap();
            let register = profile.point_register(*item).unwrap();
            assert_eq!(address, register.range().unwrap().0 + 1);
            assert!(![15, 16, 143, 144, 514, 515, 518, 519].contains(&address));
            assert_ne!(register.name, "Error code");
            assert_ne!(register.name, "Enthalpy");
        }
    }
}
