# Static Analyzer Architecture

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

SQL text is parsed into an Abstract Syntax Tree (AST). This phase uses an external SQL parser with dialect-specific support (MySQL, PostgreSQL, SQLite). The AST preserves the syntactic structure of the SQL statement.

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
- Expand wildcards (`*`, `t.*`) into concrete column lists using catalog metadata
- Validate function argument types (requires type information computed during inference)

**Not responsible for**:
- Name resolution (all references are already resolved in the RelationalExpr tree)
- Semantic validation (the tree is assumed to be structurally valid)
- Modifying the RelationalExpr tree (inference is read-only)

**Key principle**: The Infer phase is a **pure function** from RelationalExpr to metadata. It does not modify the tree, does not report semantic errors, and does not resolve names. Each operator has deterministic rules that derive output metadata from input metadata.

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

    // Wildcards (for SELECT * and COUNT(*))
    | Wildcard
    | QualifiedWildcard(table)
```

### Key Design Decision: Aggregates in ScalarExpr

Aggregate functions (COUNT, SUM, ...) appear as `AggregateCall` nodes within `ScalarExpr` because they are syntactically embedded in SQL expressions (e.g., `SELECT price * COUNT(*)`). However, during the Algebraize phase, the presence of aggregate calls triggers the creation of an `Aggregation` operator in the relational tree. The Aggregation operator is the **relational-level** representation of grouping; the `AggregateCall` in ScalarExpr is the **scalar-level** reference to an aggregate computation.

## Algebraize: SQL to Relational Algebra Conversion

### SQL's Logical Execution Order

SQL has a well-defined logical execution order that differs from its syntactic order. The Algebraize phase builds the RelationalExpr tree **bottom-up** following this logical order:

```
Step    SQL Clause        Relational Operator     What it does
────────────────────────────────────────────────────────────────────────
1st     FROM / JOIN       Scan, Join, Alias       Establish input relations
2nd     WHERE             Selection               Filter rows before grouping
3rd     GROUP BY + aggs   Aggregation             Group and aggregate
4th     HAVING            Selection               Filter groups
5th     Window funcs      Window                  Compute window functions
6th     SELECT            Projection              Choose / compute output columns
7th     DISTINCT          Distinct                Eliminate duplicates
8th     ORDER BY          Sort                    Order result rows
9th     LIMIT / OFFSET    Limit                   Restrict output row count
```

### Conversion Pseudocode

```
algebraize_select(query):
    // Step 1: FROM → base relation
    expr = build_from(query.from)          // Scan, Join, Alias nodes

    // Step 2: WHERE → Selection
    if query.where:
        validate_no_aggregates(query.where)
        validate_no_window_functions(query.where)
        expr = Selection(expr, build_scalar(query.where))

    // Step 3: GROUP BY + aggregates → Aggregation
    if query.group_by or has_aggregates(query.select):
        validate_grouping_rules(query)
        expr = Aggregation(expr, build_group_by(query), build_projections(query))

    // Step 4: HAVING → Selection
    if query.having:
        expr = Selection(expr, build_scalar(query.having))

    // Step 5: Window functions → Window
    if has_window_functions(query.select):
        expr = Window(expr, build_window_exprs(query))

    // Step 6: SELECT → Projection
    expr = Projection(expr, build_projection_columns(query.select))

    // Step 7: DISTINCT → Distinct
    if query.distinct:
        expr = Distinct(expr)

    // Step 8: ORDER BY → Sort
    if query.order_by:
        expr = Sort(expr, build_sort_keys(query.order_by))

    // Step 9: LIMIT / OFFSET → Limit
    if query.limit or query.offset:
        expr = Limit(expr, query.limit, query.offset)

    return expr
```

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

### Validation During Construction

Semantic validation is performed **during** tree construction, not as a separate pass:

- **Unknown table/column** — detected when resolving references against the Catalog
- **Aggregate in WHERE** — detected when building the Selection node for WHERE
- **Window function in WHERE/HAVING** — detected when building scalar expressions
- **Non-aggregated column in SELECT with GROUP BY** — detected when building the Aggregation/Projection
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

Cardinality = ExactlyOne | AtMostOne | Unknown
    -- ExactlyOne: guaranteed exactly one row (e.g., SELECT 1, aggregate without GROUP BY)
    -- AtMostOne:  zero or one row (e.g., WHERE on primary key, LIMIT 1)
    -- Unknown:    zero or more rows (general case)
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
        SetOperation(left, right, op)→ infer_set_op(infer(left), infer(right), op)
```

