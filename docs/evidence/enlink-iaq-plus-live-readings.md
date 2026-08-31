# enLink IAQ Plus live console evidence

Captured read-only from COM5 on 2026-08-31.

## Identity

- Observed part number: 003-ADZ-301
- Firmware code: FW-AQ-VCP+
- Firmware version: 5.06
- Region: North American band, Hybrid FSB #1, 915 MHz
- DevEUI: recorded in the private device profile and credential artifact
- USB identity: STM32 VID 0483 / PID 5740; shared with other Synetica products

## Installed/readable channels

The Configure Device page exposed:

- primary temperature;
- VOC-module temperature;
- relative humidity;
- barometric pressure;
- CO2-equivalent estimate;
- bVOC estimate;
- IAQ value and accuracy state;
- GSS CO2 module, including module version/serial, auto-calibration state, and reading;
- particle-sensor options menu, whose second measurement page still needs capture.

Example captured values were 24.7 °C, 54 %RH, 997 mbar, 500 ppm CO2e, 0.50 ppm bVOC, IAQ 25 with medium accuracy, and a GSS reading of 88 ppm. These values are evidence of parsing only, not an accuracy acceptance test. The unusually low GSS value must not be treated as a validated ambient CO2 reference.

## Radio state

- Joined
- JoinEUI/AppEUI: captured in the private credential artifact
- AppKey: captured in the private credential artifact
- Transmit interval: 15 minutes
- Uplink port: 1
- Receive port: all
- Join check: 3 hours

## Console behavior

- DTR must be enabled and RTS disabled.
- The login is the final four hexadecimal characters of the normalized banner DevEUI.
- Menu responses can arrive after the serial handle is reopened. The Windows helper therefore uses a bounded reopen/read state machine rather than assuming a single request/response exchange.
- Background discovery reads only the unauthenticated banner and caches identity. Authenticated reads occur only after an explicit technician action.
