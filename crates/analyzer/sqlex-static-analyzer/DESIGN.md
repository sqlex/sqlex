# Static Analyzer Design

## What Is Static Analyzer

Given an input SQL statement together with Catalog metadata (and dialect/function registry context), the static analyzer derives query result metadata without executing the query against a live database.

For each analyzable SQL statement, the output model contains:
- output columns (names)
- output data types
- output nullability
- result cardinality
- diagnostics for structural semantic errors and type-dependent errors

## What Analysis Results Enable

In application code, query results are often mapped into handwritten structs or maps. Those mappings drift easily when SQL changes, when table schemas evolve, or when result shape assumptions are wrong.

The static analyzer removes that drift by producing metadata that generators can consume directly:
- **Column names + data types** define generated row-object fields and field types (type safety).
- **Nullability** defines whether generated fields are optional/nullable (null safety).
- **Cardinality** defines the generated return shape (single value, optional value, collection, or guaranteed empty).

Recommended cardinality-to-return-shape mapping:

```
Cardinality   Generated API Shape
────────────────────────────────────────────────────────────────────
ExactlyZero   no value / unit-like return (query is guaranteed empty)
ExactlyOne    non-null single row object
AtMostOne     optional single row object
OneOrMore     collection with non-empty guarantee (or collection + contract)
ZeroOrMore    collection
```

This mapping turns common runtime failures (wrong receiver shape, wrong nullable assumptions, stale field mapping) into compile-time diagnostics and generated-type constraints.

## Theoretical Foundation

### Why Relational Algebra?

SQL is a declarative language built on relational algebra — the mathematical framework invented by E.F. Codd in 1970. Every SQL query, no matter how complex, can be decomposed into a composition of a small set of relational operators: Selection (σ), Projection (π), Join (⋈), Aggregation (γ), Set Operations (∪, ∩, −), etc.

Each relational operator has **well-defined semantic rules** for how it transforms its input relation(s):

- **Schema**: what columns appear in the output
- **Data types**: what type each output column has
- **Nullability**: whether each output column can be NULL
- **Cardinality**: how many rows the output may contain

By converting SQL into a relational algebra expression tree and then recursively applying per-operator rules, we can infer complete metadata for any query result set — without executing the query or connecting to a database.

This approach is superior to ad-hoc, SQL-AST-oriented analysis because:

1. **Completeness** — the operator set is finite and closed; covering all operators means covering all SQL
2. **Composability** — operators compose naturally; nested queries, CTEs, and complex joins are just deeper trees
3. **Correctness** — each operator's rules are derived from relational algebra theory, not reverse-engineered from SQL syntax
4. **Extensibility** — adding support for new SQL features means mapping them to existing operators or adding a new operator with its own rules

## Three-Phase Pipeline

```
SQL text ──► Parse ──► Algebraize ──► Infer ──► ResultSet
              │           │              │
           sqlparser   AST+Catalog    RelationalExpr
                      → RelationalExpr → Metadata
```

### Phase 1: Parse

SQL text is parsed into an Abstract Syntax Tree (AST). This phase uses an external SQL parser with dialect-aware support. The AST preserves the syntactic structure of the SQL statement.

**Available information**: SQL text, dialect

**Responsibilities**:
- Tokenize and parse SQL text into a structured AST
- Report syntax errors (malformed SQL)

**Not responsible for**:
- Semantic validation (unknown tables, type mismatches, etc.)
- Any access to the Catalog or function metadata
- Any transformation beyond syntactic parsing

### Phase 2: Algebraize

The AST is converted into a **RelationalExpr** tree — a relational algebra representation.

**Available information**: AST, Catalog (database schema), Function registry (function signatures and classification)

**Responsibilities**:
- Resolve table references against the Catalog (verify table existence)
- Resolve column references against input relations (verify column existence, disambiguate)
- Classify functions (scalar / aggregate / window) using the function registry
- Validate function arity (argument count)
- Validate SQL semantics: GROUP BY rules, aggregate context restrictions (no aggregates in WHERE), window function context restrictions
- Infer output column names (aliases, expression-based naming)
- Expand projection wildcards (`*`, `t.*`) into explicit column references for structural planning
- Build the RelationalExpr tree bottom-up following SQL's logical execution order

**Not responsible for**:
- Data type inference (does not determine what type a column or expression has)
- Nullability inference (does not determine whether a column can be NULL)
- Cardinality inference (does not determine how many rows a query returns)
- Function return type inference (does not determine what type a function returns)
- Function argument type validation (requires type information which is not yet available)

