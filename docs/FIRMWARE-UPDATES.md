# Firmware updates

Firmware flashing is not implemented in the Rust application. The Python firmware-manifest inspection remains a prototype.

Future implementation needs a verified target model/revision, test image, address/range, erase policy, settings-preservation requirements, and recovery tests. The application must finish active reads, release hardware ownership, validate the image, verify programming, and confirm device identity and readings afterward.

Private manufacturer procedures and notes derived from them are kept outside version control. This repository does not publish device-specific flashing instructions.
