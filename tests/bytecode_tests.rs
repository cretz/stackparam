#[macro_use]
extern crate log;
extern crate env_logger;
extern crate zip;
extern crate stackparam;

use std::env;
use std::path::PathBuf;
use std::fs::File;
use zip::ZipArchive;
use std::io::{Cursor, Read};
use stackparam::bytecode::io::reader::ClassReader;
use stackparam::bytecode::io::writer::ClassWriter;

#[test]
#[ignore]
fn bytecode_tests() {
    let _ = env_logger::init();

    // Find rt.jar
    let java_home = env::var("JAVA_HOME").expect("Unable to find JAVA_HOME");
    let java_home_path = PathBuf::from(java_home);
    let rt_jar_path: PathBuf = {
        // Try JDK first
        let mut rt_maybe = java_home_path.join("jre/lib/rt.jar");
        if !rt_maybe.is_file() {
            rt_maybe = java_home_path.join("lib/rt.jar");
            assert!(rt_maybe.is_file(), "Unable to find rt.jar on JAVA_HOME path: {}", java_home_path.display());
        }
        rt_maybe.to_owned()
    };

    // Check each class
    let file = File::open(rt_jar_path).unwrap();
    let mut rt_jar = ZipArchive::new(file).unwrap();
    for i in 0..rt_jar.len() {
        let mut file = rt_jar.by_index(i).unwrap();
        if file.name().ends_with(".class") {
            // Read the class and just write it back and confirm same bytes
            let mut in_bytes: Vec<u8> = Vec::new();
            file.read_to_end(&mut in_bytes).expect(&format!("Cannot read {}", file.name()));
            let mut in_curs = Cursor::new(in_bytes);
            let class_file = ClassReader::read_class(&mut in_curs).expect(&format!("Failed parsing {}", file.name()));
            let mut out_curs = Cursor::new(Vec::new());
            ClassWriter::new(&mut out_curs).write_class(&class_file).expect(&format!("Failed writing {}", file.name()));

            in_bytes = in_curs.into_inner();
            let out_bytes = out_curs.into_inner();
            debug!("For {} - {} and {}", file.name(), in_bytes.len(), out_bytes.len());
            assert_eq!(in_bytes.as_slice(), out_bytes.as_slice(), "Not same for {}", file.name());
        }
    }
}