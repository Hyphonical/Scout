// Integration tests for Scout

use std::fs;
use std::path::Path;
use std::process::Command;

#[test]
fn test_scan_basic() {
    // This test runs against the demo folder created by CI
    let test_dir = Path::new("demo");
    
    if !test_dir.exists() {
        // Skip if not in CI environment
        eprintln!("Skipping test: demo directory not found");
        return;
    }

    let output = Command::new("cargo")
        .args(&["run", "--release", "--", "scan", "-d", "demo"])
        .output()
        .expect("Failed to run scout scan");

    assert!(output.status.success(), "Scan command failed");
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    println!("Scan output:\n{}", stdout);
    
    // Check that it found images
    assert!(stdout.contains("Found") || stdout.contains("images"), 
            "Expected to find images in output");
}

#[test]
fn test_version_display() {
    let output = Command::new("cargo")
        .args(&["run", "--release", "--", "--version"])
        .output()
        .expect("Failed to run scout --version");

    assert!(output.status.success(), "Version command failed");
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("scout"), "Expected 'scout' in version output");
}

#[test]
fn test_help_display() {
    let output = Command::new("cargo")
        .args(&["run", "--release", "--", "--help"])
        .output()
        .expect("Failed to run scout --help");

    assert!(output.status.success(), "Help command failed");
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("scan") && stdout.contains("search"), 
            "Expected scan and search in help output");
}

#[test]
fn test_sidecar_creation() {
    let test_dir = Path::new("demo");
    
    if !test_dir.exists() {
        eprintln!("Skipping test: demo directory not found");
        return;
    }

    // Run scan
    let output = Command::new("cargo")
        .args(&["run", "--release", "--", "scan", "-d", "demo"])
        .output()
        .expect("Failed to run scout scan");

    assert!(output.status.success(), "Scan command failed");

    // Check for .scout directory
    let scout_dir = test_dir.join(".scout");
    assert!(scout_dir.exists(), "Expected .scout directory to be created");
    
    // Check that sidecars were created
    let sidecars: Vec<_> = fs::read_dir(&scout_dir)
        .expect("Failed to read .scout directory")
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().and_then(|s| s.to_str()) == Some("msgpack"))
        .collect();
    
    assert!(!sidecars.is_empty(), "Expected sidecar files to be created");
}
