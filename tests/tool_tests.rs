// Copyright (c) 2025-present, fjall-rs
// This source code is licensed under both the Apache 2.0 and MIT License
// (found in the LICENSE-* files in the repository)

#![cfg(feature = "tool")]

use assert_cmd::cargo::cargo_bin_cmd;
use predicates::prelude::*;
use sfa::Writer;
use std::fs;
use std::io::Write;
use tempfile::TempDir;

fn path_to_arg(path: &std::path::Path) -> String {
    path.to_string_lossy().to_string()
}

fn create_test_files(
    dir: &TempDir,
) -> (std::path::PathBuf, std::path::PathBuf, std::path::PathBuf) {
    let file1 = dir.path().join("file1.txt");
    let file2 = dir.path().join("file2.txt");
    let file3 = dir.path().join("file3.dat");

    fs::write(&file1, b"Hello, world!\n").unwrap();
    fs::write(&file2, b"Test content\nLine 2\n").unwrap();
    fs::write(&file3, b"\x00\x01\x02\x03\xff\xfe\xfd").unwrap();

    (file1, file2, file3)
}

// ============================================================================
// CREATE COMMAND TESTS
// ============================================================================

#[test]
fn test_create_basic() {
    let dir = tempfile::tempdir().unwrap();
    let (file1, file2, _) = create_test_files(&dir);
    let output = dir.path().join("archive.sfa");

    cargo_bin_cmd!()
        .arg("create")
        .arg(path_to_arg(&output))
        .arg(path_to_arg(&file1))
        .arg(path_to_arg(&file2))
        .assert()
        .success()
        .stdout(predicate::str::contains("Created SFA file"))
        .stdout(predicate::str::contains("with 2 sections"));

    assert!(output.exists());
}

#[test]
fn test_create_alias_c() {
    let dir = tempfile::tempdir().unwrap();
    let (file1, file2, _) = create_test_files(&dir);
    let output = dir.path().join("archive.sfa");

    cargo_bin_cmd!()
        .arg("c")
        .arg(path_to_arg(&output))
        .arg(path_to_arg(&file1))
        .arg(path_to_arg(&file2))
        .assert()
        .success()
        .stdout(predicate::str::contains("Created SFA file"))
        .stdout(predicate::str::contains("with 2 sections"));

    assert!(output.exists());
}

#[test]
fn test_create_single_file() {
    let dir = tempfile::tempdir().unwrap();
    let (file1, _, _) = create_test_files(&dir);
    let output = dir.path().join("archive.sfa");

    cargo_bin_cmd!()
        .arg("create")
        .arg(path_to_arg(&output))
        .arg(path_to_arg(&file1))
        .assert()
        .success()
        .stdout(predicate::str::contains("with 1 sections"));

    assert!(output.exists());
}

#[test]
fn test_create_multiple_files() {
    let dir = tempfile::tempdir().unwrap();
    let (file1, file2, file3) = create_test_files(&dir);
    let output = dir.path().join("archive.sfa");

    cargo_bin_cmd!()
        .arg("create")
        .arg(path_to_arg(&output))
        .arg(path_to_arg(&file1))
        .arg(path_to_arg(&file2))
        .arg(path_to_arg(&file3))
        .assert()
        .success()
        .stdout(predicate::str::contains("with 3 sections"));

    assert!(output.exists());
}

#[test]
fn test_create_force_overwrite() {
    let dir = tempfile::tempdir().unwrap();
    let (file1, _, _) = create_test_files(&dir);
    let output = dir.path().join("archive.sfa");

    // Create initial archive
    cargo_bin_cmd!()
        .arg("create")
        .arg(path_to_arg(&output))
        .arg(path_to_arg(&file1))
        .assert()
        .success();

    // Overwrite with force flag
    cargo_bin_cmd!()
        .arg("create")
        .arg("--force")
        .arg(path_to_arg(&output))
        .arg(&file1)
        .assert()
        .success();

    assert!(output.exists());
}

#[test]
fn test_create_force_short_flag() {
    let dir = tempfile::tempdir().unwrap();
    let (file1, _, _) = create_test_files(&dir);
    let output = dir.path().join("archive.sfa");

    // Create initial archive
    cargo_bin_cmd!()
        .arg("create")
        .arg(path_to_arg(&output))
        .arg(path_to_arg(&file1))
        .assert()
        .success();

    // Overwrite with short force flag
    cargo_bin_cmd!()
        .arg("create")
        .arg("-f")
        .arg(path_to_arg(&output))
        .arg(&file1)
        .assert()
        .success();
}

