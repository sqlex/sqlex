# Static Analyzer Implementation Guide

## 1. Purpose

This document translates `DESIGN.md` into an executable implementation plan for `sqlex-static-analyzer`.

Target outcome:

1. `StaticAnalyzer::execute` can apply migration DDL to an in-memory catalog.
2. `StaticAnalyzer::analyze` can parse, algebraize, infer, and return `ResultSet`.
3. `StaticAnalyzer::get_all_tables` returns schema metadata aligned with catalog state.
4. Behavior matches `tests/specs_runner.rs` contract across PostgreSQL, MySQL, SQLite.

## 2. Current Contract and Constraints

The implementation must follow existing interfaces:

- `Analyzer` trait in `crates/analyzer/sqlex-analyzer/src/lib.rs`
- output types in `crates/sqlex-common/src/types.rs`
- dialect enum in `crates/sqlex-common/src/dialect.rs`

Important constraints:

1. Keep public API minimal: export only `StaticAnalyzer` from this crate.
2. Avoid `pub use` re-exports.
3. No `unwrap()` in runtime logic; return structured analyzer errors.
4. Keep deterministic behavior for tests (stable column order and diagnostics ordering).

## 3. Scope

### 3.1 In Scope (v1)

1. DDL: `CREATE TABLE`, `ALTER TABLE` (add/drop column, add constraints), `DROP TABLE`.
2. Query analysis: `SELECT`, `WITH`/recursive CTE, joins, set operations, aggregation, window functions, `ORDER BY`, `LIMIT/OFFSET`, scalar/exists/in subqueries.
3. Inference: column names, data types, nullability, cardinality.
4. Dialect-specific behavior for identifier normalization, function validation, and naming quirks required by specs.

### 3.2 Out of Scope (v1)

1. DML side effects (`INSERT/UPDATE/DELETE`) in `execute`.
2. Views, sequences, stored procedures, user-defined types.
3. Cost-based optimization and SQL rewrite optimization.
4. Multi-error reporting API (current trait returns single `AnalyzerError`).

## 4. Proposed Source Layout

```text
crates/analyzer/sqlex-static-analyzer/src/
  lib.rs
  analyzer.rs
  diagnostics.rs
  parser.rs
  catalog/
    mod.rs
    model.rs
    normalize.rs
    mutator.rs
    ddl_type_map.rs
  algebraizer/
    mod.rs
    model/
      mod.rs
      relation.rs
      expression.rs
      schema.rs
    scope.rs
    cte.rs
    expression/*
    from_*.rs
    join.rs
    select.rs
    set_ops.rs
  infer/
    mod.rs
    metadata.rs
    scalar_infer.rs
    operator_infer.rs
    cardinality.rs
  functions/
    mod.rs
    model.rs
    common.rs
    postgres.rs
    mysql.rs
    sqlite.rs
```

Notes:

1. Keep `lib.rs` thin: module declarations + `StaticAnalyzer` export.
2. Keep module-private helpers internal; avoid cross-layer leakage.
3. Use `crate::...` absolute imports for internal modules.

## 5. Core Data Models

## 5.1 Catalog Model

```rust
pub struct Catalog {
    pub tables: Vec<TableSchema>, // preserve creation order
}

pub struct TableSchema {
    pub name: String,                  // normalized table key
    pub original_name: String,         // source text for diagnostics
    pub columns: Vec<ColumnSchema>,    // ordinal order
    pub primary_key: Option<KeyConstraint>,
    pub unique_keys: Vec<KeyConstraint>,
    pub foreign_keys: Vec<ForeignKeyConstraint>,
}

pub struct ColumnSchema {
    pub name: String,                  // normalized column key
    pub original_name: String,
    pub data_type: DataType,
    pub nullable: bool,
}

pub struct KeyConstraint {
    pub name: Option<String>,
    pub columns: Vec<String>,          // normalized column names
}

pub struct ForeignKeyConstraint {
    pub name: Option<String>,
    pub columns: Vec<String>,          // local columns
    pub ref_table: String,             // normalized table name
    pub ref_columns: Vec<String>,      // referenced columns
}
```

Catalog rules:

1. Primary-key nullability is dialect-aware and must match reference analyzer behavior:
   - PostgreSQL/MySQL: primary-key columns are treated as `nullable = false`.
   - SQLite: do not globally force primary-key columns to non-null. Follow `sqlex-database-analyzer` behavior from `PRAGMA table_info`: treat a PK column as `nullable = false` when `notnull = 1` or when it is an integer-like PK (`pk > 0 && type contains "int"`); otherwise it may remain nullable (for example, `TEXT PRIMARY KEY`).
2. Column order is preserved exactly as DDL order after alter operations.
3. Name lookup uses dialect normalization:
   - PostgreSQL: unquoted identifiers fold to lowercase.
   - MySQL/SQLite: preserve identifier text as current project convention.

## 5.2 Relational IR

Use the relational operators from `DESIGN.md` with concrete structs:

```rust
pub enum Relation {
    Scan(ScanNode),
    Values(ValuesNode),
    Selection(SelectionNode),
    Projection(ProjectionNode),
    Aggregation(AggregationNode),
    Window(WindowNode),
    Distinct(DistinctNode),
    Sort(SortNode),
    Limit(LimitNode),
    Alias(AliasNode),
    Join(JoinNode),
    SetOperation(SetOpNode),
}
```

Projection columns must carry visibility:

```rust
pub enum Visibility {
    Visible,
    Hidden,
}
```

## 5.3 Scalar IR

Scalar expression tree follows `DESIGN.md`, with one implementation detail:

1. Introduce a normalized expression fingerprint for `ORDER BY` equivalence and hidden-column dedup.
2. Represent placeholders (`?`, `$1`) as `Literal::Placeholder` for cardinality and type checks.

## 5.4 Inference Metadata

```rust
pub struct InferMetadata {
    pub columns: Vec<InferColumn>,
    pub cardinality: CardInterval,
    pub keys: Vec<ResolvedKey>,
}

pub struct InferColumn {
    pub name: String,
    pub data_type: DataType,
    pub nullable: bool,
    pub origin: ColumnOrigin,
}

pub enum ColumnOrigin {
    Base { table: String, column: String },
    Derived,
}
```

`keys` is required for selection cardinality refinement and join optimizations.

## 5.4.1 Bound Column Identity Model

To make column binding deterministic, `ColumnRef` should be bound to stable column identities during Algebraize, not repeatedly resolved by name in later phases.

Recommended model:

```rust
type SlotId = u32;
type RelationId = u32;

pub struct BoundColumn {
    pub slot_id: SlotId,                 // stable identity for downstream references
    pub name: String,                    // output column name at current node
    pub table_alias: Option<String>,     // for qualified resolution (t.col)
    pub origin: ColumnOrigin,            // lineage
}

pub struct OutputSchema {
    pub relation_id: RelationId,
    pub columns: Vec<BoundColumn>,
}

pub enum Expression {
    SlotRef(SlotId),
    CorrelatedRef { depth: usize, slot_id: SlotId },
    // ...
}
```

Design requirements:

1. Name resolution is completed in algebraize modules (`algebraizer/*`) only.
2. Infer phase consumes `SlotId`-bound expressions and `OutputSchema`, and performs no name lookup.
3. Diagnostics remain tied to original SQL text, while semantics are carried by slot identity.

## 5.5 Cardinality Domain

Internal engine uses interval:

```rust
pub enum MinRows { Zero, One }
pub enum MaxRows { Zero, One, Many }
pub struct CardInterval { pub min: MinRows, pub max: MaxRows }
```

Mapping to `sqlex_common::types::Cardinality`:

1. `[Zero, Zero] -> ExactlyZero`
2. `[One, One] -> ExactlyOne`
3. `[Zero, One] -> AtMostOne`
4. `[One, Many] -> OneOrMore`
5. `[Zero, Many] -> ZeroOrMore`

## 5.6 Diagnostics

```rust
pub struct Diagnostic {
    pub code: &'static str,
    pub phase: Phase,
    pub message: String,
}
```

Phase enum:

1. Parse
2. Catalog
3. Algebraize
4. Infer

v1 strategy is fail-fast with deterministic first error.

