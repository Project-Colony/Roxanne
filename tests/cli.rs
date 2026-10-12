use std::process::Command;

#[test]
fn version_flag_prints_the_version_without_starting_the_editor() {
    for flag in ["--version", "-V"] {
        let output = Command::new(env!("CARGO_BIN_EXE_roxanne"))
            .arg(flag)
            .output()
            .expect("run roxanne");
        assert!(
            output.status.success(),
            "{flag} exited with {}",
            output.status
        );
        assert_eq!(
            String::from_utf8_lossy(&output.stdout).trim_end(),
            format!("roxanne {}", env!("CARGO_PKG_VERSION")),
            "{flag}"
        );
    }
}
