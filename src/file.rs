use std::fs;
use std::io;
use std::path::PathBuf;

pub fn write_file(path: &PathBuf, content: &str) -> io::Result<()> {
    fs::write(path, content)
}

pub fn read_file(path: &PathBuf) -> std::io::Result<String> {
    std::fs::read_to_string(path)
}

/// Reads a file to open it in the editor.
///
/// A file that does not exist yet opens as an empty buffer, so `kern new.txt`
/// creates it on save. Every other error (not UTF-8, a directory, no permission)
/// is returned, because opening an empty buffer would let a save overwrite the file.
pub fn read_file_for_editing(path: &PathBuf) -> io::Result<String> {
    match read_file(path) {
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(String::new()),
        result => result,
    }
}