#[test]
fn test_create_without_force_fails_if_exists() {
    let dir = tempfile::tempdir().unwrap();
    let (file1, _, _) = create_test_files(&dir);
    let output = dir.path().join("archive.sfa");

    // Create initial archive
    cargo_bin_cmd!()
        .arg("create")
        .arg(path_to_arg(&output))
        .arg(path_to_arg(&file1))
        .assert()
        .success();

    // Try to create again without force - should fail
    cargo_bin_cmd!()
        .arg("create")
        .arg(path_to_arg(&output))
        .arg(path_to_arg(&file1))
        .assert()
        .failure()
        .stderr(predicate::str::contains("already exists"))
        .stderr(predicate::str::contains("Use --force to overwrite"));
}

#[test]
fn test_create_missing_input_file() {
    let dir = tempfile::tempdir().unwrap();
    let output = dir.path().join("archive.sfa");
    let missing_file = dir.path().join("nonexistent.txt");

    cargo_bin_cmd!()
        .arg("create")
        .arg(path_to_arg(&output))
        .arg(path_to_arg(&missing_file))
        .assert()
        .failure()
        .stderr(predicate::str::contains("Error reading file"));
}

#[test]
fn test_create_no_input_files() {
    let dir = tempfile::tempdir().unwrap();
    let output = dir.path().join("archive.sfa");

    cargo_bin_cmd!()
        .arg("create")
        .arg(path_to_arg(&output))
        .assert()
        .failure();
}

#[test]
fn test_create_binary_content() {
    let dir = tempfile::tempdir().unwrap();
    let binary_file = dir.path().join("binary.bin");
    fs::write(&binary_file, b"\x00\x01\x02\x03\xff\xfe\xfd\xfc").unwrap();
    let output = dir.path().join("archive.sfa");

    cargo_bin_cmd!()
        .arg("create")
        .arg(path_to_arg(&output))
        .arg(path_to_arg(&binary_file))
        .assert()
        .success();

    assert!(output.exists());
}

// ============================================================================
// CAT COMMAND TESTS
// ============================================================================

#[test]
fn test_cat_all_sections() {
    let dir = tempfile::tempdir().unwrap();
    let archive = create_test_archive(&dir);

    let output = cargo_bin_cmd!()
        .arg("cat")
        .arg(path_to_arg(&archive))
        .arg("*")
        .assert()
        .success();

    // Check raw bytes since output includes binary content from file3.dat
    let stdout = &output.get_output().stdout;
    assert!(stdout.windows(b"Hello, world!".len()).any(|w| w == b"Hello, world!"));
    assert!(stdout.windows(b"Test content".len()).any(|w| w == b"Test content"));
    assert!(stdout.windows(b"Line 2".len()).any(|w| w == b"Line 2"));
}

#[test]
fn test_cat_single_section() {
    let dir = tempfile::tempdir().unwrap();
    let archive = create_test_archive(&dir);

    cargo_bin_cmd!()
        .arg("cat")
        .arg(path_to_arg(&archive))
        .arg("file1.txt")
        .assert()
        .success()
        .stdout(predicate::eq("Hello, world!\n"));
}

#[test]
fn test_cat_with_section_flag() {
    let dir = tempfile::tempdir().unwrap();
    let archive = create_test_archive(&dir);

    cargo_bin_cmd!()
        .arg("cat")
        .arg("--section")
        .arg("file2.txt")
        .arg(path_to_arg(&archive))
        .assert()
        .success()
        .stdout(predicate::eq("Test content\nLine 2\n"));
}

#[test]
fn test_cat_with_section_short_flag() {
    let dir = tempfile::tempdir().unwrap();
    let archive = create_test_archive(&dir);

    cargo_bin_cmd!()
        .arg("cat")
        .arg("-s")
        .arg("file1.txt")
        .arg(path_to_arg(&archive))
        .assert()
        .success()
        .stdout(predicate::eq("Hello, world!\n"));
}

#[test]
fn test_cat_glob_pattern() {
    let dir = tempfile::tempdir().unwrap();
    let archive = create_test_archive(&dir);

    cargo_bin_cmd!()
        .arg("cat")
        .arg(path_to_arg(&archive))
        .arg("file*.txt")
        .assert()
        .success()
        .stdout(predicate::str::contains("Hello, world!"))
        .stdout(predicate::str::contains("Test content"));
}

#[test]
fn test_cat_multiple_patterns() {
    let dir = tempfile::tempdir().unwrap();
    let archive = create_test_archive(&dir);

    cargo_bin_cmd!()
        .arg("cat")
        .arg(path_to_arg(&archive))
        .arg("file1.txt")
        .arg("file2.txt")
        .assert()
        .success()
        .stdout(predicate::str::contains("Hello, world!"))
        .stdout(predicate::str::contains("Test content"));
}

