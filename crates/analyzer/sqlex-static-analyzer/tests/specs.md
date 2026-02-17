# Specs Authoring Guide

This document is the source of truth for `tests/specs/**/*.yaml`.

## 1. Goals

`specs_runner` validates static analyzer behavior with the following contract:

1. SQL validity and output shape are compared against the database analyzer.
2. `nullability` and `cardinality` are asserted from spec expectations.
3. Error-code assertions lock static analyzer diagnostics.
4. TDD-first cases can be tracked in specs without breaking CI.

## 2. Directory Organization

Directory names are only for organization and maintenance.
They do **not** define runtime semantics.

Recommended layout:

1. `tests/specs/common/**`: shared scenarios.
2. `tests/specs/mysql/*.yaml`: MySQL-focused scenarios (flat under dialect directory).
3. `tests/specs/postgres/*.yaml`: PostgreSQL-focused scenarios (flat under dialect directory).
4. `tests/specs/sqlite/*.yaml`: SQLite-focused scenarios (flat under dialect directory).
5. `tests/specs/mysql+postgres/*.yaml`: MySQL+PostgreSQL scenarios.
6. `tests/specs/mysql+sqlite/*.yaml`: MySQL+SQLite scenarios.
7. `tests/specs/postgres+sqlite/*.yaml`: PostgreSQL+SQLite scenarios.

File naming convention:

1. Use lowercase snake_case only: `[a-z0-9_]+.yaml`.
2. In flat dialect directories (`mysql`, `postgres`, `sqlite`, `mysql+postgres`, `mysql+sqlite`, `postgres+sqlite`), use a category prefix in filenames, such as `select_`, `errors_`, `cardinality_`, `nullability_`, `join_`, `agg_`, `functions_`, `ddl_`, `contracts_`, `window_`, or `set_ops_`.
3. Do not include dialect names in filenames when the directory already encodes dialect scope.
4. Separate trailing numbers with `_`, for example `advanced_2.yaml`.

Dialect execution is determined only by YAML fields.

## 3. YAML Schema

Top-level fields:

1. `dialects` (optional): non-empty `string[]` of `mysql`/`postgres`/`sqlite`.
2. `migrations` (optional): `string[]`; each SQL statement runs before query cases.
3. `queries` (required): query cases.

`queries[]` fields:

Field reference:

| Field | Required | Description |
| --- | --- | --- |
| `name` | Yes | Query case name, unique in the file. |
| `sql` | Yes | SQL text. |
| `expected` | Depends on case type | Expected output column list. |
| `cardinality` | Depends on case type | Expected result cardinality. |
| `expected_error_code` | Depends on case type | Expected static analyzer diagnostic code. |
| `tdd_reason` | Depends on case type | Reason for temporarily skipping this case by default. |

Case categories:

| Case category | Must set | Must not set | Notes |
| --- | --- | --- | --- |
| Valid-query | `expected`, `cardinality` | `expected_error_code` | Used for legal-request output assertions. |
| Invalid-query | `expected_error_code` | `expected`, `cardinality` | Used for valid/invalid parity and static code checks. |
| TDD valid-query | `tdd_reason`, `expected`, `cardinality` | `expected_error_code` | Same as valid-query, but skipped unless `SQLEX_RUN_TDD=1`. |
| TDD invalid-query | `tdd_reason`, `expected_error_code` | `expected`, `cardinality` | Same as invalid-query, but skipped unless `SQLEX_RUN_TDD=1`. |
| Legacy status-only | none | none | Compatibility-only mode. Do not add new cases in this shape. |

`expected[]` fields:

1. `name` (required).
2. `nullability` (required).

## 4. Dialect Resolution

Dialect resolution uses top-level `dialects` only:

1. If `dialects` is set, run exactly the listed dialects.
2. If `dialects` is absent, run all supported dialects (`mysql`, `postgres`, `sqlite`).

## 5. Validation Rules

The runner fails fast when specs are malformed:

1. Unknown top-level fields are rejected.
2. `queries` may be empty (for migration-only specs), but every present query must be valid.
3. `dialects`, when present, must be non-empty and duplicate-free.
4. Query names must be unique.
5. `sql` must be non-empty.
6. `tdd_reason`, when present, must be non-empty.
7. `expected_error_code`, when present, must be non-empty.
8. Valid-query shape: if `expected` is non-empty, `cardinality` is required and `expected_error_code` must be absent.
9. Invalid-query shape: if `expected_error_code` is set, both `expected` and `cardinality` must be absent.
10. TDD shape: if `tdd_reason` is set, the query must satisfy either the valid-query shape or the invalid-query shape.
11. Status-only shape is legacy-compatible only: it is allowed only when `expected`, `cardinality`, and `expected_error_code` are all absent; do not add new cases in this shape.

## 6. Assertion Modes

### 6.1 Valid-Query Assertions

Condition:

1. `expected` is non-empty.
2. `cardinality` is set.

Checks:

1. The database analyzer and static analyzer must both treat the SQL as a valid request.
2. `cardinality` must match static analyzer output.
3. Column count and names must match database analyzer output.
4. Static types must match database analyzer output (`sqlite_types_compatible` exceptions apply).
5. Static nullability must match `expected[].nullability`.

### 6.2 Error-Code Assertions

Condition:

1. `expected_error_code` is set.

Checks:

1. The database analyzer and static analyzer must both treat the SQL as an invalid request.
2. Static error code must match `expected_error_code`.

Error code format:

```text
[PHASE:CODE] message
```

Examples:

1. `A3071`
2. `A3065`

### 6.3 TDD Cases

Condition:

1. `tdd_reason` is set.

Checks:

1. The query must still satisfy either valid-query assertions or error-code assertions.
2. `tdd_reason` must be non-empty and explain why the case is not enabled yet.

Notes:

1. The case is skipped by default.
2. The case runs only when `SQLEX_RUN_TDD=1`.
3. New specs should use the typed categories in Section 3.

### 6.4 Status-Only Assertions (Legacy Compatibility)

Condition:

1. `expected` is absent (or empty).
2. `cardinality` is absent.
3. `expected_error_code` is absent.

Checks:

1. Only valid/invalid request status parity is checked between database analyzer and static analyzer.
2. No detailed assertions are performed for `cardinality`, output columns (name/type/nullability), or static error code.

Notes:

1. This mode is kept only for backward compatibility.
2. Do not add new specs in this mode.

## 7. Examples

### 7.1 Shared Valid-Query Case

```yaml
migrations:
  - CREATE TABLE users (id INT NOT NULL, name TEXT)
queries:
  - name: select_users
    sql: SELECT id, name FROM users
    cardinality: ZeroOrMore
    expected:
      - name: id
        nullability: false
      - name: name
        nullability: true
```

### 7.2 Error-Code Case

```yaml
dialects:
  - postgres
queries:
  - name: unsupported_literal
    sql: SELECT X'AB'
    expected_error_code: A3073
```

### 7.3 Multi-Dialect Case

```yaml
dialects:
  - mysql
  - sqlite
queries:
  - name: subset_case
    sql: SELECT 1
    cardinality: ExactlyOne
    expected:
      - name: "1"
        nullability: false
```

### 7.4 TDD Case

```yaml
queries:
  - name: future_recursive_case
    sql: WITH RECURSIVE t(n) AS (...) SELECT n FROM t
    expected_error_code: A3067
    tdd_reason: "Feature is not implemented yet and tracked in roadmap."
```

## 8. Commands

Run all specs:

```bash
cargo test -p sqlex-static-analyzer --test specs_runner
```

Run all specs with full logs:

```bash
cargo test -p sqlex-static-analyzer --test specs_runner -- --nocapture
```

Run filtered specs:

```bash
SQLEX_SPECS_FILTER=sqlite/select_column_names cargo test -p sqlex-static-analyzer --test specs_runner
```

Run filtered specs with full logs:

```bash
SQLEX_SPECS_FILTER=sqlite/select_column_names cargo test -p sqlex-static-analyzer --test specs_runner -- --nocapture
```

Enable TDD cases:

```bash
SQLEX_RUN_TDD=1 cargo test -p sqlex-static-analyzer --test specs_runner
```
