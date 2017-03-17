#[macro_use]
extern crate log;
extern crate env_logger;

use std::process::Command;

#[test]
fn java_tests() {
    let _ = env_logger::init();

    // Run the gradle test that references this agent
    let javalib_path = std::env::current_dir().expect("No current dir").join("javalib");
    let mut gradle_path = javalib_path.join("gradlew");
    if cfg!(target_os = "windows") {
        gradle_path.set_extension("bat");
    }
    info!("Starting Gradle at {}", gradle_path.to_string_lossy());

    let output = Command::new(gradle_path)
        .current_dir(javalib_path)
        .arg("--no-daemon")
        .arg(":agent-tests:cleanTest")
        .arg(":agent-tests:test")
        .output()
        .expect("Couldn't start gradle");

    let stdout = String::from_utf8_lossy(&output.stdout);
    info!("stdout: {}", stdout);
    info!("stderr: {}", String::from_utf8_lossy(&output.stderr));

    assert!(output.status.success());
}
