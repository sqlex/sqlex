//! End-to-end tests for the script command

#![allow(deprecated)]

use std::fs;

use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::TempDir;

/// Helper function to create a basic sqlex project for testing
fn create_test_project(temp_dir: &TempDir, project_name: &str) -> std::path::PathBuf {
    let mut cmd = Command::cargo_bin("sqlex").expect("Failed to find binary");
    cmd.arg("init")
        .arg(project_name)
        .current_dir(temp_dir.path());
    cmd.assert().success();

    temp_dir.path().join(project_name)
}

#[test]
fn test_script_adds_generator_successfully() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let project_name = "test_project";
    let project_path = create_test_project(&temp_dir, project_name);

    // Run script command
    let mut cmd = Command::cargo_bin("sqlex").expect("Failed to find binary");
    cmd.arg("script").current_dir(&project_path);

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("script generator added successfully"));
}

#[test]
fn test_script_updates_config_file() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let project_name = "test_project";
    let project_path = create_test_project(&temp_dir, project_name);

    // Run script command
    let mut cmd = Command::cargo_bin("sqlex").expect("Failed to find binary");
    cmd.arg("script").current_dir(&project_path);
    cmd.assert().success();

    // Read and validate config file content
    let config_path = project_path.join("sqlex.yaml");
    let config_content = fs::read_to_string(config_path).expect("Failed to read config file");

    // Verify script generator was added to config
    assert!(config_content.contains("script_generator"));
    assert!(config_content.contains("generator: script"));
    assert!(config_content.contains("scripts/generate.ts"));
}

#[test]
fn test_script_creates_files_and_directories() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let project_name = "test_project";
    let project_path = create_test_project(&temp_dir, project_name);

    // Run script command
    let mut cmd = Command::cargo_bin("sqlex").expect("Failed to find binary");
    cmd.arg("script").current_dir(&project_path);
    cmd.assert().success();

    // Verify scripts directory was created
    let scripts_dir = project_path.join("scripts");
    assert!(scripts_dir.exists());
    assert!(scripts_dir.is_dir());

    // Verify example script was created
    let example_script = scripts_dir.join("generate.ts");
    assert!(example_script.exists());
    let script_content = fs::read_to_string(&example_script).expect("Failed to read script");
    assert!(script_content.contains("project.tables"));

    // Verify type definitions were created
    let type_defs = scripts_dir.join("sqlex-types.d.ts");
    assert!(type_defs.exists());
    let type_defs_content = fs::read_to_string(&type_defs).expect("Failed to read type defs");
    assert!(type_defs_content.contains("interface Table"));
}

#[test]
fn test_script_fails_when_already_exists() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let project_name = "test_project";
    let project_path = create_test_project(&temp_dir, project_name);

    // First run - should succeed
    let mut cmd = Command::cargo_bin("sqlex").expect("Failed to find binary");
    cmd.arg("script").current_dir(&project_path);
    cmd.assert().success();

    // Second run - should fail
    let mut cmd = Command::cargo_bin("sqlex").expect("Failed to find binary");
    cmd.arg("script").current_dir(&project_path);
    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("Script generator already exists"));
}

#[test]
fn test_script_fails_without_config() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");

    // Run script command in directory without sqlex.yaml
    let mut cmd = Command::cargo_bin("sqlex").expect("Failed to find binary");
    cmd.arg("script").current_dir(temp_dir.path());

    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("sqlex.yaml not found"));
}



