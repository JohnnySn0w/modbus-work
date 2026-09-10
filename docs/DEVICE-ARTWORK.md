# Device artwork

Seven user-supplied photos are embedded in native/modbus-configurator/assets/devices: E5 bridge, DPT146, HMD65, WattNode, IAQ Plus, USB adapter and ATI F12. Original image bytes are retained; fitting preserves aspect ratio and uses white photo panels with cached texture decoding. No network is needed at runtime. Labels/serials visible in photos are not device identity evidence.

The application icon is the user-approved hand trace in assets/branding/polygon-icon.svg. Its 512px PNG supplies the window icon; ICO contains 16/24/32/48/64/128/256px sizes for Windows. The original supplied favicon is retained. Generic egui vector fallback remains for missing artwork, not a separate Synetica USB product.

Palette and typography are implemented in native/modbus-configurator/src/brand.rs. Artifact source details live beside the assets; private guideline pages stay local.
