// Conserve backup system.
// Copyright 2016, 2017, 2018 Martin Pool.

//! Run conserve CLI as a subprocess and test it.

extern crate assert_cmd;
extern crate assert_fs;
extern crate predicates;
extern crate tempfile;

use std::fs;
use std::io::prelude::*;
use std::process::Command;

use assert_cmd::prelude::*;
use assert_fs::prelude::*;
use assert_fs::TempDir;
use predicates::prelude::*;

use predicate::path::{is_dir, is_file};
use predicate::str::{contains, is_empty, is_match, starts_with};

extern crate conserve;
use conserve::test_fixtures::{ScratchArchive, TreeFixture};

#[test]
fn blackbox_no_args() {
    // Run with no arguments, should fail with a usage message to stderr.
    main_binary()
        .assert()
        .failure()
        .stdout(is_empty())
        .stderr(contains("USAGE:"));
}

#[test]
fn blackbox_help() {
    main_binary()
        .arg("--help")
        .assert()
        .success()
        .stdout(contains("A robust backup tool"))
        .stdout(contains("Copy source directory into an archive"))
        .stderr(is_empty());
}

#[test]
fn clean_error_on_non_archive() {
    // Try to backup into a directory that is not an archive.
    let testdir = TempDir::new().unwrap();
    // TODO: Errors really should go to stderr not stdout.
    main_binary()
        .arg("backup")
        .arg(testdir.path())
        .arg(".")
        .assert()
        .failure()
        .stdout(contains("Not a Conserve archive"));
}

#[test]
fn blackbox_backup() {
    let testdir = TempDir::new().unwrap();
    let arch_dir = testdir.path().join("a");

    // conserve init
    main_binary()
        .arg("init")
        .arg(&arch_dir)
        .assert()
        .success()
        .stderr(is_empty())
        .stdout(starts_with("Created new archive"));

    // New archive contains no versions.
    main_binary()
        .arg("versions")
        .arg(&arch_dir)
        .assert()
        .success()
        .stderr(is_empty())
        .stdout(is_empty());

    let src = TreeFixture::new();
    src.create_file("hello");
    src.create_dir("subdir");

    main_binary()
        .args(&["source", "ls"])
        .arg(src.path())
        .assert()
        .success()
        .stderr(is_empty())
        .stdout(
            "/\n\
             /hello\n\
             /subdir\n",
        );

    // backup
    main_binary()
        .arg("backup")
        .arg(&arch_dir)
        .arg(src.root)
        .assert()
        .success()
        .stderr(is_empty())
        .stdout(starts_with("Backup complete.\n"));
    // TODO: Now inspect the archive.

    // versions --short
    main_binary()
        .args(&["versions", "--short"])
        .arg(&arch_dir)
        .assert()
        .success()
        .stderr(is_empty())
        .stdout("b0000\n");

    main_binary()
        .args(&["debug", "block", "list"])
        .arg(&arch_dir)
        .assert()
        .success()
        .stderr(is_empty())
        .stdout(
            "9063990e5c5b2184877f92adace7c801a549b00c39cd7549877f06d5dd0d3\
             a6ca6eee42d5896bdac64831c8114c55cee664078bd105dc691270c92644ccb2ce7\n",
        );

    main_binary()
        .arg("versions")
        .arg(&arch_dir)
        .assert()
        .success()
        .stderr(is_empty())
        .stdout(is_match(r"^b0000 {21} complete   20[-0-9T:+]+\s +\d+s\n$").unwrap());
    // TODO: Set a fake date when creating the archive and then we can check
    // the format of the output?

    main_binary()
        .arg("ls")
        .arg(&arch_dir)
        .assert()
        .success()
        .stderr(is_empty())
        .stdout(
            "/\n\
             /hello\n\
             /subdir\n",
        );

    // TODO: Factor out comparison to expected tree.
    let restore_dir = TempDir::new().unwrap();

    main_binary()
        .arg("restore")
        .arg("-v")
        .arg(&arch_dir)
        .arg(restore_dir.path())
        .assert()
        .success()
        .stderr(is_empty())
        .stdout(starts_with(
            "/\n\
             /hello\n\
             /subdir\n\
             Restore complete.\n",
        ));

    restore_dir.child("subdir").assert(is_dir());
    restore_dir
        .child("hello")
        .assert(is_file())
        .assert("contents");

    // Try to restore again over the same directory: should decline.
    main_binary()
        .arg("restore")
        .arg("-v")
        .arg(&arch_dir)
        .arg(restore_dir.path())
        .assert()
        .failure()
        .stderr(is_empty())
        .stdout(contains("Destination directory not empty"));

    // Restore with specified band id / backup version.
    {
        let restore_dir2 = TempDir::new().unwrap();
        // Try to restore again over the same directory: should decline.
        main_binary()
            .args(&["restore", "-b", "b0"])
            .arg(&arch_dir)
            .arg(restore_dir2.path())
            .assert()
            .success();
        // TODO: Check tree contents, but they should be the same as above.
    }

    // Validate
    main_binary()
        .arg("validate")
        .arg(&arch_dir)
        .assert()
        .success()
        .stderr(is_empty())
        .stdout(starts_with("Archive is OK.\n"));

    // TODO: Compare vs source tree.
}

#[test]
fn empty_archive() {
    let adir = TempDir::new().unwrap();
    main_binary()
        .arg("init")
        .arg(adir.path())
        .assert()
        .success();

    let restore_dir = TempDir::new().unwrap();
    main_binary()
        .arg("restore")
        .arg(adir.path())
        .arg(restore_dir.path())
        .assert()
        .failure()
        .stdout(contains("Archive has no complete bands"));

    main_binary()
        .arg("ls")
        .arg(adir.path())
        .assert()
        .failure()
        .stdout(contains("Archive has no complete bands"));

    main_binary()
        .arg("versions")
        .arg(adir.path())
        .assert()
        .success()
        .stdout(is_empty());
}

/// Check behavior on an incomplete version.
///
/// Commands that read from the archive should by default decline, unless given
/// `--incomplete`.
#[test]
fn incomplete_version() {
    let af = ScratchArchive::new();
    af.setup_incomplete_empty_band();

    main_binary()
        .arg("versions")
        .arg(af.path())
        .assert()
        .success()
        .stderr(is_empty())
        .stdout(contains("b0000"))
        .stdout(contains("incomplete"));

    // ls fails on incomplete band
    main_binary()
        .arg("ls")
        .arg(af.path())
        .assert()
        .failure()
        .stderr(is_empty())
        .stdout(contains("Archive has no complete bands"));

    // ls --incomplete accurately says it has nothing
    main_binary()
        .args(&["ls", "-b", "b0", "--incomplete"])
        .arg(af.path())
        .assert()
        .success()
        .stderr(is_empty())
        .stdout(is_empty());
}

fn main_binary() -> std::process::Command {
    Command::main_binary().unwrap()
}