Validation is **integrated into construction**, not a separate pass. If a semantic error is detected during tree construction (e.g., aggregate in WHERE clause, unknown table), a diagnostic is emitted and analysis continues where possible.

**Key principle**: The Algebraize phase is concerned with **structural correctness** — is this a valid SQL query? Can all references be resolved? It builds the tree but does not annotate it with type or nullability information.

### Phase 3: Infer

The RelationalExpr tree is traversed **bottom-up recursively**, computing metadata at each node.

**Available information**: RelationalExpr tree, Catalog (for Scan node column types/nullability, primary keys, unique constraints, foreign keys), Function registry (for return type and nullability inference)

**Responsibilities**:
- Infer data types for all output columns (from catalog lookups, literal types, operator rules, function return types)
- Infer nullability for all output columns (from catalog constraints, operator semantics, function nullability rules)
- Infer cardinality (from operator rules, primary key / unique constraint analysis, limit analysis)
- Validate function argument types (requires type information computed during inference)

**Not responsible for**:
- Name resolution (all references are already resolved in the RelationalExpr tree)
- Structural semantic validation (the tree shape and name resolution are assumed valid)
- Modifying the RelationalExpr tree (inference is read-only)

**Key principle**: The Infer phase is **read-only** with respect to `RelationalExpr` and deterministic in metadata derivation. It may emit type-dependent diagnostics (for example, function argument type mismatch), but it does not perform structural validation or name resolution.

```
infer(node):
    for each child of node:
        child_metadata = infer(child)
    return apply_operator_rules(node, child_metadata...)
```

## Relational Algebra Expression Tree

The core intermediate representation is a tree of relational operators. Every SQL query is represented as a composition of these operators.

### Operator Catalog

```
Category        Operator        Notation    Description
──────────────────────────────────────────────────────────────────────
Leaf nodes      Scan            R           Read all rows from a base table
                Values          {r1,r2,..}  Literal row set (e.g. SELECT 1, VALUES ...)

Unary ops       Selection       σ(R)        Filter rows by a predicate
                Projection      π(R)        Choose / compute output columns
                Aggregation     γ(R)        Group rows and apply aggregate functions
                Window          ω(R)        Compute window functions over partitions
                Distinct        δ(R)        Eliminate duplicate rows
                Sort            τ(R)        Order rows by sort keys
                Limit           λ(R)        Restrict row count (LIMIT / OFFSET)

Alias           Alias           ρ(R)        Rename a relation (derived table, CTE)

Binary ops      Join            R ⋈ S       Combine two relations by a condition
                SetOperation    R ∪ S       Combine two relations by set theory
```

### Tree Structure (Pseudocode)

```
RelationalExpr =
    // Leaf nodes
    | Scan(table)
    | Values(rows: [[ScalarExpr]])

    // Unary operators
    | Selection(input: RelationalExpr, condition: ScalarExpr)
    | Projection(input: RelationalExpr, columns: [ProjectionColumn])
    | Aggregation(input: RelationalExpr, group_by: [ScalarExpr], aggregates: [ProjectionColumn])
    | Window(input: RelationalExpr, window_exprs: [ProjectionColumn])
    | Distinct(input: RelationalExpr)
    | Sort(input: RelationalExpr, keys: [SortKey])
    | Limit(input: RelationalExpr, count?: ScalarExpr, offset?: ScalarExpr)

    // Alias
    | Alias(input: RelationalExpr, name: String)

    // Binary operators
    | Join(left: RelationalExpr, right: RelationalExpr, kind: JoinKind, condition?: JoinCondition)
    | SetOperation(left: RelationalExpr, right: RelationalExpr, op: SetOp, all: bool)
```

Projection columns carry visibility:

```
ProjectionColumn = {
    expr: ScalarExpr,
    alias?: String,
    visibility: Visible | Hidden
}
```

`Hidden` projection columns are internal-only columns used for planning (for example, sorting by an expression not present in the final output). A final Projection step drops hidden columns before producing the query ResultSet.

## Scalar Expression Tree

Relational operators work on relations (sets of rows). The expressions **within** operators — predicates, computed columns, sort keys — are **scalar expressions** that compute a single value per row.

