# Interface structure and visual vocabulary

Reviewed September 11, 2026. This is the working design contract for Polygon
Device Configurator; recommendations below are distinguished from implemented work.

## What each section owns

| Section | Purpose | Contents |
| --- | --- | --- |
| Devices | Connection and measurement status | Physical devices, routes, readings, timestamps and communication state |
| Configuration | E5 bridge programming | Target, selected point table, changes, programming, backup and restore |
| History | Measurement history | Recorded series, units, timestamps, gaps and export |
| References | Model capabilities | Model descriptions, register maps and manufacturer manuals |
| Troubleshooting | Fault diagnosis | Application and device setup and recovery guidance |
| Settings | Application preferences | Appearance, display units and automatic polling preference |
| Diagnostics | Communication diagnostics | USB route selection, console operations, technical results and activity |

A USB interface is a route, not proof that a sensor is communicating. A point
table identifies the configured model, not independently the attached sensor.
A reference model is not a disconnected physical device. Keep these distinctions
visible without repeating long explanations beside every reading.

## Implemented behavior

- Reference model details now show descriptions, images, manuals, register-map
  navigation and troubleshooting instead of disconnected states and empty readings.
- ATI F12 no longer inherits the USB adapter's support description.
- Attention banners share a treatment for missing configuration targets and the
  adapter interlock. Each contains the state and the next useful action.
- Hover outlines use blue/cyan; orange is reserved for attention.
- Diagnostics uses "Read now", matching device actions. Backup explicitly names
  the E5 bridge. Configuration references use the actual singular navigation label.
- Detail headings can wrap. Screenshot capture includes model-reference details
  separately from physical-device details.

## Visual vocabulary

| Treatment | Meaning |
| --- | --- |
| Cyan / dark blue | Selection, navigation and interaction |
| Orange with explicit text | Attention, blocked action or configuration concern |
| Neutral border and surface | Group related information; not inherently a button |
| Device photograph | Model recognition; never connection or validation evidence |
| Large value with unit | Measurement |
| Plain human-readable timestamp | When the last good reading was received |
| Fixed bottom status area | Manual operation progress, queued actions and completion |

Use labels with verbs for actions and nouns for destinations. Keep routine polling
quiet. Retain last good values on failure, visibly identify stale data, and never
substitute zero for missing data. Color must reinforce text, not carry meaning alone.
Use the bundled Polygon artwork; do not invent service-line icons.

## Layout requirements

Configuration separates programming from save and copy utilities under File tools,
uses attention banners for configuration changes, point removals and backup
failures, and groups the overview into Connections and Sensors. This grouping
does not assert a verified wiring topology. Cards wrap instead of requiring a
horizontal trip through the whole device list.

Reference register maps now contain definitions and native units without live
readout columns or display-unit controls. Device register inspection retains
those controls. Screenshots include both variants.

Usability requirements include clear technical wording, readable controls, predictable navigation, keyboard focus and usable layouts at smaller window sizes. Formal accessibility certification has not been performed.

## Remaining refinements

1. Validate the Connections/Sensors grouping when the
   same model has been read through multiple routes.
2. Standardize remaining inline warnings and field errors using severity-specific
   shared components; do not turn informational text into warning banners.
3. Review compact-window layouts and keyboard focus separately. Hover styling
   alone is not a complete accessibility assessment.

These refinements are pending verification.