### Per-Operator Rules

#### Leaf Operators

**Scan(table)**
- Schema: columns from the Catalog's table definition
- Nullability: from the Catalog (column's NOT NULL constraint)
- Cardinality: `Unknown` (a table may have any number of rows)

**Values(rows)**
- Schema: inferred from the literal types in the first row
- Nullability: NULL literals → nullable; non-NULL literals → not nullable
- Cardinality: `ExactlyOne` if exactly one row; `Unknown` otherwise

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
- Cardinality: if `group_by` is empty → `ExactlyOne`; otherwise → `Unknown`

**Window(input, window_exprs)** — ω
- Schema: = input (window functions do not remove or reorder columns; they add computed columns)
- Nullability: = input for pass-through columns; window function results follow function-specific rules
- Cardinality: = input (window functions do not change row count)

**Distinct(input)** — δ
- Schema: = input
- Nullability: = input
- Cardinality: `drop_lower_bound(input)` — deduplication may reduce rows but never increase them

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

The Join operator combines two relations. Its behavior depends on the join kind:

```
Join Kind    Schema          Nullability                     Cardinality
─────────────────────────────────────────────────────────────────────────────
Inner        left ++ right   left ++ right                   drop_lower_bound(left)
Left         left ++ right   left ++ make_nullable(right)    = left
Right        left ++ right   make_nullable(left) ++ right    = right
Full         left ++ right   make_nullable(left ++ right)    max(left, right)
Cross        left ++ right   left ++ right                   drop_lower_bound(left)
```

Where:
- `left ++ right` means concatenating the column lists from both sides
- `make_nullable(cols)` forces all columns to be nullable (because unmatched rows produce NULLs)
- `drop_lower_bound` means ExactlyOne → Unknown (inner join may produce 0 or many rows)

**SetOperation(left, right, op, all)** — ∪ ∩ −

```
Set Op       Schema       Nullability                  Cardinality
──────────────────────────────────────────────────────────────────────
Union        from left    merge(left, right)           max(left, right)
Intersect    from left    merge(left, right)           min(left, right)
Except       from left    = left                       = left
```

Where:
- `merge(left, right)` means: a column is nullable if it is nullable on **either** side
- `min/max` on cardinality: `min(ExactlyOne, Unknown) = ExactlyOne`, `max(AtMostOne, Unknown) = Unknown`

## Cardinality Analysis

Cardinality represents the **row count bounds** of a query result. It is critical for downstream code generation — a query guaranteed to return at most one row can be mapped to a scalar value instead of a collection.

### Cardinality Lattice

```
ExactlyOne  ⊂  AtMostOne  ⊂  Unknown

ExactlyOne  — guaranteed exactly one row (e.g., aggregate without GROUP BY)
AtMostOne   — zero or one row (e.g., WHERE on full primary key, LIMIT 1)
Unknown     — zero or more rows (general case)
```

### Cardinality Combinators

```
drop_lower_bound(ExactlyOne) = Unknown      -- may produce 0 or many rows
drop_lower_bound(AtMostOne)  = AtMostOne    -- already allows 0
drop_lower_bound(Unknown)    = Unknown

constrain_at_most_one(ExactlyOne) = ExactlyOne
constrain_at_most_one(AtMostOne)  = AtMostOne
constrain_at_most_one(Unknown)    = AtMostOne

max(ExactlyOne, ExactlyOne) = ExactlyOne
max(ExactlyOne, AtMostOne)  = AtMostOne
max(ExactlyOne, Unknown)    = Unknown
max(AtMostOne, Unknown)     = Unknown

min(ExactlyOne, Unknown)    = ExactlyOne
min(AtMostOne, Unknown)     = AtMostOne
```

### Selection Cardinality Rules

The Selection operator (WHERE) is the most interesting operator for cardinality analysis. By analyzing the predicate structure, we can determine whether the filter guarantees at most one row:

```
Condition Pattern                                  Effect
──────────────────────────────────────────────────────────────────────
WHERE false / WHERE 1=0                            → AtMostOne
WHERE pk_col = <value> (all PK columns covered)    → AtMostOne
WHERE unique_col = <value> (all UK columns covered) → AtMostOne
WHERE pk_col IN (<single_value>)                   → AtMostOne
WHERE pk_col IS NULL (PK is NOT NULL by definition) → AtMostOne
WHERE c1 = ? AND c2 = ? (AND covers full PK/UK)   → AtMostOne
WHERE ... OR ...                                   → no refinement (conservative)
Other                                              → no refinement
```

The analysis works by:
1. Extracting equality constraints from the condition (column = value pairs)
2. Collecting all constrained columns across AND conjuncts
3. Checking if the constrained columns **cover** a primary key or unique constraint
4. If covered → the result is at most one row

```
analyze_selection_cardinality(input_cardinality, condition, catalog):
    if is_always_false(condition):
        return AtMostOne

    eq_columns = extract_equality_columns(condition)  // {col1, col2, ...}

    for each primary_key in catalog:
        if eq_columns ⊇ primary_key.columns:
            return constrain_at_most_one(input_cardinality)

    for each unique_constraint in catalog:
        if eq_columns ⊇ unique_constraint.columns:
            return constrain_at_most_one(input_cardinality)

    return input_cardinality  // no refinement possible
```

### Limit Cardinality Rules

```
LIMIT 0                → AtMostOne (empty result)
LIMIT 1                → constrain_at_most_one(input)
LIMIT n (n > 1)        → input (no useful refinement)
OFFSET > 0             → drop_lower_bound(input) (offset may skip all rows)
LIMIT 1 + OFFSET > 0   → constrain_at_most_one(drop_lower_bound(input))
```

## Join Nullability Refinement

### The General Rule

In a LEFT JOIN, all columns from the right side are forced nullable — because unmatched rows produce NULLs. Similarly for RIGHT JOIN (left side nullable) and FULL JOIN (both sides nullable).

### Foreign Key Optimization

There is an important exception: if a **foreign key constraint** guarantees that every row on the FK side has a matching row on the PK side, then the join-side columns need not be forced nullable.

```
Given:  orders.user_id REFERENCES users(id)
        orders.user_id is NOT NULL

Query:  SELECT * FROM orders LEFT JOIN users ON orders.user_id = users.id

Analysis:
  - Every order has a non-NULL user_id (NOT NULL constraint)
  - Every user_id references a valid users.id (FK constraint)
  - Therefore: every order row WILL match a users row
  - Therefore: users columns need NOT be forced nullable
```

Pseudocode:

```
fk_guarantees_match(join_condition, left_table, right_table, catalog):
    // Extract the join column pairs from the ON condition
    (left_col, right_col) = extract_equijoin_columns(join_condition)

    // Check if there's a FK from left to right (or right to left)
    fk = catalog.find_foreign_key(from=left_table.left_col, to=right_table.right_col)
    if fk exists AND left_col is NOT NULL:
        return true  // FK guarantees a match for every row

    return false
```

### JOIN USING Column Coalescing

When a JOIN uses `USING(col)`, the SQL standard specifies that the USING columns appear only once in the output (not duplicated from both sides). The right side's copy is excluded from the output schema, and the left side's copy is kept.

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
WindowCall(name, ...)   from function registry           typically true (frame may be empty)
Cast(e, target)         target                          nullable(e)
IsNull(e)               Bool                            false (IS NULL never returns NULL)
InList(e, list)         Bool                            nullable(e) OR any nullable in list
Between(e, lo, hi)      Bool                            nullable(e) OR nullable(lo) OR nullable(hi)
Case(...)               see below                       see below
ScalarSubquery(q)       from subquery's single column   true (subquery may return no rows)
Exists(q)               Bool                            false
Wildcard / QualifiedWildcard  expanded during Projection
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
2. **Aggregate functions** — if the function matches a known aggregate
3. **Scalar functions** — if the function matches a known scalar function
4. **Unknown** — fallback for unrecognized functions

### Aggregate-as-Window

When an aggregate function (e.g., SUM) is used with an OVER clause, it becomes a window function. This changes its nullability semantics — window aggregates are **always nullable** because the window frame may be empty, unlike regular aggregates which always produce a value (e.g., COUNT returns 0, not NULL, for empty groups).

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
AlwaysNullable      always nullable regardless of args    LAG, LEAD, FIRST_VALUE
NeverNullable       never nullable regardless of args     COUNT, EXISTS, COALESCE*
Custom              function-specific logic               IF, CASE, IFNULL

* COALESCE is never nullable only if at least one argument is not nullable
```
