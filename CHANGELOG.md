# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- A help command (`kern -h` or `kern --help`) and a version command (`kern -V` or `kern --version`) were introduced.
- An update command (`kern update`) was introduced to enhance user experience and reduce the effort regarding maintenance. It also features a `--check` flag to just check for updates without installing them.

### Changed
- The command line interface (CLI) now uses `clap` as its argument parser.

### Fixed
- Opening a file that exists but cannot be read as text (not UTF-8, a directory, or no permission) now prints an error and exits instead of opening an empty buffer, which a save would have written over the file. A path that does not exist still opens as a new, empty file.

## [0.1.1] - 2026-10-24

### Changed
- When opening a document, the cursor now defaults to the end of the document.

## [0.1.0] - 2026-10-22

### Added
- Initial release

---

## Changelog Categories

- **Added** — New features or functionality.
- **Changed** — Changes to existing functionality.
- **Deprecated** — Features that will be removed in a future release.
- **Removed** — Removed features or functionality.
- **Fixed** — Bug fixes.
- **Security** — Security-related improvements or fixes.

## Versioning

This project follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html):

- **MAJOR** — Incompatible API changes.
- **MINOR** — New functionality that remains backward compatible.
- **PATCH** — Backward-compatible bug fixes.

Changes under `Unreleased` will be assigned to a version when a new release is published.