## 6. Analyzer Lifecycle

## 6.1 `StaticAnalyzer` Structure

```rust
pub struct StaticAnalyzer {
    dialect: Dialect,
    catalog: Catalog,
    functions: FunctionRegistry,
}
```

## 6.2 `execute(&mut self, sql)`

Pipeline:

1. Parse SQL into statements.
2. For each statement:
   - validate statement kind is DDL supported by catalog mutator.
   - apply mutator to catalog.
3. Stop at first failure and return `AnalyzerError::ExecutionError`.

## 6.3 `analyze(&self, sql)`

Pipeline:

1. Parse SQL (single query statement expected for `analyze`).
2. Algebraize AST + catalog into `Relation`.
3. Infer metadata from `Relation`.
4. Convert inferred columns/cardinality to `ResultSet`.

## 6.4 `get_all_tables(&self)`

Return `Vec<Table>` from catalog with stable column order.

## 7. Parser and Normalization

## 7.1 Parser Selection

Use `sqlparser` dialect implementations:

1. `PostgreSqlDialect`
2. `MySqlDialect`
3. `SQLiteDialect`

## 7.2 Identifier Normalization

Reuse policy from `sqlex-analyzer` `ObjectNameExt`:

1. PostgreSQL unquoted names fold to lowercase.
2. Quoted names preserve exact text.
3. MySQL/SQLite keep source case (project-level rule).

All lookup paths (`table`, `column`, `cte`) must normalize consistently.

## 8. DDL Mutator Specification (`execute`)

## 8.1 Supported Statements

1. `CREATE TABLE`
2. `ALTER TABLE ... ADD COLUMN`
3. `ALTER TABLE ... DROP COLUMN`
4. `ALTER TABLE ... ADD CONSTRAINT`
5. `DROP TABLE`

## 8.2 CREATE TABLE Rules

1. Reject duplicate table names.
2. Build column list with type mapping and nullability.
3. Collect inline constraints:
   - `PRIMARY KEY`
   - `UNIQUE`
   - `NOT NULL`
   - `REFERENCES`
4. Collect table constraints:
   - `PRIMARY KEY (...)`
   - `UNIQUE (...)`
   - `FOREIGN KEY (...) REFERENCES ...`
5. Validate referenced columns/tables exist for foreign keys.

## 8.3 ALTER TABLE Rules

1. `ADD COLUMN`:
   - reject duplicate column names.
   - append column at tail.
2. `DROP COLUMN`:
   - reject missing column.
   - reject dropping PK/UK/FK participant unless constraint removed first (or auto-remove in v1 with warning behavior disabled; choose strict reject for deterministic safety).
3. `ADD CONSTRAINT`:
   - support PK/UK/FK.
   - validate column existence and key shape.

## 8.4 DROP TABLE Rules

1. Reject missing table.
2. Reject drop when referenced by other table foreign key (strict mode), or remove dependent FKs if project decides permissive mode.
3. For test alignment, strict mode is preferred unless specs require cascade behavior.

## 8.5 SQL Type to `DataType` Mapping

Implement per dialect mapper in `catalog/ddl_type_map.rs`:

1. PostgreSQL:
   - `int2/int4/int8`, `float4/float8`, `numeric`, `varchar`, `char`, `text`, `timestamp`, `timestamptz`, etc.
2. MySQL:
   - signed/unsigned integer family, `double`, `decimal`, `varchar`, `char`, `text`, `json`, binary family.
3. SQLite:
   - affinity-based mapping (`int -> BigInt`, `varchar -> Varchar`, `char -> Char`, `text -> Text`, etc.).

Mirror behavior used by `sqlex-database-analyzer` mapping to reduce divergence.

## 9. Algebraize Specification (`analyze` phase 2)

## 9.1 Planner Scope State

`Algebraizer` keeps an internal scope state that holds:

1. visible relations in current scope
2. outer relation scopes for correlated subqueries
3. CTE bindings for current query block
4. named window definitions for current query block
5. relation/slot id allocators
6. literal-assignment mode for dialect-specific typing behavior

Scope is stack-based:

1. query block pushes scope
2. derived table/subquery pushes nested scope
3. leaving block pops scope