#[test]
fn test_cat_combined_flag_and_positional() {
    let dir = tempfile::tempdir().unwrap();
    let archive = create_test_archive(&dir);

    cargo_bin_cmd!()
        .arg("cat")
        .arg("-s")
        .arg("file1.txt")
        .arg(path_to_arg(&archive))
        .arg("file2.txt")
        .assert()
        .success()
        .stdout(predicate::str::contains("Hello, world!"))
        .stdout(predicate::str::contains("Test content"));
}

#[test]
fn test_cat_no_pattern_fails() {
    let dir = tempfile::tempdir().unwrap();
    let archive = create_test_archive(&dir);

    cargo_bin_cmd!()
        .arg("cat")
        .arg(path_to_arg(&archive))
        .assert()
        .failure()
        .stderr(predicate::str::contains("No section pattern specified"));
}

#[test]
fn test_cat_nonexistent_file() {
    let dir = tempfile::tempdir().unwrap();
    let nonexistent = dir.path().join("nonexistent.sfa");

    cargo_bin_cmd!()
        .arg("cat")
        .arg(path_to_arg(&nonexistent))
        .arg("*")
        .assert()
        .failure()
        .stderr(predicate::str::contains("Error opening SFA file"));
}

#[test]
fn test_cat_invalid_file() {
    let dir = tempfile::tempdir().unwrap();
    let invalid_file = dir.path().join("invalid.sfa");
    fs::write(&invalid_file, b"not a valid sfa file").unwrap();

    cargo_bin_cmd!()
        .arg("cat")
        .arg(path_to_arg(&invalid_file))
        .arg("*")
        .assert()
        .failure()
        .stderr(predicate::str::contains("Error opening SFA file"));
}

#[test]
fn test_cat_invalid_glob_pattern() {
    let dir = tempfile::tempdir().unwrap();
    let archive = create_test_archive(&dir);

    cargo_bin_cmd!()
        .arg("cat")
        .arg(path_to_arg(&archive))
        .arg("[invalid")
        .assert()
        .failure()
        .stderr(predicate::str::contains("Error parsing glob pattern"));
}

#[test]
fn test_cat_pattern_no_match() {
    let dir = tempfile::tempdir().unwrap();
    let archive = create_test_archive(&dir);

    cargo_bin_cmd!()
        .arg("cat")
        .arg(path_to_arg(&archive))
        .arg("nonexistent*")
        .assert()
        .success()
        .stdout(predicate::str::is_empty());
}

#[test]
fn test_cat_binary_content() {
    let dir = tempfile::tempdir().unwrap();
    let archive = create_test_archive(&dir);

    let output = cargo_bin_cmd!()
        .arg("cat")
        .arg(path_to_arg(&archive))
        .arg("file3.dat")
        .assert()
        .success();

    // Verify binary content was output correctly
    let stdout = output.get_output().stdout.clone();
    assert_eq!(stdout, b"\x00\x01\x02\x03\xff\xfe\xfd");
}

#[test]
fn test_cat_empty_archive() {
    let dir = tempfile::tempdir().unwrap();
    let archive = dir.path().join("empty.sfa");
    let empty_file = dir.path().join("empty.txt");
    fs::write(&empty_file, b"").unwrap();

    cargo_bin_cmd!()
        .arg("create")
        .arg(path_to_arg(&archive))
        .arg(path_to_arg(&empty_file))
        .assert()
        .success();

    cargo_bin_cmd!()
        .arg("cat")
        .arg(path_to_arg(&archive))
        .arg("*")
        .assert()
        .success()
        .stderr(predicate::str::contains("SFA file contains no sections"));
}

// ============================================================================
// DUMP COMMAND TESTS
// ============================================================================

fn create_test_archive(dir: &TempDir) -> std::path::PathBuf {
    let (file1, file2, file3) = create_test_files(dir);
    let output = dir.path().join("archive.sfa");

    cargo_bin_cmd!()
        .arg("create")
        .arg(path_to_arg(&output))
        .arg(path_to_arg(&file1))
        .arg(path_to_arg(&file2))
        .arg(path_to_arg(&file3))
        .assert()
        .success();

    output
}

fn setup_extract_test(dir: &TempDir, _archive: &std::path::Path) {
    // Remove original files so extract can create them
    fs::remove_file(dir.path().join("file1.txt")).ok();
    fs::remove_file(dir.path().join("file2.txt")).ok();
    fs::remove_file(dir.path().join("file3.dat")).ok();
}