```
ScalarExpr =
    // References and literals
    | ColumnRef(table?, column)          -- qualified (t.col) or unqualified (col)
    | Literal(NULL | Bool | Int | Float | String)

    // Operators
    | BinaryOp(left: ScalarExpr, op, right: ScalarExpr)   -- +, -, =, <, AND, OR, LIKE, ...
    | UnaryOp(op, expr: ScalarExpr)                        -- NOT, -, +

    // Function calls
    | Function(name, args: [ScalarExpr])                   -- scalar functions: UPPER, COALESCE, ...
    | AggregateCall(name, args: [ScalarExpr], distinct?)   -- COUNT, SUM, AVG, ...
    | WindowCall(name, args: [ScalarExpr], partition_by, order_by)  -- ROW_NUMBER, RANK, ...

    // Type conversion
    | Cast(expr: ScalarExpr, target_type)

    // Predicates
    | IsNull(expr: ScalarExpr, negated?)
    | InList(expr: ScalarExpr, list: [ScalarExpr], negated?)
    | Between(expr: ScalarExpr, low: ScalarExpr, high: ScalarExpr, negated?)

    // Conditional
    | Case(operand?: ScalarExpr, when_clauses: [(condition, result)], else?: ScalarExpr)

    // Subqueries (embedding relational expressions inside scalar context)
    | ScalarSubquery(RelationalExpr)                       -- (SELECT max(x) FROM ...)
    | InSubquery(expr: ScalarExpr, RelationalExpr, negated?)
    | Exists(RelationalExpr, negated?)

    // Wildcards (for SELECT *)
    | Wildcard
    | QualifiedWildcard(table)
```

`Wildcard` and `QualifiedWildcard` used by projection (`SELECT *`, `SELECT t.*`) are normalized into explicit projection columns during Algebraize projection construction. Infer operates on the expanded representation.

`COUNT(*)` is handled as an aggregate-specific star argument and is **not** expanded into projection columns.

### Key Design Decision: Aggregates in ScalarExpr

Aggregate functions (COUNT, SUM, ...) appear as `AggregateCall` nodes within `ScalarExpr` because they are syntactically embedded in SQL expressions (e.g., `SELECT price * COUNT(*)`). However, during the Algebraize phase, the presence of aggregate calls triggers the creation of an `Aggregation` operator in the relational tree. The Aggregation operator is the **relational-level** representation of grouping; the `AggregateCall` in ScalarExpr is the **scalar-level** reference to an aggregate computation.

## Algebraize: SQL to Relational Algebra Conversion

### SQL's Logical Execution Order

SQL has a well-defined logical execution order that differs from its syntactic order. The Algebraize phase builds the RelationalExpr tree **bottom-up** following this logical order:

```
Step    SQL Clause                Relational Operator        What it does
────────────────────────────────────────────────────────────────────────────────
1st     FROM / JOIN               Scan, Join, Alias          Establish input relations
2nd     WHERE                     Selection                  Filter rows before grouping
3rd     GROUP BY + aggregates*    Aggregation               Group and aggregate (*aggregates detected across SELECT/HAVING/ORDER BY)
4th     HAVING                    Selection                 Filter groups
5th     Window functions*         Window                    Compute window expressions (*detected across SELECT/ORDER BY)
6th     SELECT (+ hidden sort)    Projection                Build visible output columns plus optional hidden sort columns
7th     DISTINCT                  Distinct                  Eliminate duplicates
8th     ORDER BY                  Sort                      Order result rows
9th     LIMIT / OFFSET            Limit                     Restrict output row count
10th    FINAL OUTPUT              Projection                Drop hidden columns
```

### Conversion Pseudocode

```
algebraize_select(query):
    // Step 1: FROM → base relation
    expr = build_from(query.from)          // Scan, Join, Alias nodes

    // Pre-scan expression classes across clauses
    agg_usage = collect_aggregate_usage(query.select, query.having, query.order_by)
    win_usage = collect_window_usage(query.select, query.order_by)

    // Step 2: WHERE → Selection
    if query.where:
        validate_no_aggregates(query.where)
        validate_no_window_functions(query.where)
        expr = Selection(expr, build_scalar(query.where))

    // Step 3: GROUP BY + aggregates → Aggregation
    if query.group_by or agg_usage.any:
        validate_grouping_rules(query)
        expr = Aggregation(expr, build_group_by(query), build_projections(query))

    // Step 4: HAVING → Selection
    if query.having:
        expr = Selection(expr, build_scalar(query.having))

    // Step 5: Window functions → Window
    if win_usage.any:
        expr = Window(expr, build_window_exprs(query))

    // Step 6.1: Build visible SELECT projection columns
    // (including wildcard expansion: `*`, `t.*`)
    visible_cols = build_visible_projection_columns(query.select)

    // Step 6.2: ORDER BY planning block
    {
        // 1) Resolve ORDER BY references (ordinal / alias / expression equivalence)
        order_refs = resolve_order_by_refs(query.order_by, visible_cols)

        // 2) Validate ORDER BY constraints (DISTINCT restrictions, alias ambiguity, etc.)
        validate_order_by_constraints(order_refs, query.distinct)

        // 3) Materialize hidden sort columns when allowed, then build sort keys
        hidden_sort_cols, rewritten_order_refs = materialize_hidden_sort_columns(
            order_refs,
            query.distinct
        )
        sort_keys = build_sort_keys(rewritten_order_refs)
    }

    // Step 6.3: SELECT + optional hidden ORDER BY expressions → Projection
    expr = Projection(expr, visible_cols + hidden_sort_cols)

    // Step 7: DISTINCT → Distinct
    if query.distinct:
        expr = Distinct(expr)

    // Step 8: ORDER BY → Sort
    if query.order_by:
        expr = Sort(expr, sort_keys)

    // Step 9: LIMIT / OFFSET → Limit
    if query.limit or query.offset:
        expr = Limit(expr, query.limit, query.offset)

    // Step 10: Final output projection (drop hidden columns)
    if hidden_sort_cols not empty:
        expr = Projection(expr, visible_cols)

    return expr
```

