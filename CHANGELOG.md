# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [0.4.0] - 2026-09-09

### Added
- k9s-style help view in main panel.
- Bracketed paste handling and row TSV copy.
- Persist MRU tables, SQL history, and page size.
- Live filter, jump-to-row, and quick reconnect.
- `--connection` and `--version` CLI flags.

### Changed
- k9s-style UI theme pass.
- k9s-style stacked connection info in top bar.
- Demo improvements and clippy lint fixes.

## [0.3.0] - 2026-06-06

### Added
- Table CRUD operations (insert, update, delete rows).
- Edit table cell values in place.
- Recently opened tables section in the top bar.
- Virtual table viewer for large row values.
- Better parsing for user-defined PostgreSQL enum types.

### Changed
- Improved table navigation to show more columns at once.
- SQL executor and editor improvements.
- Simplified internal rendering and data-loading code.

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