#[test]
fn test_dump_basic() {
    let dir = tempfile::tempdir().unwrap();
    let archive = create_test_archive(&dir);

    cargo_bin_cmd!()
        .arg("dump")
        .arg(path_to_arg(&archive))
        .assert()
        .success()
        .stdout(predicate::str::contains("SFA file:"))
        .stdout(predicate::str::contains("Number of sections: 3"))
        .stdout(predicate::str::contains("Section 0:"))
        .stdout(predicate::str::contains("Section 1:"))
        .stdout(predicate::str::contains("Section 2:"))
        .stdout(predicate::str::contains("file1.txt"))
        .stdout(predicate::str::contains("file2.txt"))
        .stdout(predicate::str::contains("file3.dat"))
        .stdout(predicate::str::contains("Dumped 3 sections"));
}

#[test]
fn test_dump_alias_d() {
    let dir = tempfile::tempdir().unwrap();
    let archive = create_test_archive(&dir);

    cargo_bin_cmd!()
        .arg("d")
        .arg(path_to_arg(&archive))
        .assert()
        .success()
        .stdout(predicate::str::contains("Number of sections: 3"));
}

#[test]
fn test_dump_alias_t() {
    let dir = tempfile::tempdir().unwrap();
    let archive = create_test_archive(&dir);

    cargo_bin_cmd!()
        .arg("t")
        .arg(path_to_arg(&archive))
        .assert()
        .success()
        .stdout(predicate::str::contains("Number of sections: 3"));
}

#[test]
fn test_dump_alias_test() {
    let dir = tempfile::tempdir().unwrap();
    let archive = create_test_archive(&dir);

    cargo_bin_cmd!()
        .arg("test")
        .arg(path_to_arg(&archive))
        .assert()
        .success()
        .stdout(predicate::str::contains("Number of sections: 3"));
}

#[test]
fn test_dump_with_content() {
    let dir = tempfile::tempdir().unwrap();
    let archive = create_test_archive(&dir);

    cargo_bin_cmd!()
        .arg("dump")
        .arg("--content")
        .arg(path_to_arg(&archive))
        .assert()
        .success()
        .stdout(predicate::str::contains("Content:"))
        .stdout(predicate::str::contains("Hello, world!"));
}

#[test]
fn test_dump_with_content_short_flag() {
    let dir = tempfile::tempdir().unwrap();
    let archive = create_test_archive(&dir);

    cargo_bin_cmd!()
        .arg("dump")
        .arg("-C")
        .arg(path_to_arg(&archive))
        .assert()
        .success()
        .stdout(predicate::str::contains("Content:"));
}

#[test]
fn test_dump_with_section_pattern() {
    let dir = tempfile::tempdir().unwrap();
    let archive = create_test_archive(&dir);

    cargo_bin_cmd!()
        .arg("dump")
        .arg("--section")
        .arg("file1.txt")
        .arg(path_to_arg(&archive))
        .assert()
        .success()
        .stdout(predicate::str::contains("Section 0:"))
        .stdout(predicate::str::contains("file1.txt"))
        .stdout(predicate::str::contains("Dumped 1 of 3 sections"));
}

#[test]
fn test_dump_with_section_pattern_short_flag() {
    let dir = tempfile::tempdir().unwrap();
    let archive = create_test_archive(&dir);

    cargo_bin_cmd!()
        .arg("dump")
        .arg("-s")
        .arg("file2.txt")
        .arg(path_to_arg(&archive))
        .assert()
        .success()
        .stdout(predicate::str::contains("Dumped 1 of 3 sections"));
}

#[test]
fn test_dump_with_section_glob_pattern() {
    let dir = tempfile::tempdir().unwrap();
    let archive = create_test_archive(&dir);

    cargo_bin_cmd!()
        .arg("dump")
        .arg("--section")
        .arg("file*.txt")
        .arg(path_to_arg(&archive))
        .assert()
        .success()
        .stdout(predicate::str::contains("Dumped 2 of 3 sections"));
}

#[test]
fn test_dump_with_section_pattern_no_match() {
    let dir = tempfile::tempdir().unwrap();
    let archive = create_test_archive(&dir);

    cargo_bin_cmd!()
        .arg("dump")
        .arg("--section")
        .arg("nonexistent*")
        .arg(path_to_arg(&archive))
        .assert()
        .success()
        .stdout(predicate::str::contains("Dumped 0 of 3 sections"));
}

#[test]
fn test_dump_with_content_and_section() {
    let dir = tempfile::tempdir().unwrap();
    let archive = create_test_archive(&dir);

    cargo_bin_cmd!()
        .arg("dump")
        .arg("--content")
        .arg("--section")
        .arg("file1.txt")
        .arg(path_to_arg(&archive))
        .assert()
        .success()
        .stdout(predicate::str::contains("Content:"))
        .stdout(predicate::str::contains("Hello, world!"))
        .stdout(predicate::str::contains("Dumped 1 of 3 sections"));
}

