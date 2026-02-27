# Static Analyzer Error Codes

This document defines the current error-code convention used by `sqlex-static-analyzer`.

## 1. Rendering Format

Analyzer failures are surfaced as:

```text
[CODE] message
```

`CODE` comes from `AnalyzerError::analysis(code, message)`.

## 2. Source of Truth

Error-code constants are defined in source and should be treated as canonical:

1. Parse: `src/error_code.rs`
2. Catalog: `src/catalog/error_code.rs`
3. Algebraizer: `src/algebraizer/error_code.rs`
4. Infer: `src/infer/error_code.rs`

When adding or changing codes, update constants first, then update specs and this document.

## 3. Code Format

All codes use `<module><major><minor>`:

1. `<module>`: one uppercase letter (`P`/`C`/`A`/`I`)
2. `<major>`: two digits (`00`-`99`) for top-level category
3. `<minor>`: two digits (`00`-`99`) for sub-category

## 4. Module Segments

### 4.1 Parse (`P`)

1. `P00xx`: parser entry and statement-shape validation

### 4.2 Catalog (`C`)

1. `C00xx`: catalog storage
2. `C01xx`: statement dispatch
3. `C02xx`: `CREATE TABLE`
4. `C03xx`: `ALTER TABLE`
5. `C04xx`: `DROP TABLE`
6. `C05xx`: constraint column validation
7. `C06xx`: foreign-key validation
8. `C07xx`: drop-column dependency checks

### 4.3 Algebraizer (`A`)

1. `A00xx`: scalar/column resolution baseline
2. `A01xx`: projection/alias/grouping binding
3. `A02xx`: function and CTE semantic checks
4. `A03xx`: `ORDER BY` / set-op / subquery checks
5. `A04xx`: aggregate/window semantics
6. `A05xx`: join and dialect-specific join/function checks
7. `A06xx`: unsupported SQL features in current algebraizer path
8. `A07xx`: terminal unsupported path checks

### 4.4 Infer (`I`)

1. `I01xx`: expression-level inference and validation
2. `I02xx`: relation/cardinality-level inference

## 5. Maintenance Rules

1. Use constants; avoid hard-coded string literals in logic.
2. Keep codes stable once released; evolve message text when possible.
3. Sync `tests/specs/**` `expected_error_code` values when code values change.
4. Keep this file aligned with constants-module segment definitions.
