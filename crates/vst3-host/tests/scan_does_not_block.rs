//! A plugin a background scan has not reached yet must not hold whoever asks for it until the
//! scan ends. The UI asks for `plugin_info` every frame and creates a session's plugins on its own
//! thread, so on a first launch (no cache file) a session with a VST3 insert froze the window
//! for the whole scan. Its own binary: the scan here is cut short on purpose, and the registry
//! is process-wide.

#![cfg(unix)]

mod common;

use std::time::{Duration, Instant};

/// Polls `done` for up to `limit`.
fn wait_for(limit: Duration, done: impl Fn() -> bool) -> bool {
    let start = Instant::now();
    while start.elapsed() < limit {
        if done() {
            return true;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    done()
}

#[test]
fn a_plugin_the_running_scan_has_not_reached_does_not_wait_for_it() {
    use std::os::unix::fs::PermissionsExt as _;
    let dir = common::fixture_dir();
    let gain = soundcraft_vst3_host::scan_paths(&[dir.to_path_buf()]).into_iter().find(|d| d.name == "Fixture Gain").expect("fixture").id;

    // A scan child that never answers, like a plugin waiting on a licence dialog.
    let scanner = std::env::temp_dir().join(format!("sc-hanging-scanner-{}.sh", std::process::id()));
    std::fs::write(&scanner, "#!/bin/sh\nexec sleep 60\n").expect("write scanner");
    std::fs::set_permissions(&scanner, std::fs::Permissions::from_mode(0o755)).expect("chmod scanner");
    soundcraft_vst3_host::set_scanner(Some(scanner.clone()));

    let generation = soundcraft_vst3_host::scan_generation();
    soundcraft_vst3_host::scan_in_background();
    assert!(wait_for(Duration::from_secs(10), || soundcraft_vst3_host::progress().is_some()), "the scan started loading a file");
    assert!(soundcraft_vst3_host::scanning());
    assert!(!soundcraft_vst3_host::ready(&gain), "the scan has not reached the fixture");

    let asked = Instant::now();
    assert!(soundcraft_vst3_host::plugin_info(&gain).is_none(), "no info before the scan reaches it");
    assert!(asked.elapsed() < Duration::from_secs(2), "plugin_info waited {:?} for the scan", asked.elapsed());

    // Skip kills the hanging child; the scan ends and says so.
    soundcraft_vst3_host::skip_scan();
    assert!(wait_for(Duration::from_secs(10), || !soundcraft_vst3_host::scanning()), "the scan ended after Skip");
    assert!(soundcraft_vst3_host::scan_generation() > generation, "the end of a scan is announced");
    assert!(soundcraft_vst3_host::ready(&gain), "with no scan running, nothing waits");
    let _ = std::fs::remove_file(&scanner);
}
