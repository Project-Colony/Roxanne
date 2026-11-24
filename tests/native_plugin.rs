use roxanne::plugins::{PluginConfig, PluginManager};
use std::env::consts::{DLL_PREFIX, DLL_SUFFIX};

/// Loads the sample plugin from `examples/`, which `cargo test` builds in the
/// same target profile directory as the test binaries, and checks that its callbacks reach the status bar.
#[test]
fn sample_plugin_loads_and_reports_its_status() {
    let test_binary = std::env::current_exe().expect("test binary path");
    let name = format!("{DLL_PREFIX}sample_plugin{DLL_SUFFIX}");
    // The test binary sits in `<profile>/deps/` with the older Cargo layout and
    // deeper, in `<profile>/build/<package>/<hash>/out/`, with the layout Cargo
    // 1.100 uses. The example is in `<profile>/examples/` either way.
    let library = test_binary
        .ancestors()
        .skip(1)
        .map(|dir| dir.join("examples").join(&name))
        .find(|library| library.is_file())
        .unwrap_or_else(|| test_binary.with_file_name(&name));
    assert!(
        library.is_file(),
        "{} is missing: run `cargo test` without a target filter so the example is built",
        library.display()
    );

    let config = PluginConfig {
        enabled: Vec::new(),
        dynamic: vec![library.to_string_lossy().into_owned()],
    };
    let (mut manager, warnings) = PluginManager::new(&config);
    assert!(warnings.is_empty(), "{warnings:?}");

    manager.on_text_changed("one two\nthree", "demo.txt");
    let statuses = manager.statuses();
    assert!(
        statuses
            .iter()
            .any(|status| status.label == "Stats" && status.value == "3 words, 2 lines"),
        "{statuses:?}"
    );
}