Notes:
- Hidden sort columns allow ordering by non-output expressions for non-`DISTINCT` queries.
- For `DISTINCT`, if `ORDER BY` references expressions not present in visible output, emit a diagnostic (no hidden-sort fallback).

### ORDER BY Binding Policy

`ORDER BY` items are resolved against the post-SELECT projection with this priority:
1. **Ordinal reference** (`ORDER BY 1`, `ORDER BY 2`, ...): bind to visible projection position.
2. **Alias reference**: bind to a uniquely named visible projection alias.
3. **Expression equivalence**: bind to a visible projection expression if normalized scalar expression is equivalent.
4. **Fallback**:
   - if `DISTINCT` is absent: materialize the expression as a hidden projection column and bind to it.
   - if `DISTINCT` is present: emit a diagnostic.

Distributed ORDER BY planning flow:
1. For each `ORDER BY` item, try binding in this order: ordinal position, unique visible alias, then normalized expression equivalence.
2. If binding succeeds, generate a sort key pointing to the bound visible column.
3. If binding fails and query is not `DISTINCT`, materialize a hidden projection column for that expression, deduplicated by normalized expression shape, then bind sort key to that hidden column.
4. If binding fails and query is `DISTINCT`, emit a diagnostic.
5. Return both planned hidden columns and sort keys.

Implementation notes:
- Hidden columns are deduplicated by normalized scalar expression, not by textual SQL.
- Hidden aliases use an internal reserved prefix (for example `__ord$`) and are never exposed in final output.
- If alias binding is ambiguous (duplicate visible aliases), emit a diagnostic.

### Example: SQL to Relational Algebra Tree

Given:
```sql
SELECT u.name, COUNT(o.id) AS order_count
FROM users u
LEFT JOIN orders o ON u.id = o.user_id
WHERE u.active = true
GROUP BY u.name
HAVING COUNT(o.id) > 5
ORDER BY order_count DESC
LIMIT 10
```

The resulting tree (read bottom-up):

```
Limit(count=10)
  └─ Sort(order_count DESC)
       └─ Projection(u.name, COUNT(o.id) AS order_count)
            └─ Selection(COUNT(o.id) > 5)                    -- HAVING
                 └─ Aggregation(group_by=[u.name], aggs=[COUNT(o.id)])
                      └─ Selection(u.active = true)           -- WHERE
                           └─ Join(LEFT, ON u.id = o.user_id)
                                ├─ Alias("u")
                                │    └─ Scan(users)
                                └─ Alias("o")
                                     └─ Scan(orders)
```

In this example, `ORDER BY` uses a visible alias (`order_count`), so no hidden sort column is needed and the final output projection step is elided.

### Validation During Construction

Semantic validation is performed **during** tree construction, not as a separate pass:

- **Unknown table/column** — detected when resolving references against the Catalog
- **Aggregate in WHERE** — detected when building the Selection node for WHERE
- **Window function in WHERE/HAVING** — detected when building scalar expressions
- **Aggregate / window detection scope** — aggregate usage is collected across `SELECT`, `HAVING`, `ORDER BY`; window usage across `SELECT`, `ORDER BY`
- **Non-aggregated column in SELECT with GROUP BY** — detected when building the Aggregation/Projection
- **ORDER BY / DISTINCT restriction** — if `DISTINCT` is present, `ORDER BY` must reference visible output expressions (or ordinal/alias equivalents)
- **ORDER BY hidden-column planning** — if `DISTINCT` is absent, non-output `ORDER BY` expressions are materialized as hidden projection columns and trimmed in final projection
- **ORDER BY alias ambiguity** — duplicate visible aliases make alias-based ORDER BY binding invalid
- **Ambiguous column reference** — detected when resolving unqualified column names against multiple input relations

