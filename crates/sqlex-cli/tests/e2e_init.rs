//! End-to-end tests for the init command

#![allow(deprecated)]

use std::fs;

use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::TempDir;

#[test]
fn test_init_creates_project_structure() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let project_name = "test_project";
    let project_path = temp_dir.path().join(project_name);

    let mut cmd = Command::cargo_bin("sqlex").expect("Failed to find binary");
    cmd.arg("init")
        .arg(project_name)
        .current_dir(temp_dir.path());

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Initialized sqlex project"));

    // Verify project directory was created
    assert!(project_path.exists());
    assert!(project_path.is_dir());

    // Verify config file was created
    let config_path = project_path.join("sqlex.yaml");
    assert!(config_path.exists());

    // Verify migrations directory was created
    let migrations_path = project_path.join("migrations");
    assert!(migrations_path.exists());
    assert!(migrations_path.is_dir());
}

#[test]
fn test_init_validates_config_content() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let project_name = "test_project";
    let project_path = temp_dir.path().join(project_name);

    let mut cmd = Command::cargo_bin("sqlex").expect("Failed to find binary");
    cmd.arg("init")
        .arg(project_name)
        .current_dir(temp_dir.path());

    cmd.assert().success();

    // Read and validate config file content
    let config_path = project_path.join("sqlex.yaml");
    let config_content = fs::read_to_string(config_path).expect("Failed to read config file");

    // Verify config contains expected fields
    assert!(config_content.contains("name:"));
    assert!(config_content.contains("dialect:"));
    assert!(config_content.contains("migrations:"));
    assert!(config_content.contains("analyzer:"));
    assert!(config_content.contains("generators:"));
}

#[test]
fn test_init_in_existing_directory() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let project_name = "existing_project";
    let project_path = temp_dir.path().join(project_name);

    // Create directory first
    fs::create_dir(&project_path).expect("Failed to create directory");

    let mut cmd = Command::cargo_bin("sqlex").expect("Failed to find binary");
    cmd.arg("init")
        .arg(project_name)
        .current_dir(temp_dir.path());

    cmd.assert().success();

    // Verify config file was created
    let config_path = project_path.join("sqlex.yaml");
    assert!(config_path.exists());
}

#[test]
fn test_init_skips_existing_config() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let project_name = "test_project";

    // First init
    let mut cmd = Command::cargo_bin("sqlex").expect("Failed to find binary");
    cmd.arg("init")
        .arg(project_name)
        .current_dir(temp_dir.path());
    cmd.assert().success();

    // Second init - should skip existing config
    let mut cmd = Command::cargo_bin("sqlex").expect("Failed to find binary");
    cmd.arg("init")
        .arg(project_name)
        .current_dir(temp_dir.path());
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Config file already exists"));
}
