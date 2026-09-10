use super::*;

#[test]
fn enumeration_preserves_usb_identity_and_handles_missing_descriptors() {
    for product in [None, Some("USB adapter".to_string())] {
        let info = port_info(serialport::SerialPortInfo {
            port_name: "COM103".into(),
            port_type: serialport::SerialPortType::UsbPort(serialport::UsbPortInfo {
                vid: 0x0403,
                pid: 0x6001,
                serial_number: Some("unit-123".into()),
                manufacturer: None,
                product: product.clone(),
            }),
        });
        assert_eq!(info.port, "COM103");
        assert_eq!(info.usb_vid, Some(0x0403));
        assert_eq!(info.usb_pid, Some(0x6001));
        assert_eq!(info.serial_number.as_deref(), Some("unit-123"));
        assert_eq!(
            info.description,
            product.unwrap_or_else(|| "USB serial interface".into())
        );
        assert!(!info.busy);
        assert!(info.identity.is_none());
    }
    for port_type in [
        serialport::SerialPortType::Unknown,
        serialport::SerialPortType::BluetoothPort,
    ] {
        let info = port_info(serialport::SerialPortInfo {
            port_name: "COM88".into(),
            port_type,
        });
        assert_eq!(info.description, "Serial interface");
        assert!(info.usb_vid.is_none() && info.usb_pid.is_none() && info.serial_number.is_none());
    }
}

#[test]
fn offline_backend_cannot_open_either_hardware_route() {
    let backend = OfflineBackend;
    assert!(backend.inventory().unwrap().is_empty());
    assert!(backend.open("COM103").is_err());
    assert!(
        backend
            .open_adapter(
                "COM103",
                crate::adapter::Settings {
                    slave: 1,
                    even: false,
                    two_stops: true
                }
            )
            .is_err()
    );
}
