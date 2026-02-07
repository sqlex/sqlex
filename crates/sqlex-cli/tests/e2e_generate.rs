//! End-to-end tests for the generate command

#![allow(deprecated)]

use std::fs;

use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::TempDir;

/// Helper function to create a test project with config and migrations
fn setup_test_project(temp_dir: &TempDir) -> std::path::PathBuf {
    let project_path = temp_dir.path().to_path_buf();

    // Create config file
    let config_content = r#"
name: test_project
dialect: postgres
migrations: migrations
analyzer: static
generators:
  - name: debug_output
    generator: debug
    config:
      output: output/debug.txt
"#;
    fs::write(project_path.join("sqlex.yaml"), config_content)
        .expect("Failed to write config file");

    // Create migrations directory
    let migrations_dir = project_path.join("migrations");
    fs::create_dir_all(&migrations_dir).expect("Failed to create migrations directory");

    // Create a simple migration file
    let migration_content = r#"
CREATE TABLE users (
    id SERIAL PRIMARY KEY,
    username VARCHAR(255) NOT NULL,
    email VARCHAR(255) NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT NOW()
);

CREATE TABLE posts (
    id SERIAL PRIMARY KEY,
    user_id INTEGER NOT NULL REFERENCES users(id),
    title VARCHAR(255) NOT NULL,
    content TEXT,
    created_at TIMESTAMP NOT NULL DEFAULT NOW()
);
"#;
    fs::write(migrations_dir.join("000_initial.sql"), migration_content)
        .expect("Failed to write migration file");

    project_path
}

#[test]
fn test_generate_runs_successfully() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let project_path = setup_test_project(&temp_dir);

    let mut cmd = Command::cargo_bin("sqlex").expect("Failed to find binary");
    cmd.arg("generate").current_dir(&project_path);

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Generated successfully"));
}

#[test]
fn test_generate_creates_output_files() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let project_path = setup_test_project(&temp_dir);

    let mut cmd = Command::cargo_bin("sqlex").expect("Failed to find binary");
    cmd.arg("generate").current_dir(&project_path);

    cmd.assert().success();

    // Verify output directory was created
    let output_dir = project_path.join("output");
    assert!(output_dir.exists(), "Output directory should exist");
    assert!(output_dir.is_dir(), "Output path should be a directory");
}

#[test]
fn test_generate_fails_without_config() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");

    let mut cmd = Command::cargo_bin("sqlex").expect("Failed to find binary");
    cmd.arg("generate").current_dir(temp_dir.path());

    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("Config file not found"));
}