#[test]
fn test_dump_empty_archive() {
    let dir = tempfile::tempdir().unwrap();
    let archive = dir.path().join("empty.sfa");

    // Create an empty archive (this would require using the library directly,
    // but for now we'll test with a non-existent file which should error)
    // Actually, let's create a minimal valid archive
    let file1 = dir.path().join("file1.txt");
    fs::write(&file1, b"").unwrap();

    cargo_bin_cmd!()
        .arg("create")
        .arg(path_to_arg(&archive))
        .arg(&file1)
        .assert()
        .success();

    cargo_bin_cmd!()
        .arg("dump")
        .arg(path_to_arg(&archive))
        .assert()
        .success()
        .stderr(predicate::str::contains("SFA file contains no sections"));
}

#[test]
fn test_dump_invalid_file() {
    let dir = tempfile::tempdir().unwrap();
    let invalid_file = dir.path().join("invalid.sfa");
    fs::write(&invalid_file, b"not a valid sfa file").unwrap();

    cargo_bin_cmd!()
        .arg("dump")
        .arg(path_to_arg(&invalid_file))
        .assert()
        .failure()
        .stderr(predicate::str::contains("Error opening SFA file"));
}

#[test]
fn test_dump_nonexistent_file() {
    let dir = tempfile::tempdir().unwrap();
    let nonexistent = dir.path().join("nonexistent.sfa");

    cargo_bin_cmd!()
        .arg("dump")
        .arg(path_to_arg(&nonexistent))
        .assert()
        .failure()
        .stderr(predicate::str::contains("Error opening SFA file"));
}

#[test]
fn test_dump_invalid_glob_pattern() {
    let dir = tempfile::tempdir().unwrap();
    let archive = create_test_archive(&dir);

    cargo_bin_cmd!()
        .arg("dump")
        .arg("--section")
        .arg("[invalid")
        .arg(path_to_arg(&archive))
        .assert()
        .failure()
        .stderr(predicate::str::contains("Error parsing glob pattern"));
}

// ============================================================================
// EXTRACT COMMAND TESTS
// ============================================================================

#[test]
fn test_extract_basic() {
    let dir = tempfile::tempdir().unwrap();
    let archive = create_test_archive(&dir);

    setup_extract_test(&dir, &archive);

    cargo_bin_cmd!()
        .current_dir(dir.path())
        .arg("extract")
        .arg(path_to_arg(&archive))
        .assert()
        .success()
        .stdout(predicate::str::contains("Extracted: file1.txt"))
        .stdout(predicate::str::contains("Extracted: file2.txt"))
        .stdout(predicate::str::contains("Extracted: file3.dat"))
        .stdout(predicate::str::contains("Extracted 3 sections"));

    // Verify extracted files
    let file1_content = fs::read(dir.path().join("file1.txt")).unwrap();
    assert_eq!(file1_content, b"Hello, world!\n");

    let file2_content = fs::read(dir.path().join("file2.txt")).unwrap();
    assert_eq!(file2_content, b"Test content\nLine 2\n");

    let file3_content = fs::read(dir.path().join("file3.dat")).unwrap();
    assert_eq!(file3_content, b"\x00\x01\x02\x03\xff\xfe\xfd");
}

#[test]
fn test_extract_alias_x() {
    let dir = tempfile::tempdir().unwrap();
    let archive = create_test_archive(&dir);
    setup_extract_test(&dir, &archive);

    cargo_bin_cmd!()
        .current_dir(dir.path())
        .arg("x")
        .arg(path_to_arg(&archive))
        .assert()
        .success()
        .stdout(predicate::str::contains("Extracted 3 sections"));

    assert!(dir.path().join("file1.txt").exists());
}

#[test]
fn test_extract_single_section() {
    let dir = tempfile::tempdir().unwrap();
    let archive = create_test_archive(&dir);
    setup_extract_test(&dir, &archive);

    cargo_bin_cmd!()
        .current_dir(dir.path())
        .arg("extract")
        .arg("--section")
        .arg("file1.txt")
        .arg(path_to_arg(&archive))
        .assert()
        .success()
        .stdout(predicate::str::contains("Extracted: file1.txt"))
        .stdout(predicate::str::contains("Extracted 1 of 3 sections"));

    assert!(dir.path().join("file1.txt").exists());
    assert!(!dir.path().join("file2.txt").exists());
    assert!(!dir.path().join("file3.dat").exists());
}