## Infer: Operator Semantic Rules

Each relational operator defines **deterministic rules** for how it transforms input metadata into output metadata. The Infer phase is a simple recursive traversal that applies these rules bottom-up.

### Metadata Model

```
RelationalMetadata = {
    columns: [ColumnMetadata],    -- output schema
    cardinality: Cardinality      -- row count bounds
}

ColumnMetadata = {
    table: String?,               -- originating table (for qualified references)
    name: String,                 -- column name
    data_type: DataType,          -- column data type
    nullable: bool                -- whether the column can be NULL
}

Cardinality = {
    min: MinRows,                -- lower bound
    max: MaxRows                 -- upper bound
}

MinRows = Zero | One
MaxRows = Zero | One | Many
    -- invariant: min <= max

Aliases:
    ExactlyZero = [Zero, Zero]
    ExactlyOne  = [One, One]
    AtMostOne   = [Zero, One]
    OneOrMore   = [One, Many]
    ZeroOrMore  = [Zero, Many]
```

### Inference Pseudocode

```
infer(expr):
    match expr:
        Scan(table)                  → infer_scan(table)
        Values(rows)                 → infer_values(rows)
        Selection(input, cond)       → infer_selection(infer(input), cond)
        Projection(input, cols)      → infer_projection(infer(input), cols)
        Aggregation(input, gb, aggs) → infer_aggregation(infer(input), gb, aggs)
        Window(input, exprs)         → infer_window(infer(input), exprs)
        Distinct(input)              → infer_distinct(infer(input))
        Sort(input, keys)            → infer_sort(infer(input))
        Limit(input, count, offset)  → infer_limit(infer(input), count, offset)
        Alias(input, name)           → infer_alias(infer(input), name)
        Join(left, right, kind, cond)→ infer_join(infer(left), infer(right), kind, cond)
        SetOperation(left, right, op, all)→ infer_set_op(infer(left), infer(right), op, all)
```

### Per-Operator Rules

#### Leaf Operators