## 9.2 FROM/JOIN Construction

1. `FROM table`: build `Scan` + optional `Alias`.
2. derived table: algebraize inner query, then alias it.
3. joins:
   - `INNER/LEFT/RIGHT/FULL/CROSS`
   - `ON` condition scalarization
   - `USING` requires both sides contain all columns
4. MySQL `FULL JOIN` should raise semantic error (spec requirement).

## 9.3 Column Resolution

Resolution order:

1. qualified reference (`t.col`) checks relation alias first.
2. unqualified reference scans visible relations:
   - one match => success
   - zero => unknown column diagnostic
   - multiple => ambiguous column diagnostic

`JOIN USING` special:

1. unqualified using-column resolves to merged logical column.
2. explicit qualified references keep side semantics.

## 9.3.1 `ColumnRef` Binding Procedure

Binding should convert AST `ColumnRef` to bound scalar references:

1. try resolve in current scope:
   - qualified (`t.col`): locate relation by alias/name, then locate column in that relation output schema.
   - unqualified (`col`): scan all visible relations in current scope.
2. resolution result:
   - exactly one match -> bind to `Expression::SlotRef(slot_id)`.
   - zero matches -> continue searching outer scopes for correlated binding.
   - multiple matches -> emit ambiguous-column diagnostic.
3. correlated binding:
   - if found in outer scope at depth `d`, bind to `Expression::CorrelatedRef { depth: d, slot_id }`.
   - if not found in any outer scope, emit unknown-column diagnostic.

Implementation note:

1. After binding, all scalar expression nodes should carry `SlotRef/CorrelatedRef` instead of raw names.
2. Type/nullability inference for column references uses slot metadata directly.

## 9.3.2 Cross-Operator Output Schema Propagation

Each relational operator must publish a deterministic `OutputSchema` for its parent:

1. `Scan`: create fresh slots from catalog columns.
2. `Selection` / `Sort` / `Distinct` / `Limit`: pass through child schema unchanged.
3. `Alias`: preserve slots, update visible relation alias for qualified lookup.
4. `Projection`:
   - each projection item produces one output slot.
   - projection expressions reference child slots; output slots form a new schema.
5. `Aggregation`:
   - output contains group-by expression slots and aggregate result slots only.
   - non-grouped plain input columns are not visible unless grouped or aggregated.
6. `Window`:
   - pass through input slots and append slots for window expressions.
7. `Join`:
   - default output is left slots + right slots.
   - `JOIN USING` adds merged logical columns according to `USING` rules (or replaces duplicates, depending on chosen representation), and this behavior must be consistent for both binding and final projection.
8. `SetOperation`:
   - bind by position; output schema shape follows left branch column names/order.
   - right branch columns are compatibility-checked and mapped positionally.

Key principle:

1. Parent nodes bind against child `OutputSchema`, never by re-resolving raw SQL names across tree levels.
2. This prevents cross-level ambiguity and keeps correlated-subquery semantics explicit.

## 9.4 Aggregate/Window Detection

Pre-scan `SELECT`, `HAVING`, `ORDER BY`:

1. detect aggregate calls
2. detect window calls (`OVER`)

Validation:

1. no aggregate/window in `WHERE`.
2. no window in `HAVING`.
3. grouping rules for non-aggregated selected expressions.

## 9.5 Projection and Wildcard Expansion

1. expand `*` to all visible columns in scope order.
2. expand `t.*` to relation columns only.
3. keep explicit select item order.
4. attach visibility (`Visible` by default).

## 9.6 ORDER BY Binding and Hidden Columns

Binding priority:

1. ordinal
2. unique visible alias
3. expression equivalence
4. hidden column materialization (only when `DISTINCT` absent)

Hidden column rules:

1. dedup by normalized scalar fingerprint.
2. alias prefix: `__ord$<n>`.
3. append hidden columns in pre-sort projection.
4. apply final projection to drop hidden columns.

## 9.7 CTE Rules

Non-recursive CTE:

1. no duplicate CTE names.
2. no forward references.
3. no self reference.
4. optional column alias count must equal CTE output column count.

Recursive CTE:

1. require `WITH RECURSIVE`.
2. self-reference allowed only in recursive branch.
3. anchor and recursive branches must have same column count.
4. column alias count must match output columns.

Scope isolation:

1. CTE defined inside derived table must not leak outward.

PostgreSQL case rules:

1. unquoted CTE name fold to lowercase.
2. quoted CTE name exact-match behavior must be preserved.

## 9.8 Set Operations

For `UNION/INTERSECT/EXCEPT`:

1. left/right output column count must match.
2. output names come from left side.
3. type compatibility checked column-wise.

## 9.9 Structural Diagnostics Catalog

Minimum diagnostic set:

1. table not found
2. column not found
3. ambiguous column
4. invalid alias/table reference after aliasing
5. invalid group by
6. invalid function arity
7. unsupported dialect feature (for example MySQL full join)
8. invalid CTE usage (duplicate/self/forward/alias-count mismatch/scope leak)

## 10. Inference Specification (`analyze` phase 3)

## 10.1 Scalar Type and Nullability

Implement bottom-up `infer_scalar(expr, input_columns, fn_registry)`.

Core rules:

1. `ColumnRef`: from bound input column metadata.
2. `Literal`:
   - non-null literals => `nullable = false`
   - `NULL` => `nullable = true`, type unknown/custom.
3. `BinaryOp`:
   - arithmetic: numeric promotion
   - comparison/logical: result bool
   - nullable usually OR of operand nullability (except `IS NULL` family).
4. `CASE`: common-type merge + branch nullability aggregation.
5. `COALESCE`: nullable iff all arguments nullable.
6. `NULLIF`: always nullable.
7. `Cast`: target type, nullable from input.
8. `ScalarSubquery`: nullable true (can return no rows).
9. `Exists`: bool non-null.

## 10.2 Function Registry and Validation

Function registry includes:

1. category: scalar/aggregate/window
2. arity rule
3. return type rule
4. nullability rule
5. argument type validator with dialect coercion profile

Required function coverage from specs:

1. scalar: `UPPER`, `LOWER`, `TRIM`, `LTRIM`, `RTRIM`, `LENGTH`, `CHAR_LENGTH`, `ABS`, `CEIL`, `FLOOR`, `ROUND`, `SQRT`, `EXP`, `LN`, `LOG10`, `SIGN`, `POWER`, `MOD`, `SUBSTR`, `COALESCE`, `NULLIF`.
2. aggregate: `COUNT`, `SUM`, `AVG`, `MIN`, `MAX`.
3. window: `ROW_NUMBER`, `RANK`, `DENSE_RANK`, `LEAD`, `LAG`.

Dialect coercion policy:

1. PostgreSQL: strict for incompatible argument types.
2. MySQL: permissive implicit casts for test-covered functions.
3. SQLite: permissive for many functions but keep strict checks for cases required by specs (for example `CEIL/FLOOR` on text in function validation specs).

## 10.3 Operator Inference

Follow `DESIGN.md` per-operator rules with explicit implementations:

1. `infer_scan`
2. `infer_values`
3. `infer_selection`
4. `infer_projection`
5. `infer_aggregation`
6. `infer_window`
7. `infer_distinct`
8. `infer_sort`
9. `infer_limit`
10. `infer_alias`
11. `infer_join`
12. `infer_set_op`

## 10.4 Join Nullability and Cardinality

Implement:

1. base outer-join nullability forcing.
2. strict guaranteed-match optimization using FK + not-null + full referenced key coverage.
3. companion predicates for at-most-one side.
4. `JOIN USING` merged-column nullability rules.

## 10.5 Selection Cardinality Refinement

Implement extraction pipeline:

1. detect always-false condition (`FALSE`, `1=0`, contradictions).
2. reject OR-based refinement (conservative).
3. extract equality-constrained columns over AND conjuncts.
4. extract single-value `IN` constraints.
5. check full PK/UK coverage in input keys.
6. apply `constrain_at_most_one`.

## 10.6 Limit Cardinality

Implement rules from design:

1. `LIMIT 0`
2. `LIMIT 1`
3. `OFFSET` handling
4. `LIMIT 1 + OFFSET`

