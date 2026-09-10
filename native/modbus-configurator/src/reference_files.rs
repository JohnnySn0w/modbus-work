//! Extract allowlisted bundled references and open them with the Windows file handler.

/// Write an allowlisted artifact into a unique directory under the supplied root.
pub(super) fn extract_reference_artifact(
    path: &str,
    root: &std::path::Path,
) -> std::io::Result<std::path::PathBuf> {
    let bytes: &[u8] = match path {
        "docs/reference/ati-f12-operation-manual.pdf" => {
            include_bytes!("../../../docs/reference/ati-f12-operation-manual.pdf")
        }
        "docs/reference/e5-hardware-guide.pdf" => {
            include_bytes!("../../../docs/reference/e5-hardware-guide.pdf")
        }
        "docs/reference/hmd65-user-guide.pdf" => {
            include_bytes!("../../../docs/reference/hmd65-user-guide.pdf")
        }
        "docs/reference/vaisala-dpt146-user-guide-M211372EN-E.pdf" => {
            include_bytes!("../../../docs/reference/vaisala-dpt146-user-guide-M211372EN-E.pdf")
        }
        "docs/reference/wattnode-installation-manual.pdf" => {
            include_bytes!("../../../docs/reference/wattnode-installation-manual.pdf")
        }
        "docs/reference/wattnode-reference-manual.pdf" => {
            include_bytes!("../../../docs/reference/wattnode-reference-manual.pdf")
        }

        "artifacts/bridge-config/README.md" => {
            include_bytes!("../../../artifacts/bridge-config/README.md")
        }
        "artifacts/bridge-config/vaisala-dpt146-manifest.yaml" => {
            include_bytes!("../../../artifacts/bridge-config/vaisala-dpt146-manifest.yaml")
        }
        "artifacts/device-profiles/to-test/vaisala-hmd65.yaml" => {
            include_bytes!("../../../artifacts/device-profiles/to-test/vaisala-hmd65.yaml")
        }
        "artifacts/device-profiles/to-test/wattnode-wnd-m1-mb.yaml" => {
            include_bytes!("../../../artifacts/device-profiles/to-test/wattnode-wnd-m1-mb.yaml")
        }
        "artifacts/device-profiles/synetica-enlink-iaq-plus.yaml" => {
            include_bytes!("../../../artifacts/device-profiles/synetica-enlink-iaq-plus.yaml")
        }
        "docs/reference/usb-comi-tb-manual.pdf" => {
            include_bytes!("../../../docs/reference/usb-comi-tb-manual.pdf")
        }
        "artifacts/bridge-config/ati-badger-f12-d12-documentation-test.tsv" => include_bytes!(
            "../../../artifacts/bridge-config/ati-badger-f12-d12-documentation-test.tsv"
        ),
        _ => return Err(std::io::Error::other("Unknown bundled artifact")),
    };
    let unique = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(std::io::Error::other)?
        .as_nanos();
    let folder = root.join(format!("modbus-reference-{}-{unique}", std::process::id()));
    std::fs::create_dir(&folder)?;
    let output = folder.join(
        std::path::Path::new(path)
            .file_name()
            .ok_or_else(|| std::io::Error::other("Invalid artifact name"))?,
    );
    std::fs::write(&output, bytes)?;
    Ok(output)
}
/// Extract a reference and launch the Windows file handler; propagate failures.
pub(super) fn open_reference_artifact(path: &str) -> std::io::Result<()> {
    let output = extract_reference_artifact(path, &std::env::temp_dir())?;
    #[cfg(windows)]
    {
        std::process::Command::new("explorer.exe")
            .arg(&output)
            .spawn()?;
    }
    #[cfg(not(windows))]
    {
        return Err(std::io::Error::other(
            "Reference opening is implemented for Windows only",
        ));
    }
    Ok(())
}
