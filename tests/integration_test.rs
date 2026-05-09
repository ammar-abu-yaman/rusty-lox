use std::fs;
use std::path::PathBuf;
use std::process::Command;

#[cfg(feature = "integration-tests")]
#[test]
fn test_lox_files() {
    // Ensure the binary is built
    let build_status = Command::new("cargo")
        .arg("build")
        .status()
        .expect("Failed to build project");
    assert!(build_status.success(), "Build failed");

    let test_dir = PathBuf::from("tests-files");
    let entries = fs::read_dir(test_dir).expect("Could not read tests-files directory");

    let mut failed_tests = Vec::new();

    for entry in entries {
        let entry = entry.expect("Could not read entry");
        let path = entry.path();

        if path.extension().and_then(|s| s.to_str()) == Some("lox") {
            let test_name = path.file_stem().unwrap().to_str().unwrap();
            let expected_path = path.with_extension("expected");

            if !expected_path.exists() {
                continue;
            }

            let expected_output = fs::read_to_string(&expected_path)
                .expect("Could not read expected file")
                .replace("\r\n", "\n");

            // Run the binary directly from target/debug
            // The name comes from Cargo.toml: codecrafters-interpreter
            let output = Command::new("./target/debug/codecrafters-interpreter")
                .args(["run", path.to_str().unwrap()])
                .output()
                .expect("Failed to execute command");

            let actual_output = String::from_utf8_lossy(&output.stdout)
                .replace("\r\n", "\n")
                .trim_end()
                .to_string();

            let expected_output_trimmed = expected_output.trim_end().to_string();

            if actual_output != expected_output_trimmed {
                failed_tests.push(format!(
                    "Test '{}' failed.\nExpected:\n---\n{}\n---\nActual:\n---\n{}\n---",
                    test_name, expected_output_trimmed, actual_output
                ));
            }
        }
    }

    if !failed_tests.is_empty() {
        panic!(
            "The following {} tests failed:\n\n{}",
            failed_tests.len(),
            failed_tests.join("\n\n")
        );
    }
}