## 10.7 Set Operation Cardinality and Nullability

Implement interval combinators and per-op rules exactly as defined in design.

## 10.8 Column Name Inference Policy

This is dialect-sensitive and required for spec parity.

PostgreSQL:

1. plain column ref -> column name.
2. aggregate/function without alias -> lowercase function label (`count`, `max`, `coalesce`, `length`, `case`).
3. general expression without alias -> `?column?`.

MySQL:

1. plain column ref -> column name.
2. expression without alias -> SQL textual representation (for example `age + 1`, `COUNT(*)`).
3. string literal column name removes surrounding single quotes for bare literal select (`'hello'` -> `hello`), matching current specs.

SQLite:

1. plain column ref -> column name.
2. expression without alias -> SQL textual representation.
3. string literal keeps quotes in textual representation (`'hello'`).

## 11. Error Mapping Strategy

`Diagnostic` to public errors:

1. parse/catalog errors during `execute` -> `AnalyzerError::ExecutionError`.
2. parse/algebraize/infer errors during `analyze` -> `AnalyzerError::AnalysisError`.

Message format suggestion:

```text
[ALGEBRAIZE:E1003] ambiguous column 'id'
```

## 12. Testing Strategy

## 12.1 Unit Tests

Add focused unit tests for:

1. identifier normalization per dialect.
2. DDL mutator operations.
3. function registry arity/type checks.
4. order-by planning behavior (ordinal/alias/hidden columns).
5. cardinality combinators and selection refinement.
6. join guaranteed-match logic.

## 12.2 Integration Specs

Use existing `tests/specs_runner.rs` as contract.

Recommended loop:

```bash
cargo test -p sqlex-static-analyzer --test specs_runner -- --specs postgres/cardinality/basic
cargo test -p sqlex-static-analyzer --test specs_runner -- --specs mysql/errors/advanced
```

## 12.3 Regression Gates

Before each merge:

1. `cargo fmt --all`
2. `cargo clippy --all-targets --all-features -- -D warnings`
3. full specs runner for three dialects

## 13. Delivery Milestones

## Milestone A: Skeleton and Catalog

1. Create module layout.
2. Implement parser wrapper and diagnostics base.
3. Implement catalog + DDL mutator + `get_all_tables`.
4. Pass all `ddl/*` specs.

## Milestone B: Core Query Path

1. Implement scan/projection/selection/join/set-op algebraization.
2. Implement scalar inference basics and projection naming.
3. Pass `select/basic`, `select/advanced`, `join/basic`, `set_ops/basic`.

## Milestone C: Aggregation and Window

1. Aggregate detection, grouping validation, aggregation inference.
2. Window classification and inference.
3. Pass `agg/*`, `window/*`, `errors/function_validation`.

## Milestone D: CTE and Subquery Semantics

1. CTE scope engine (recursive and non-recursive).
2. Derived-table isolation and correlated subquery binding.
3. Pass `select/cte*`, `errors/cte_*`.

## Milestone E: Cardinality and Nullability Refinement

1. key propagation through operators.
2. selection key-based at-most-one analysis.
3. join guaranteed-match nullability/cardinality optimization.
4. pass `cardinality/*`, `nullability/*`.

## Milestone F: Final Hardening

1. run full specs across all dialects.
2. close remaining mismatches against database analyzer.
3. perform linting/format gates.

## 14. Implementation Checklist

1. Build parser wrapper with dialect dispatch.
2. Build catalog model and DDL mutator.
3. Build function registry (common + dialect overrides).
4. Build relational/scalar IR.
5. Implement algebraizer phase.
6. Implement inferencer and operator rules.
7. Wire analyzer methods and error mapping.
8. Verify spec files only use canonical `Cardinality` values.
9. Run full formatter/linter/spec suite.

## 15. Definition of Done

The static analyzer implementation is complete when:

1. all analyzer trait methods are fully implemented (no `todo!()`).
2. all current specs pass for PostgreSQL/MySQL/SQLite.
3. generated result metadata matches database analyzer contract for names/types and spec contract for nullability/cardinality.
4. code passes `fmt` and strict `clippy`.
