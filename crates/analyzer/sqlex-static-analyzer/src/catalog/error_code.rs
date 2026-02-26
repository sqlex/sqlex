//! Catalog error code definitions.
//!
//! Numbering policy:
//! - Format: `<module><major><minor>`, where module is `C` for catalog.
//! - `<major>`: 2 digits (00-99), top-level category.
//! - `<minor>`: 2 digits (00-99), sub-category.
//! - Catalog majors:
//!   - `00`: catalog storage layer
//!   - `01`: statement dispatch
//!   - `02`: CREATE TABLE
//!   - `03`: ALTER TABLE
//!   - `04`: DROP TABLE
//!   - `05`: table-constraint column validation
//!   - `06`: foreign-key validation
//!   - `07`: DROP COLUMN dependency checks

/// The target table already exists in catalog storage.
pub(crate) const CATALOG_TABLE_ALREADY_EXISTS: &str = "C0000";
/// The target table does not exist in catalog storage.
pub(crate) const CATALOG_TABLE_NOT_FOUND: &str = "C0001";

/// The SQL statement is not supported by catalog execution dispatch.
pub(crate) const DISPATCH_UNSUPPORTED_STATEMENT: &str = "C0100";

/// `CREATE TABLE AS SELECT` is not supported.
pub(crate) const CREATE_TABLE_AS_SELECT_UNSUPPORTED: &str = "C0200";
/// `CREATE TABLE` failed because table already exists.
pub(crate) const CREATE_TABLE_ALREADY_EXISTS: &str = "C0201";
/// `CREATE TABLE` contains duplicated column names.
pub(crate) const CREATE_TABLE_DUPLICATE_COLUMN: &str = "C0202";

/// `ALTER TABLE` target table was not found.
pub(crate) const ALTER_TABLE_TABLE_NOT_FOUND: &str = "C0300";
/// `ALTER TABLE ADD COLUMN` failed because column already exists.
pub(crate) const ALTER_TABLE_ADD_COLUMN_EXISTS: &str = "C0301";
/// `ALTER TABLE DROP COLUMN` failed because column was not found.
pub(crate) const ALTER_TABLE_DROP_COLUMN_NOT_FOUND: &str = "C0302";
/// The `ALTER TABLE` operation is not supported.
pub(crate) const ALTER_TABLE_OPERATION_UNSUPPORTED: &str = "C0303";
/// SQLite does not support `ALTER TABLE ADD CONSTRAINT`.
pub(crate) const ALTER_TABLE_ADD_CONSTRAINT_UNSUPPORTED_SQLITE: &str = "C0304";

/// `DROP` statement only supports dropping tables.
pub(crate) const DROP_ONLY_TABLE_SUPPORTED: &str = "C0400";
/// `DROP TABLE` target table does not exist.
pub(crate) const DROP_TABLE_NOT_FOUND: &str = "C0401";
/// `DROP TABLE` is blocked because table is referenced by foreign keys.
pub(crate) const DROP_TABLE_REFERENCED_BY_FOREIGN_KEY: &str = "C0402";

/// A table constraint requires at least one column.
pub(crate) const CONSTRAINT_COLUMNS_EMPTY: &str = "C0500";
/// A table constraint references an unknown column.
pub(crate) const CONSTRAINT_COLUMN_NOT_FOUND: &str = "C0501";
/// A table constraint contains duplicated columns.
pub(crate) const CONSTRAINT_DUPLICATE_COLUMN: &str = "C0502";

/// A foreign key has no local columns.
pub(crate) const FOREIGN_KEY_LOCAL_COLUMNS_EMPTY: &str = "C0600";
/// Local and referenced column counts differ in a foreign key.
pub(crate) const FOREIGN_KEY_COLUMN_COUNT_MISMATCH: &str = "C0601";
/// A foreign key references a missing local column.
pub(crate) const FOREIGN_KEY_LOCAL_COLUMN_NOT_FOUND: &str = "C0602";
/// A foreign key references an unknown table.
pub(crate) const FOREIGN_KEY_REF_TABLE_NOT_FOUND: &str = "C0603";
/// A foreign key references an unknown column.
pub(crate) const FOREIGN_KEY_REF_COLUMN_NOT_FOUND: &str = "C0604";

/// A column cannot be dropped because it is used by primary key.
pub(crate) const DROP_COLUMN_USED_BY_PRIMARY_KEY: &str = "C0700";
/// A column cannot be dropped because it is used by unique key.
pub(crate) const DROP_COLUMN_USED_BY_UNIQUE_KEY: &str = "C0701";
/// A column cannot be dropped because it is used by foreign key.
pub(crate) const DROP_COLUMN_USED_BY_FOREIGN_KEY: &str = "C0702";
/// A column cannot be dropped because it is referenced by other foreign keys.
pub(crate) const DROP_COLUMN_REFERENCED_BY_FOREIGN_KEYS: &str = "C0703";
