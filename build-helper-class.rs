#[macro_use]
extern crate log;
extern crate env_logger;

use std::process::Command;

fn main() {
    // Only build the class helper if it's not already there
    //    if !Path::new("javalib/native/build/classes/main/stackparam/StackParamNative.class").exists() {
    let javalib_path = std::env::current_dir().expect("No current dir").join("javalib");
    let mut gradle_path = javalib_path.join("gradlew");
    if cfg!(target_os = "windows") {
        gradle_path.set_extension("bat");
    }

    println!("Starting Gradle at {}", gradle_path.to_string_lossy());

    let output = Command::new(gradle_path)
        .current_dir(javalib_path)
        .arg("--no-daemon")
        .arg(":native:classes")
        .output()
        .expect("Couldn't start gradle");

    println!("Gradle stdout: {}", String::from_utf8_lossy(&output.stdout));
    println!("Gradle stderr: {}", String::from_utf8_lossy(&output.stderr));
    assert!(output.status.success());
    //    }
}
