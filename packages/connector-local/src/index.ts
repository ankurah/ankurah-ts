// MIRRORS: ankurah/connectors/local-process/src/lib.rs
//
// @ankurah/connector-local — Local in-process connector.
//
// Connects multiple Node instances within the same process for testing.
// Uses direct function calls / event emitters instead of Tokio channels
// (Exception E18: Tokio channels -> TS async patterns).
//
// Rust crate: ankurah-connector-local-process
// Key types: LocalConnector
//
// TODO: Port local connector from ankurah/connectors/local-process/src/