#[test]
fn test_extract_section_short_flag() {
    let dir = tempfile::tempdir().unwrap();
    let archive = create_test_archive(&dir);
    setup_extract_test(&dir, &archive);

    cargo_bin_cmd!()
        .current_dir(dir.path())
        .arg("extract")
        .arg("-s")
        .arg("file2.txt")
        .arg(path_to_arg(&archive))
        .assert()
        .success()
        .stdout(predicate::str::contains("Extracted 1 of 3 sections"));

    assert!(dir.path().join("file2.txt").exists());
}

#[test]
fn test_extract_section_glob_pattern() {
    let dir = tempfile::tempdir().unwrap();
    let archive = create_test_archive(&dir);
    setup_extract_test(&dir, &archive);

    cargo_bin_cmd!()
        .current_dir(dir.path())
        .arg("extract")
        .arg("--section")
        .arg("file*.txt")
        .arg(path_to_arg(&archive))
        .assert()
        .success()
        .stdout(predicate::str::contains("Extracted 2 of 3 sections"));

    assert!(dir.path().join("file1.txt").exists());
    assert!(dir.path().join("file2.txt").exists());
    assert!(!dir.path().join("file3.dat").exists());
}

#[test]
fn test_extract_with_force() {
    let dir = tempfile::tempdir().unwrap();
    let archive = create_test_archive(&dir);
    setup_extract_test(&dir, &archive);

    // Extract first time
    cargo_bin_cmd!()
        .current_dir(dir.path())
        .arg("extract")
        .arg(path_to_arg(&archive))
        .assert()
        .success();

    // Extract again with force
    cargo_bin_cmd!()
        .current_dir(dir.path())
        .arg("extract")
        .arg("--force")
        .arg(path_to_arg(&archive))
        .assert()
        .success()
        .stdout(predicate::str::contains("Extracted 3 sections"));
}

#[test]
fn test_extract_with_force_short_flag() {
    let dir = tempfile::tempdir().unwrap();
    let archive = create_test_archive(&dir);
    setup_extract_test(&dir, &archive);

    // Extract first time
    cargo_bin_cmd!()
        .current_dir(dir.path())
        .arg("extract")
        .arg(path_to_arg(&archive))
        .assert()
        .success();

    // Extract again with short force flag
    cargo_bin_cmd!()
        .current_dir(dir.path())
        .arg("extract")
        .arg("-f")
        .arg(path_to_arg(&archive))
        .assert()
        .success();
}

#[test]
fn test_extract_without_force_fails_if_exists() {
    let dir = tempfile::tempdir().unwrap();
    let archive = create_test_archive(&dir);
    setup_extract_test(&dir, &archive);

    // Extract first time
    cargo_bin_cmd!()
        .current_dir(dir.path())
        .arg("extract")
        .arg(path_to_arg(&archive))
        .assert()
        .success();

    // Try to extract again without force - should fail
    cargo_bin_cmd!()
        .current_dir(dir.path())
        .arg("extract")
        .arg(path_to_arg(&archive))
        .assert()
        .failure()
        .stderr(predicate::str::contains("already exists"))
        .stderr(predicate::str::contains("Use --force to overwrite"));
}

#[test]
fn test_extract_with_block_size() {
    let dir = tempfile::tempdir().unwrap();
    let archive = create_test_archive(&dir);
    setup_extract_test(&dir, &archive);
    cargo_bin_cmd!()
        .current_dir(dir.path())
        .arg("extract")
        .arg("--block-size")
        .arg("32KB")
        .arg(path_to_arg(&archive))
        .assert()
        .success()
        .stdout(predicate::str::contains("Extracted 3 sections"));

    assert!(dir.path().join("file1.txt").exists());
}

#[test]
fn test_extract_with_block_size_short_flag() {
    let dir = tempfile::tempdir().unwrap();
    let archive = create_test_archive(&dir);
    setup_extract_test(&dir, &archive);
    cargo_bin_cmd!()
        .current_dir(dir.path())
        .arg("extract")
        .arg("-b")
        .arg("16KB")
        .arg(path_to_arg(&archive))
        .assert()
        .success();

    assert!(dir.path().join("file1.txt").exists());
}

#[test]
fn test_extract_with_different_block_sizes() {
    for block_size in ["4B", "1KB", "4KB", "64KB", "1MB"] {
        let test_dir = tempfile::tempdir().unwrap();
        let test_archive = create_test_archive(&test_dir);
        setup_extract_test(&test_dir, &test_archive);

        cargo_bin_cmd!()
            .current_dir(test_dir.path())
            .arg("extract")
            .arg("--block-size")
            .arg(block_size)
            .arg(path_to_arg(&test_archive))
            .assert()
            .success();

        let file1_content = fs::read(test_dir.path().join("file1.txt")).unwrap();
        assert_eq!(file1_content, b"Hello, world!\n");
    }
}

