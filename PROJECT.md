# Product scope

Polygon Device Configurator connects technicians to real measurements and repeatable configuration workflows on Windows. The primary E5 workflow is connect, select sensor/TSV, review, back up, program, verify and observe live data.

The app supports direct Modbus USB transport and E5 bridge transport. Native Synetica sensors such as IAQ Plus require their own console protocol support; they are not Modbus instruments behind an E5 bridge. Several Synetica product families and firmware updates are future scope.

COM numbers, photo labels and selected profiles are not sensor identities. USB routes are dynamic. On a shared RS-485 bus use one active master; turning GUI polling off is not equivalent to switching the E5 bridge off.

Current implementation and hardware acceptance are maintained in [CURRENT-STATUS.md](CURRENT-STATUS.md), with outstanding work in [GOALS.md](GOALS.md). Historical shipment/bench notes were preserved in docs/archive/2026-09-10-before-documentation-pass/PROJECT.md. Downstream DSP integration remains deferred; historical Loriot evidence does not mean the application currently implements an upstream integration.
