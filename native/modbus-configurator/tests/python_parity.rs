use modbus_configurator::reference::Reference;
use serde::Deserialize;

#[derive(Deserialize)]
struct Case {
    device: String,
    index: usize,
    value: Option<i64>,
    expected: String,
}

#[test]
fn every_python_register_decode_case_matches_rust() {
    let reference = Reference::bundled().unwrap();
    let cases: Vec<Case> =
        serde_json::from_str(include_str!("../assets/python-register-cases.json")).unwrap();
    assert_eq!(cases.len(), 68 * 8);
    for case in cases {
        let register = &reference.registers[&case.device][case.index];
        assert_eq!(
            register.decode(case.value),
            case.expected,
            "{} {} {:?}",
            case.device,
            register.name,
            case.value
        );
    }
}

#[test]
fn python_device_actions_have_their_reference_data() {
    let reference = Reference::bundled().unwrap();
    for key in [
        "synetica_usb",
        "bridge",
        "dpt146",
        "hmd65",
        "wattnode",
        "iaq_plus",
        "adapter",
    ] {
        let device = &reference.devices[key];
        assert!(!device.help_setup.is_empty());
        assert!(!device.help_troubleshooting.is_empty());
    }
    assert_eq!(
        reference.devices["dpt146"].status,
        "Validated configuration"
    );
    assert_eq!(reference.devices["hmd65"].status, "Ready to test");
    assert_eq!(reference.devices["wattnode"].status, "Ready to test");
    assert_eq!(reference.premade.len(), 3);
    assert_eq!(reference.default_region, "us915_hybrid_fsb1");
    assert!(reference.regions[&reference.default_region].enabled);
    assert!(!reference.regions["eu868"].enabled);
    let ct = reference.registers["wattnode"]
        .iter()
        .find(|r| r.name == "CT amps 1")
        .unwrap();
    assert_eq!(ct.access, "R/W");
}

#[test]
fn bundled_devices_contain_no_staged_values_or_facts() {
    let reference: serde_json::Value =
        serde_json::from_str(include_str!("../assets/python-reference.json")).unwrap();
    for device in reference["devices"].as_object().unwrap().values() {
        assert!(device.get("facts").is_none());
        for reading in device["readouts"].as_array().unwrap() {
            assert!(reading.get("value").is_none());
            assert!(reading.get("quality").is_none());
        }
    }
}