#[test]
fn test_extract_with_all_flags() {
    let dir = tempfile::tempdir().unwrap();
    let archive = create_test_archive(&dir);
    setup_extract_test(&dir, &archive);

    cargo_bin_cmd!()
        .current_dir(dir.path())
        .arg("extract")
        .arg("--force")
        .arg("--block-size")
        .arg("32KB")
        .arg("--section")
        .arg("file1.txt")
        .arg(path_to_arg(&archive))
        .assert()
        .success()
        .stdout(predicate::str::contains("Extracted 1 of 3 sections"));
}

#[test]
fn test_extract_nonexistent_file() {
    let dir = tempfile::tempdir().unwrap();
    let nonexistent = dir.path().join("nonexistent.sfa");

    cargo_bin_cmd!()
        .arg("extract")
        .arg(path_to_arg(&nonexistent))
        .assert()
        .failure()
        .stderr(predicate::str::contains("Error opening SFA file"));
}

#[test]
fn test_extract_invalid_file() {
    let dir = tempfile::tempdir().unwrap();
    let invalid_file = dir.path().join("invalid.sfa");
    fs::write(&invalid_file, b"not a valid sfa file").unwrap();

    cargo_bin_cmd!()
        .arg("extract")
        .arg(path_to_arg(&invalid_file))
        .assert()
        .failure()
        .stderr(predicate::str::contains("Error opening SFA file"));
}

#[test]
fn test_extract_empty_archive() {
    let dir = tempfile::tempdir().unwrap();
    let archive = dir.path().join("empty.sfa");
    let empty_file = dir.path().join("empty.txt");
    fs::write(&empty_file, b"").unwrap();

    cargo_bin_cmd!()
        .arg("create")
        .arg(path_to_arg(&archive))
        .arg(path_to_arg(&empty_file))
        .assert()
        .success();

    cargo_bin_cmd!()
        .arg("extract")
        .arg(path_to_arg(&archive))
        .assert()
        .success()
        .stderr(predicate::str::contains("SFA file contains no sections"));
}

#[test]
fn test_extract_invalid_glob_pattern() {
    let dir = tempfile::tempdir().unwrap();
    let archive = create_test_archive(&dir);

    cargo_bin_cmd!()
        .arg("extract")
        .arg("--section")
        .arg("[invalid")
        .arg(path_to_arg(&archive))
        .assert()
        .failure()
        .stderr(predicate::str::contains("Error parsing glob pattern"));
}

#[test]
fn test_extract_section_pattern_no_match() {
    let dir = tempfile::tempdir().unwrap();
    let archive = create_test_archive(&dir);

    cargo_bin_cmd!()
        .arg("extract")
        .arg("--section")
        .arg("nonexistent*")
        .arg(path_to_arg(&archive))
        .assert()
        .success()
        .stdout(predicate::str::contains("Extracted 0 of 3 sections"));
}

// ============================================================================
// ROUND-TRIP TESTS
// ============================================================================

#[test]
fn test_create_and_extract_round_trip() {
    let dir = tempfile::tempdir().unwrap();
    let (file1, file2, file3) = create_test_files(&dir);
    let archive = dir.path().join("archive.sfa");

    // Create archive
    cargo_bin_cmd!()
        .arg("create")
        .arg(path_to_arg(&archive))
        .arg(&file1)
        .arg(path_to_arg(&file2))
        .arg(path_to_arg(&file3))
        .assert()
        .success();

    // Extract to different location
    let extract_dir = tempfile::tempdir().unwrap();

    cargo_bin_cmd!()
        .current_dir(extract_dir.path())
        .arg("extract")
        .arg(path_to_arg(&archive))
        .assert()
        .success();

    // Verify contents
    let extracted1 = fs::read(extract_dir.path().join("file1.txt")).unwrap();
    let extracted2 = fs::read(extract_dir.path().join("file2.txt")).unwrap();
    let extracted3 = fs::read(extract_dir.path().join("file3.dat")).unwrap();

    assert_eq!(extracted1, fs::read(&file1).unwrap());
    assert_eq!(extracted2, fs::read(&file2).unwrap());
    assert_eq!(extracted3, fs::read(&file3).unwrap());
}