**Scan(table)**
- Schema: columns from the Catalog's table definition
- Nullability: from the Catalog (column's NOT NULL constraint)
- Cardinality: `ZeroOrMore` (a base table may contain any number of rows)

**Values(rows)**
- Schema: inferred per column by combining all rows (common type / type promotion across row values)
- Nullability: inferred per column across all rows (`nullable = OR(row_i_col_nullable)`)
- Cardinality: `ExactlyZero` if empty, `ExactlyOne` if one row, `OneOrMore` if multiple rows

#### Unary Operators

**Selection(input, condition)** — σ
- Schema: = input (filtering does not change columns)
- Nullability: = input
- Cardinality: depends on condition analysis (see Cardinality Analysis section)

**Projection(input, columns)** — π
- Schema: from the projection list (each column's name and inferred type)
- Nullability: inferred per expression (see Scalar Expression Type Inference)
- Cardinality: = input (projection does not change row count)

**Aggregation(input, group_by, aggregates)** — γ
- Schema: group-by columns (pass-through from input) + aggregate result columns
- Nullability: group-by columns retain input nullability; aggregate results follow function-specific rules
- Cardinality: if `group_by` is empty → `ExactlyOne`; otherwise → `= input`

**Window(input, window_exprs)** — ω
- Schema: = input (window functions do not remove or reorder columns; they add computed columns)
- Nullability: = input for pass-through columns; window function results follow function-specific rules
- Cardinality: = input (window functions do not change row count)

**Distinct(input)** — δ
- Schema: = input
- Nullability: = input
- Cardinality: = input (conservative; deduplication never increases row count)

**Sort(input, keys)** — τ
- Schema: = input
- Nullability: = input
- Cardinality: = input (sorting does not change row count)

**Limit(input, count, offset)** — λ
- Schema: = input
- Nullability: = input
- Cardinality: see Limit Cardinality Rules section

**Alias(input, name)** — ρ
- Schema: = input, but all columns are relabeled with the new table name
- Nullability: = input
- Cardinality: = input

#### Binary Operators

**Join(left, right, kind, condition)** — ⋈

Schema:
- `left ++ right` for all join kinds (except `JOIN USING`, see dedicated section below)

Cardinality (interval model):
- **Inner**
  - `min = One` iff (`guaranteed_match_from_left && left.min == One`) OR (`guaranteed_match_from_right && right.min == One`); otherwise `Zero`
  - `max = upper_mul(left.max, right.max)`
  - if `at_most_one_right_per_left`, then `max = upper_min(max, left.max)`
  - if `at_most_one_left_per_right`, then `max = upper_min(max, right.max)`
- **Left**
  - `min = left.min`
  - `max = left.max` if (`right.max == Zero` OR `at_most_one_right_per_left`), else `upper_mul(left.max, right.max)`
- **Right**
  - `min = right.min`
  - `max = right.max` if (`left.max == Zero` OR `at_most_one_left_per_right`), else `upper_mul(left.max, right.max)`
- **Full**
  - `min = lower_or(left.min, right.min)`
  - `max = right.max` if `left.max == Zero`
  - `max = left.max` if `right.max == Zero`
  - otherwise `max = Many`
- **Cross**
  - `min = lower_and(left.min, right.min)`
  - `max = upper_mul(left.max, right.max)`

Nullability:
- `Inner/Cross`: preserve side nullability.
- `Left/Right/Full`: follow outer-join nullability forcing rules (see Join Nullability Refinement).

**SetOperation(left, right, op, all)** — ∪ ∩ −

Schema and nullability:
- schema comes from the left side (after compatibility checks)
- `Union All / Union` (distinct): nullability is merged (`nullable = left.nullable OR right.nullable`)
- `Intersect All / Intersect` (distinct): nullability is merged (`nullable = left.nullable OR right.nullable`)
- `Except All / Except` (distinct): nullability is inherited from left

Cardinality:
- `Union All`: `min = lower_or(left.min, right.min)`, `max = upper_add(left.max, right.max)`
- `Union` (distinct): same lower bound; conservative upper bound `upper_add(left.max, right.max)`
- `Intersect All`: `min = Zero`, `max = upper_min(left.max, right.max)`
- `Intersect` (distinct): `min = Zero`, `max = upper_min(left.max, right.max)`
- `Except All`: `max = left.max`, `min = left.min` if `right.max == Zero` else `Zero`
- `Except` (distinct): `max = left.max`, `min = left.min` if `right.max == Zero` else `Zero`

## Cardinality Analysis

Cardinality represents the **row count bounds** of a query result. It is critical for downstream code generation — a query guaranteed to return at most one row can be mapped to a scalar value instead of a collection.

### Cardinality Domain

Cardinality is represented as an interval:

```
[min, max]
min ∈ {Zero, One}
max ∈ {Zero, One, Many}
constraint: min <= max
```

Canonical aliases:

```
ExactlyZero = [Zero, Zero]
ExactlyOne  = [One, One]
AtMostOne   = [Zero, One]
OneOrMore   = [One, Many]
ZeroOrMore  = [Zero, Many]
```

### Cardinality Combinators

```
drop_lower_bound([m, M]) = [Zero, M]

constrain_at_most_one([m, M]) = [m, min(M, One)]

upper_min(a, b):   max-domain minimum (Zero < One < Many)
upper_add(a, b):   Zero+Zero=Zero, Zero+One=One, One+One=Many, Many+x=Many
upper_mul(a, b):   Zero*x=Zero, One*One=One, otherwise Many
lower_or(a, b):    One if either side is One, else Zero
lower_and(a, b):   One only if both sides are One, else Zero
```

### Selection Cardinality Rules

The Selection operator (WHERE) is the most interesting operator for cardinality analysis. By analyzing the predicate structure, we can determine whether the filter guarantees at most one row:

```
Condition Pattern                                  Effect
──────────────────────────────────────────────────────────────────────
WHERE false / WHERE 1=0                            → ExactlyZero
WHERE all columns of an input primary key are equality-constrained       → AtMostOne
WHERE all columns of an input unique key are equality-constrained         → AtMostOne
WHERE full input key IN (<single tuple>)                                  → AtMostOne
WHERE a full proven-NOT-NULL input key is constrained to NULL             → ExactlyZero
WHERE key column IS NULL with nullable/unknown key nullability            → no refinement
WHERE c1 = ? AND c2 = ? (AND covers a full input PK/UK key)              → AtMostOne
WHERE c = v AND c = w (v != w)                     → ExactlyZero
WHERE ... OR ...                                   → no refinement (conservative)
Other                                              → no refinement
```

The analysis works by:
1. Extracting equality constraints from the condition (column = value pairs)
2. Extracting single-tuple `IN` constraints on full keys
3. Collecting constrained columns across AND conjuncts
4. Detecting contradiction patterns (for example `c = v AND c = w`, `v != w`) and impossible null checks on proven non-null keys
5. Checking if constrained columns **cover** a full primary/unique constraint that is valid for the current input relation
6. If covered → the result is at most one row

`input_constraints` denotes the primary/unique constraints that are valid for the current input relation at this point in the relational tree (derived from Catalog constraints and relational-operator semantics).

```
analyze_selection_cardinality(input_cardinality, condition, input_constraints):

    if is_always_false(condition):
        return ExactlyZero

    // Step 1: contradiction and impossible-predicate checks
    if has_contradictory_equalities(condition):
        return ExactlyZero

    if has_full_key_is_null_on_proven_non_nullable_key(condition, input_constraints):
        return ExactlyZero

    // Conservative rule: do not refine through OR disjunctions
    if contains_or(condition):
        return input_cardinality

    // Step 2: gather key constraints from equality and single-tuple IN
    eq_columns = extract_equality_columns(condition)            // columns constrained by "="
    in_single_tuple_keys = extract_single_tuple_in_keys(condition) // key columns constrained by IN(single tuple)
    constrained_columns = union(eq_columns, in_single_tuple_keys)

    // Step 3: full primary-key coverage implies at-most-one
    if covers_any_primary_key(constrained_columns, input_constraints):
        return constrain_at_most_one(input_cardinality)

    // Step 4: full unique-key coverage implies at-most-one
    if covers_any_unique_key(constrained_columns, input_constraints):
        return constrain_at_most_one(input_cardinality)

    return input_cardinality  // no refinement possible
```

### Limit Cardinality Rules

```
LIMIT 0                → ExactlyZero (empty result)
LIMIT 1                → constrain_at_most_one(input)
LIMIT n (n > 1)        → input (no useful refinement)
OFFSET > 0             → ExactlyZero if input.max <= One; otherwise drop_lower_bound(input)
LIMIT 1 + OFFSET > 0   → ExactlyZero if input.max <= One; otherwise AtMostOne
```

## Join Nullability Refinement

### Base Rules

```
Join Kind    Nullability Transformation
──────────────────────────────────────────────────────────────
Inner        preserve both sides
Cross        preserve both sides
Left         preserve left, force right nullable
Right        preserve right, force left nullable
Full         force both sides nullable
```

### Guaranteed-Match Optimization (Strict)

Outer-join nullability forcing can be skipped only when **all** conditions hold:
1. `ON` is a pure conjunction of equi-join predicates (`a.col = b.col`).
2. The equi-join pairs cover a full FK mapping from the preserved side to the nullable side.
3. All FK columns on the preserved side are `NOT NULL`.
4. The referenced side columns covered by those pairs form a full `PRIMARY KEY` or `UNIQUE` key.
5. No extra `ON` predicate exists beyond those equi-key predicates.

If all hold:
- `LEFT JOIN`: keep right-side original nullability (no forced nullable).
- `RIGHT JOIN`: symmetric for left side.
- `FULL JOIN`: no optimization (still force both sides nullable).

Pseudocode:

```
guaranteed_match_from_left(on, left, right, catalog):
    if !is_pure_equi_join_conjunction(on):
        return false
    // reject any ON predicate beyond equi-join key pairs
    if has_extra_on_predicates(on):
        return false

    pairs = extract_equijoin_pairs(on)
    if !covers_full_fk_not_null(left -> right, pairs, catalog):
        return false

    if !covers_full_unique_or_pk(right, pairs, catalog):
        return false

    return true
```

### Companion Predicates for Join Cardinality

```
at_most_one_right_per_left(on, left, right, catalog):
    if !is_pure_equi_join_conjunction(on):
        return false
    pairs = extract_equijoin_pairs(on)
    return covers_full_unique_or_pk_not_null(right, pairs, catalog)
```

`guaranteed_match_from_right` and `at_most_one_left_per_right` are defined symmetrically by swapping left/right roles in the same predicates.

### JOIN USING Column Coalescing

`JOIN USING(col)` outputs only one copy of each `USING` column pair.

Nullability of the merged output column:
- `INNER USING`: conservative baseline `left.nullable OR right.nullable` (can be tightened by equivalence/constraint propagation in later refinements)
- `LEFT USING`: `left.nullable`
- `RIGHT USING`: `right.nullable`
- `FULL USING`: `left.nullable OR right.nullable`

## Scalar Expression Type Inference

Every `ScalarExpr` node has two inferred properties: **data type** and **nullability**. These are computed bottom-up through the expression tree.

### Inference Rules by Expression Kind

```
Expression              Data Type                       Nullable
──────────────────────────────────────────────────────────────────────────
ColumnRef(t, c)         from catalog/input schema       from catalog/input schema
Literal(NULL)           Unknown                         true
Literal(Bool/Int/...)   from literal type               false
BinaryOp(l, op, r)     from operator rules              nullable(l) OR nullable(r)
UnaryOp(NOT, e)         Bool                            nullable(e)
UnaryOp(-, e)           type(e)                         nullable(e)
Function(name, args)    from function registry           from function registry
AggregateCall(name, ..) from function registry           from function registry
WindowCall(name, ...)   from function registry           function-specific and frame-aware
Cast(e, target)         target                          nullable(e)
IsNull(e)               Bool                            false (IS NULL never returns NULL)
InList(e, list)         Bool                            nullable(e) OR any nullable in list
InSubquery(e, q)        Bool                            nullable(e) OR nullable(subquery output expression)
Between(e, lo, hi)      Bool                            nullable(e) OR nullable(lo) OR nullable(hi)
Case(...)               see below                       see below
ScalarSubquery(q)       from subquery's single column   true (subquery may return no rows)
Exists(q)               Bool                            false
Wildcard / QualifiedWildcard  expanded during Algebraize projection construction
```

### CASE Expression Rules

```
CASE WHEN c1 THEN r1 WHEN c2 THEN r2 ELSE e END

Data type:  common_type(r1, r2, e)  -- type promotion across all branches
Nullable:   nullable(r1) OR nullable(r2) OR nullable(e)
            OR (no ELSE clause → implicitly ELSE NULL → always nullable)
```

## Function System

Functions are a **shared concern** between the Algebraize and Infer phases. The Algebraize phase needs to classify functions (scalar vs aggregate vs window) and validate arity. The Infer phase needs to determine return types and nullability.

### Function Categories

```
Category      Examples                          Context
────────────────────────────────────────────────────────────────────
Scalar        UPPER, LOWER, COALESCE, IF,       Anywhere in expressions
              CONCAT, ABS, ROUND, CAST, ...

Aggregate     COUNT, SUM, AVG, MIN, MAX,        Only in Aggregation or
              GROUP_CONCAT, ARRAY_AGG, ...       with OVER (becomes window)

Window        ROW_NUMBER, RANK, DENSE_RANK,     Only with OVER clause
              NTILE, LAG, LEAD, FIRST_VALUE, ...
```

### Resolution Priority

When resolving a function name, the priority order is:

1. **Window functions** — if the function has an OVER clause and matches a known window function
2. **Aggregate-as-window** — if the function has an OVER clause and matches a known aggregate, classify it as window
3. **Aggregate functions** — if the function matches a known aggregate without OVER and appears in an aggregate-valid context (using signature/context checks)
4. **Scalar functions** — if the function matches a known scalar function
5. **Unknown** — fallback for unrecognized functions

### Aggregate-as-Window

When an aggregate function (for example `SUM`) is used with an `OVER` clause, it is treated as a window expression. Its nullability is **not universally always-nullable**; it is function-specific and frame-aware.

Safe baseline rules:
- Ranking/distribution window functions (for example `ROW_NUMBER`, `RANK`, `DENSE_RANK`, `NTILE`) are non-null for each produced row.
- `COUNT(...) OVER (...)` is non-null.
- Other window aggregates (`SUM/AVG/MIN/MAX/...`) are nullable when an empty frame or function semantics can produce `NULL`.
- Value window functions (`LAG/LEAD/FIRST_VALUE/LAST_VALUE/NTH_VALUE`) use function-specific nullability rules (arguments, defaults, and frame behavior).

### Function Type Inference

Each function defines:

```
function_def = {
    name: String,
    arity: Exact(n) | Range(min, max) | AtLeast(n) | Any,
    return_type: Fixed(type) | SameAsArg(index) | NumericPromotion | Custom(fn),
    nullability: AnyArgNullable | AlwaysNullable | NeverNullable | Custom(fn)
}
```

Common nullability patterns:

```
Pattern             Meaning                              Examples
────────────────────────────────────────────────────────────────────
AnyArgNullable      nullable if ANY argument is nullable  UPPER(x), ABS(x), x + y
AlwaysNullable      always nullable regardless of args    nullable-only implementation-specific functions
NeverNullable       never nullable regardless of args     COUNT, EXISTS
Custom              function-specific logic               IF, CASE, IFNULL, COALESCE
```
