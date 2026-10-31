mod common;

use common::temp_file_path;
use kern::file::{read_file, read_file_for_editing};
use std::fs;

#[test]
fn error_no_such_file() {
    let path = temp_file_path();
    let result = read_file(&path);

    assert!(result.is_err());
}

#[test]
fn reads_file_contents() {
    let path = temp_file_path();
    let expected = "Hello, kern!\nHello from the 2nd line!";

    fs::write(&path, expected).unwrap();

    let result = read_file(&path);

    assert_eq!(result.unwrap(), expected);

    fs::remove_file(&path).unwrap();
}

#[test]
fn missing_file_opens_as_empty_buffer() {
    let path = temp_file_path();

    let result = read_file_for_editing(&path);

    assert_eq!(result.unwrap(), "");
    assert!(!path.exists());
}

#[test]
fn opens_existing_file_for_editing() {
    let path = temp_file_path();
    fs::write(&path, "line one\nline two").unwrap();

    let result = read_file_for_editing(&path);

    assert_eq!(result.unwrap(), "line one\nline two");

    fs::remove_file(&path).unwrap();
}

#[test]
fn file_that_is_not_utf8_is_an_error_not_an_empty_buffer() {
    let path = temp_file_path();
    let bytes = [0xff, 0xfe, 0x00, b'k', b'e', b'r', b'n'];
    fs::write(&path, bytes).unwrap();

    let error = read_file_for_editing(&path).unwrap_err();

    assert_eq!(error.kind(), std::io::ErrorKind::InvalidData);
    assert_eq!(fs::read(&path).unwrap(), bytes);

    fs::remove_file(&path).unwrap();
}

#[test]
fn directory_is_an_error_not_an_empty_buffer() {
    let dir = std::env::temp_dir();

    let result = read_file_for_editing(&dir);

    assert!(result.is_err());
}