#[test]
fn test_create_dump_extract_workflow() {
    let dir = tempfile::tempdir().unwrap();
    let (file1, file2, _) = create_test_files(&dir);
    let archive = dir.path().join("archive.sfa");

    // Create
    cargo_bin_cmd!()
        .arg("create")
        .arg(path_to_arg(&archive))
        .arg(&file1)
        .arg(path_to_arg(&file2))
        .assert()
        .success();

    // Dump to verify
    cargo_bin_cmd!()
        .arg("dump")
        .arg(path_to_arg(&archive))
        .assert()
        .success()
        .stdout(predicate::str::contains("Number of sections: 2"));

    // Extract
    let extract_dir = tempfile::tempdir().unwrap();

    cargo_bin_cmd!()
        .current_dir(extract_dir.path())
        .arg("extract")
        .arg(path_to_arg(&archive))
        .assert()
        .success();

    assert!(extract_dir.path().join("file1.txt").exists());
    assert!(extract_dir.path().join("file2.txt").exists());
}

// ============================================================================
// EDGE CASES AND ERROR HANDLING
// ============================================================================

#[test]
fn test_create_with_empty_file() {
    let dir = tempfile::tempdir().unwrap();
    let empty_file = dir.path().join("empty.txt");
    fs::write(&empty_file, b"").unwrap();
    let output = dir.path().join("archive.sfa");

    cargo_bin_cmd!()
        .arg("create")
        .arg(path_to_arg(&output))
        .arg(path_to_arg(&empty_file))
        .assert()
        .success();

    // Verify we can dump it (empty sections are not stored)
    cargo_bin_cmd!()
        .arg("dump")
        .arg(path_to_arg(&output))
        .assert()
        .success()
        .stderr(predicate::str::contains("SFA file contains no sections"));
}

#[test]
fn test_create_with_large_file() {
    let dir = tempfile::tempdir().unwrap();
    let large_file = dir.path().join("large.txt");
    let content = vec![b'A'; 100000]; // 100KB
    fs::write(&large_file, &content).unwrap();
    let output = dir.path().join("archive.sfa");

    cargo_bin_cmd!()
        .arg("create")
        .arg(path_to_arg(&output))
        .arg(path_to_arg(&large_file))
        .assert()
        .success();

    // Remove original file
    fs::remove_file(&large_file).ok();

    // Extract and verify
    cargo_bin_cmd!()
        .current_dir(dir.path())
        .arg("extract")
        .arg(path_to_arg(&output))
        .assert()
        .success();

    let extracted = fs::read(dir.path().join("large.txt")).unwrap();
    assert_eq!(extracted.len(), 100000);
    assert_eq!(extracted, content);
}

#[test]
fn test_extract_with_special_characters_in_filename() {
    let dir = tempfile::tempdir().unwrap();
    let special_file = dir.path().join("file with spaces.txt");
    fs::write(&special_file, b"content with spaces").unwrap();
    let output = dir.path().join("archive.sfa");

    cargo_bin_cmd!()
        .arg("create")
        .arg(path_to_arg(&output))
        .arg(path_to_arg(&special_file))
        .assert()
        .success();

    // Remove original file
    fs::remove_file(&special_file).ok();

    cargo_bin_cmd!()
        .current_dir(dir.path())
        .arg("extract")
        .arg(path_to_arg(&output))
        .assert()
        .success();

    let extracted = fs::read(dir.path().join("file with spaces.txt")).unwrap();
    assert_eq!(extracted, b"content with spaces");
}

#[test]
fn test_no_arguments_shows_help() {
    cargo_bin_cmd!()
        .assert()
        .failure()
        .stderr(predicate::str::contains("Usage:"));
}

#[test]
fn test_invalid_command() {
    cargo_bin_cmd!().arg("invalid").assert().failure();
}

#[test]
fn test_command_without_required_args() {
    cargo_bin_cmd!().arg("create").assert().failure();

    cargo_bin_cmd!().arg("dump").assert().failure();

    cargo_bin_cmd!().arg("extract").assert().failure();
}

#[test]
fn test_extract_path_traversal_attack() {
    let dir = tempfile::tempdir().unwrap();
    let archive = dir.path().join("malicious.sfa");

    // Manually create an SFA file with a path traversal attack section name
    let mut file = std::fs::File::create(&archive).unwrap();
    let mut writer = Writer::from_writer(&mut file);
    writer.start("../../etc/shadow.attack.test").unwrap();
    writer.write_all(b"malicious content").unwrap();
    writer.finish().unwrap();
    file.sync_all().unwrap();
    drop(file);

    // Try to extract the archive - it should refuse to create the file
    // due to path traversal protection
    cargo_bin_cmd!()
        .current_dir(dir.path())
        .arg("extract")
        .arg(path_to_arg(&archive))
        .assert()
        .failure()
        .stderr(predicate::str::contains("Error jailing path"));

    // Verify that the malicious file was not created in the extraction directory
    // (path_jail should prevent any file creation outside the jail)
    assert!(!dir.path().join("shadow.attack.test").exists());
}
