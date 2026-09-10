use modbus_configurator::{
    console::{ConsoleParser, PromptState},
    contract::*,
    service::Service,
};
use std::time::Duration;

#[test]
fn terminal_control_strings_cannot_inject_prompts_across_fragments() {
    for sequence in [
        b"\x1b]title;Password:\x07".as_slice(),
        b"\x1b]title;\x1bXPassword:\x07".as_slice(),
    ] {
        for size in 1..=sequence.len() {
            let mut parser = ConsoleParser::default();
            for chunk in sequence.chunks(size) {
                parser.feed(chunk);
            }
            assert_eq!(parser.state(), PromptState::Unknown);
            assert_eq!(
                parser.feed(b"\x1bXenLink Main Menu:"),
                PromptState::MainMenu
            );
            assert_eq!(parser.text(), "enLink Main Menu:");
        }
    }
}

#[test]
fn every_chunk_size_recognizes_fragmented_ansi_prompt() {
    let transcript =
        b"\0\x1b[32mPassword:\x1b[0m\r\nenLink Main Menu:\rC\rModbus Configuration Menu:";
    for size in 1..=transcript.len() {
        let mut parser = ConsoleParser::default();
        for chunk in transcript.chunks(size) {
            parser.feed(chunk);
        }
        assert_eq!(parser.state(), PromptState::ModbusMenu, "chunk size {size}");
    }
}

#[test]
fn stale_menu_cannot_override_later_password_or_import_prompt() {
    let mut parser = ConsoleParser::default();
    parser.feed(b"enLink Main Menu:\nModbus Import/Export Menu:\n");
    assert_eq!(
        parser.feed(b"Finish the import with an empty line"),
        PromptState::ImportInput
    );
    assert_eq!(parser.feed(b"\rPassword:"), PromptState::Password);
    parser.reset();
    assert_eq!(parser.state(), PromptState::Unknown);
    assert_eq!(
        parser.feed(b"unrecognized firmware output"),
        PromptState::Unknown
    );
}

#[test]
fn window_is_bounded_and_osc_contents_are_ignored() {
    let mut parser = ConsoleParser::default();
    parser.feed(b"Password:");
    parser.feed(&vec![
        b' ';
        modbus_configurator::console::MAX_RESPONSE_BYTES + 1024
    ]);
    assert_eq!(parser.state(), PromptState::Unknown);
    parser.feed(b"\x1b]0;enLink Main Menu:\x1b");
    assert_eq!(parser.feed(b"\\"), PromptState::Unknown);
}

#[test]
fn json_replay_runs_through_background_service_with_correlated_events() {
    let service = Service::start().unwrap();
    let command: Command = serde_json::from_str(include_str!("fixtures/navigation.json")).unwrap();
    service.send(command).unwrap();
    let mut states = vec![];
    loop {
        let event = service.events.recv_timeout(Duration::from_secs(3)).unwrap();
        assert_eq!(event.request_id, 1);
        let encoded = serde_json::to_string(&event).unwrap();
        let decoded: Event = serde_json::from_str(&encoded).unwrap();
        match decoded.kind {
            EventKind::PromptState { state } => states.push(state),
            EventKind::Result { .. } => break,
            EventKind::Error { message, .. } => panic!("{message}"),
            _ => {}
        }
    }
    assert_eq!(
        states,
        vec![
            PromptState::Unknown,
            PromptState::Password,
            PromptState::MainMenu,
            PromptState::MainMenu,
            PromptState::ModbusMenu,
            PromptState::ImportExport,
            PromptState::ModbusMenu,
            PromptState::ReadComplete,
            PromptState::Continue
        ]
    );
}

#[test]
fn replay_rejects_hardware_identity_gate_instead_of_silently_ignoring_it() {
    let service = Service::start().unwrap();
    service
        .send(Command {
            request_id: 9,
            port: Some("COM5".into()),
            expected_identity: None,
            operation: Operation::Replay { chunks: vec![] },
        })
        .unwrap();
    let event = service.events.recv_timeout(Duration::from_secs(3)).unwrap();
    assert_eq!(event.request_id, 9);
    assert!(matches!(
        event.kind,
        EventKind::Error {
            code: ErrorCode::InvalidRequest,
            ..
        }
    ));
}

#[test]
fn offline_service_returns_no_ports_and_rejects_hardware_requests() {
    use modbus_configurator::{contract::*, service::Service};
    let service = Service::offline().unwrap();
    service
        .send(Command {
            request_id: 1,
            port: None,
            expected_identity: None,
            operation: Operation::Inventory,
        })
        .unwrap();
    let event = service
        .events
        .recv_timeout(std::time::Duration::from_secs(2))
        .unwrap();
    assert!(matches!(event.kind, EventKind::PortSnapshot { ports } if ports.is_empty()));
    service
        .send(Command {
            request_id: 2,
            port: Some("COM41".into()),
            expected_identity: Some(Identity {
                model: "ENL-MOD-32".into(),
                firmware: "3.6".into(),
            }),
            operation: Operation::BridgeReadAll,
        })
        .unwrap();
    let event = service
        .events
        .recv_timeout(std::time::Duration::from_secs(2))
        .unwrap();
    assert!(matches!(
        event.kind,
        EventKind::Error {
            code: ErrorCode::IdentityMismatch,
            ..
        }
    ));
}
