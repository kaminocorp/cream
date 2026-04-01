//! # cream-audit
//!
//! Append-only write path and query interface for the immutable audit ledger.
//!
//! Every payment lifecycle event produces an [`AuditEntry`] that is persisted
//! through the [`AuditWriter`] trait. Entries are queryable via [`AuditReader`].
//! Both traits are backed by PostgreSQL implementations (`PgAuditWriter`,
//! `PgAuditReader`) but can be mocked for testing.
//!
//! The writer is intentionally insert-only — there is no update or delete
//! method at the Rust level, mirroring the database-level trigger enforcement.

pub mod error;
pub mod reader;
pub mod writer;

// Convenience re-exports
pub use error::AuditError;
pub use reader::{AuditQuery, AuditReader, PgAuditReader};
pub use writer::{AuditWriter, PgAuditWriter};
