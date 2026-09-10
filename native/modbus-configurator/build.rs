use std::{env, path::PathBuf, process::Command};
fn main() {
    println!("cargo:rerun-if-changed=assets/branding/polygon.ico");
    println!("cargo:rerun-if-changed=assets/branding/app.rc");
    if env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }
    let sdk = PathBuf::from(env::var_os("ProgramFiles(x86)").expect("Windows SDK location"))
        .join("Windows Kits/10/bin");
    let mut compilers: Vec<_> = std::fs::read_dir(sdk)
        .expect("Windows SDK installed")
        .filter_map(Result::ok)
        .map(|e| e.path().join("x64/rc.exe"))
        .filter(|p| p.is_file())
        .collect();
    compilers.sort();
    let compiler = compilers.last().expect("Windows SDK resource compiler");
    let output = PathBuf::from(env::var_os("OUT_DIR").unwrap()).join("polygon.res");
    assert!(
        Command::new(compiler)
            .args(["/nologo", "/fo"])
            .arg(&output)
            .arg("assets/branding/app.rc")
            .status()
            .expect("Compile Windows resources")
            .success()
    );
    println!(
        "cargo:rustc-link-arg-bin=modbus-configurator={}",
        output.display()
    );
}
