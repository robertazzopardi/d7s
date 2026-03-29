# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [0.2.0] - 2026-03-29

### Added
- Copy table cell values to clipboard.
- Clear the status line on key press.

### Changed
- Consolidated from a multi-crate workspace (`d7s_auth`, `d7s_db`, `d7s_ui`) into a single crate for simpler publishing. The supporting crates published to crates.io under v0.1.0 (`d7s_auth`, `d7s_db`, `d7s_ui`) have been removed and will be yanked from crates.io.
- Simplified widget render methods.
- Moved connections and SQL executor into `DatabaseExplorer`.

### Fixed
- Fixed top bar height rendering.
- Fixed no connections displayed when exiting a database.
- Fixed column selection blur on first column when returning to row selection.

## [0.1.0] - 2026-02-03

### Added

- TUI database client for PostgreSQL and SQLite.
- Save and manage named database connections.
- Keyring support for the major platforms mac, linux and windows (not fully tested). 
- Browse databases, schemas, tables, columns, and table data with keyboard navigation.
- Execute SQL queries and view the results.
- Connection environment management (dev, staging, prod).
- Nix flake support.

