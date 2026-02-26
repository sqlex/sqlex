# Static Analyzer Error Codes

This document defines the **error code convention** used by `sqlex-static-analyzer`
and provides the complete code list extracted from `src/**/*.rs`, including
the corresponding **error reason** for each code.

## 1. Rendering Format

Diagnostics are rendered as:

```text
[CODE] [PHASE] message
```

Rendering implementation:

- `src/diagnostics.rs` (`Diagnostic::into_execution_error` / `Diagnostic::into_analysis_error`)

## 2. Phase and Code Namespace

Phase token mapping (`Phase` enum in `src/diagnostics.rs`):

1. `PARSE` -> `Pxxxx`
2. `CATALOG` -> `Cxxxx`
3. `ALGEBRAIZE` -> `Axxxx`
4. `INFER` -> `Ixxxx`

Current numbering bands in this crate:

1. Parse: `P1xxx`
2. Catalog: `C2xxx`
3. Algebraize: `A3xxx`
4. Infer: `I4xxx`

## 3. Usage Rules

1. Allocate a new unique code under the correct phase prefix.
2. Emit diagnostics via `Diagnostic::new(code, phase, message)`.
3. Keep code stable once released; evolve only the message text if needed.
4. Add/adjust spec assertions (`expected_error_code`) when behavior changes.

## 4. Complete Code List (Extracted from Source)

Extraction scope:

- `crates/analyzer/sqlex-static-analyzer/src/**/*.rs`
- `Diagnostic::new(...)` calls
- Dynamic code binding is expanded for `I4108`/`I4109`

Columns:

1. `Code`: diagnostic code.
2. `Phase`: phase derived from code prefix.
3. `Reason`: normalized reason text from diagnostic message templates.
4. `Primary Source`: earliest source location where the code appears.
5. `Occurrences`: number of `Diagnostic::new` occurrences in `src`.

## 5. Summary

- Total unique codes: 103
- PARSE (`P`): 4
- CATALOG (`C`): 26
- ALGEBRAIZE (`A`): 61
- INFER (`I`): 12

## 6.1 PARSE Codes

| Code | Phase | Reason | Primary Source | Occurrences |
| --- | --- | --- | --- | --- |
| `P1001` | `PARSE` | failed to parse SQL: {err} | `src/parser.rs:16` | 1 |
| `P1002` | `PARSE` | empty SQL is not allowed | `src/parser.rs:19` | 1 |
| `P1003` | `PARSE` | analyze expects exactly one SQL statement | `src/parser.rs:33` | 1 |
| `P1004` | `PARSE` | analyze only supports SELECT/WITH query statements | `src/parser.rs:44` | 1 |

## 6.2 CATALOG Codes

| Code | Phase | Reason | Primary Source | Occurrences |
| --- | --- | --- | --- | --- |
| `C2001` | `CATALOG` | unsupported statement in execute: {} | `src/catalog/mutator.rs:51` | 1 |
| `C2002` | `CATALOG` | CREATE TABLE AS SELECT is not supported in this iteration | `src/catalog/mutator.rs:65` | 1 |
| `C2003` | `CATALOG` | table '{}' already exists | `src/catalog/mutator.rs:79` | 1 |
| `C2004` | `CATALOG` | duplicate column '{}' in table '{}' | `src/catalog/mutator.rs:93` | 1 |
| `C2005` | `CATALOG` | failed to add table: {message} | `src/catalog/mutator.rs:137` | 1 |
| `C2006` | `CATALOG` | table '{}' not found for ALTER TABLE | `src/catalog/mutator.rs:159` | 1 |
| `C2007` | `CATALOG` | column '{}' already exists in '{}' | `src/catalog/mutator.rs:179` | 1 |
| `C2008` | `CATALOG` | column '{}' not found in table '{}' | `src/catalog/mutator.rs:204` | 1 |
| `C2009` | `CATALOG` | unsupported ALTER TABLE operation in this iteration | `src/catalog/mutator.rs:226` | 1 |
| `C2010` | `CATALOG` | DROP only supports TABLE in this iteration | `src/catalog/mutator.rs:246` | 1 |
| `C2011` | `CATALOG` | table '{}' does not exist | `src/catalog/mutator.rs:259` | 1 |
| `C2012` | `CATALOG` | cannot drop table '{}': referenced by foreign key constraints | `src/catalog/mutator.rs:267` | 1 |
| `C2013` | `CATALOG` | failed to drop table: {message} | `src/catalog/mutator.rs:278` | 1 |
| `C2014` | `CATALOG` | {label} must contain at least one column | `src/catalog/mutator.rs:451` | 1 |
| `C2015` | `CATALOG` | column '{}' not found in table '{}' | `src/catalog/mutator.rs:462` | 1 |
| `C2016` | `CATALOG` | column '{}' repeated in {} | `src/catalog/mutator.rs:472` | 1 |
| `C2017` | `CATALOG` | foreign key in '{}' has no local columns | `src/catalog/mutator.rs:521` | 1 |
| `C2018` | `CATALOG` | foreign key in '{}' has mismatched local/referenced column counts | `src/catalog/mutator.rs:528` | 1 |
| `C2019` | `CATALOG` | foreign key references missing local column '{}' in '{}' | `src/catalog/mutator.rs:539` | 1 |
| `C2020` | `CATALOG` | foreign key references unknown table '{}' from '{}' | `src/catalog/mutator.rs:557` | 1 |
| `C2021` | `CATALOG` | foreign key references unknown column '{}.{}' | `src/catalog/mutator.rs:569` | 1 |
| `C2022` | `CATALOG` | cannot drop column '{}.{}': used by primary key | `src/catalog/mutator.rs:597` | 1 |
| `C2023` | `CATALOG` | cannot drop column '{}.{}': used by unique key | `src/catalog/mutator.rs:612` | 1 |
| `C2024` | `CATALOG` | cannot drop column '{}.{}': used by foreign key | `src/catalog/mutator.rs:627` | 1 |
| `C2025` | `CATALOG` | cannot drop column '{}.{}': referenced by other foreign keys | `src/catalog/mutator.rs:646` | 1 |
| `C2026` | `CATALOG` | SQLite does not support ALTER TABLE ADD CONSTRAINT | `src/catalog/mutator.rs:217` | 1 |

## 6.3 ALGEBRAIZE Codes

| Code | Phase | Reason | Primary Source | Occurrences |
| --- | --- | --- | --- | --- |
| `A3001` | `ALGEBRAIZE` | only query statements are supported in analyze | `src/algebraizer/mod.rs:56` | 1 |
| `A3002` | `ALGEBRAIZE` | unknown qualified wildcard target: {qualifier_name} | `src/algebraizer/relation/select.rs:157` | 1 |
| `A3003` | `ALGEBRAIZE` | table not found: {normalized_table_name} | `src/algebraizer/relation/from_table_factor.rs:43` | 1 |
| `A3004` | `ALGEBRAIZE` | empty compound identifier | `src/algebraizer/expression/mod.rs:29` | 1 |
| `A3005` | `ALGEBRAIZE` | invalid floating literal '{number}': {err} | `src/algebraizer/expression/literal.rs:33` | 1 |
| `A3006` | `ALGEBRAIZE` | invalid integer literal '{number}': {err} | `src/algebraizer/expression/literal.rs:42` | 1 |
| `A3008` | `ALGEBRAIZE` | column not found: {column_name} | `src/algebraizer/expression/column.rs:53` | 2 |
| `A3009` | `ALGEBRAIZE` | ambiguous column reference: {column_name} / ambiguous column reference: {qualifier}.{column_name} | `src/algebraizer/expression/column.rs:132` | 3 |
| `A3010` | `ALGEBRAIZE` | ambiguous relation reference: {qualifier} / unknown relation reference: {qualifier} | `src/algebraizer/expression/column.rs:106` | 2 |
| `A3011` | `ALGEBRAIZE` | column not found: {qualifier}.{column_name} | `src/algebraizer/expression/column.rs:74` | 2 |
| `A3012` | `ALGEBRAIZE` | empty compound identifier in projection | `src/algebraizer/expression/column.rs:187` | 1 |
| `A3013` | `ALGEBRAIZE` | derived table alias column count mismatch: expected {}, got {} | `src/algebraizer/relation/from_table_factor.rs:99` | 1 |
| `A3014` | `ALGEBRAIZE` | CTE column alias count mismatch: expected {}, got {} | `src/algebraizer/relation/cte.rs:74` | 2 |
| `A3015` | `ALGEBRAIZE` | recursive CTE column alias count mismatch: expected {}, got {} | `src/algebraizer/relation/cte.rs:189` | 1 |
| `A3016` | `ALGEBRAIZE` | aggregate expression is not allowed in GROUP BY | `src/algebraizer/relation/select.rs:81` | 1 |
| `A3017` | `ALGEBRAIZE` | non-aggregated projection is not allowed when GROUP BY is absent | `src/algebraizer/relation/select.rs:264` | 1 |
| `A3018` | `ALGEBRAIZE` | invalid {clause_name} value '{number}': {error} | `src/algebraizer/relation/query.rs:276` | 1 |
| `A3019` | `ALGEBRAIZE` | set operation column count mismatch: left {}, right {} | `src/algebraizer/relation/set_ops.rs:37` | 1 |
| `A3020` | `ALGEBRAIZE` | function '{}' expects at least {} argument(s), got {} | `src/algebraizer/expression/function.rs:254` | 1 |
| `A3021` | `ALGEBRAIZE` | function '{}' expects at most {} argument(s), got {} | `src/algebraizer/expression/function.rs:265` | 1 |
| `A3024` | `ALGEBRAIZE` | reserved keyword cannot be used as alias: {} | `src/algebraizer/expression/function.rs:373` | 1 |
| `A3025` | `ALGEBRAIZE` | duplicate CTE name: {cte_name} | `src/algebraizer/relation/cte.rs:30` | 3 |
| `A3026` | `ALGEBRAIZE` | recursive CTE term column count mismatch: seed {}, recursive {} | `src/algebraizer/relation/cte.rs:168` | 1 |
| `A3027` | `ALGEBRAIZE` | JOIN USING requires at least one shared column | `src/algebraizer/relation/join.rs:339` | 1 |
| `A3030` | `ALGEBRAIZE` | ORDER BY INTERPOLATE is not supported in this iteration | `src/algebraizer/relation/query.rs:52` | 1 |
| `A3031` | `ALGEBRAIZE` | ORDER BY WITH FILL is not supported in this iteration | `src/algebraizer/expression/function.rs:175` | 2 |
| `A3032` | `ALGEBRAIZE` | ORDER BY position starts from 1 / invalid ORDER BY position '{number}': {error} | `src/algebraizer/relation/query.rs:217` | 2 |
| `A3033` | `ALGEBRAIZE` | ORDER BY position {} is out of range for {} column(s) | `src/algebraizer/relation/query.rs:150` | 1 |
| `A3035` | `ALGEBRAIZE` | {usage} expects subquery to return exactly one column, got {} | `src/algebraizer/expression/subquery.rs:29` | 1 |
| `A3041` | `ALGEBRAIZE` | aggregate expression is not allowed in WHERE | `src/algebraizer/relation/select.rs:57` | 1 |
| `A3042` | `ALGEBRAIZE` | window expression is not allowed in WHERE | `src/algebraizer/relation/select.rs:64` | 1 |
| `A3043` | `ALGEBRAIZE` | window expression is not allowed in HAVING | `src/algebraizer/relation/select.rs:103` | 1 |
| `A3044` | `ALGEBRAIZE` | projection expression '{name}' must reference grouped columns or aggregates | `src/algebraizer/relation/select.rs:275` | 1 |
| `A3045` | `ALGEBRAIZE` | HAVING expression must reference grouped columns or aggregates | `src/algebraizer/relation/select.rs:290` | 1 |
| `A3046` | `ALGEBRAIZE` | duplicate WINDOW definition: {normalized_name} | `src/algebraizer/relation/select.rs:383` | 1 |
| `A3047` | `ALGEBRAIZE` | cyclic WINDOW definition: {name} | `src/algebraizer/relation/select.rs:419` | 1 |
| `A3048` | `ALGEBRAIZE` | unknown WINDOW definition: {name} / unknown WINDOW definition: {normalized_base} / unknown WINDOW definition: {normalized_name} | `src/algebraizer/expression/function.rs:72` | 3 |
| `A3049` | `ALGEBRAIZE` | ORDER BY expression must appear in SELECT list when DISTINCT semantics are active | `src/algebraizer/relation/query.rs:170` | 1 |
| `A3050` | `ALGEBRAIZE` | {clause_name} expects a non-negative integer literal | `src/algebraizer/relation/query.rs:286` | 1 |
| `A3051` | `ALGEBRAIZE` | advanced SELECT clauses are not supported in this algebraizer path | `src/algebraizer/relation/select.rs:36` | 1 |
| `A3052` | `ALGEBRAIZE` | GROUP BY form is not supported in this algebraizer path | `src/algebraizer/relation/select.rs:360` | 1 |
| `A3053` | `ALGEBRAIZE` | GLOBAL JOIN is not supported in this algebraizer path | `src/algebraizer/relation/join.rs:35` | 1 |
| `A3054` | `ALGEBRAIZE` | FULL JOIN is not supported for mysql | `src/algebraizer/relation/join.rs:214` | 1 |
| `A3055` | `ALGEBRAIZE` | JOIN operator is not supported for dialect {}: {:?} | `src/algebraizer/relation/join.rs:223` | 1 |
| `A3056` | `ALGEBRAIZE` | NATURAL JOIN is not supported in this algebraizer path | `src/algebraizer/relation/join.rs:243` | 1 |
| `A3057` | `ALGEBRAIZE` | internal algebraizer invariant violated: left slot {} not found in schema / internal algebraizer invariant violated: missing left USING slot {} / internal algebraizer invariant violated: missing right USING slot {} / internal algebraizer invariant violated: right slot {} not found in schema | `src/algebraizer/relation/join.rs:110` | 4 |
| `A3058` | `ALGEBRAIZE` | function '{function_name_lower}' does not support OVER clause | `src/algebraizer/expression/function.rs:134` | 1 |
| `A3059` | `ALGEBRAIZE` | window function '{function_name_lower}' requires OVER clause | `src/algebraizer/expression/function.rs:81` | 2 |
| `A3060` | `ALGEBRAIZE` | CEIL/FLOOR modifiers are not supported in this iteration | `src/algebraizer/expression/function.rs:336` | 1 |
| `A3061` | `ALGEBRAIZE` | TRIM modifiers are not supported in this iteration | `src/algebraizer/expression/mod.rs:114` | 1 |
| `A3062` | `ALGEBRAIZE` | LATERAL derived tables are not supported in this iteration | `src/algebraizer/relation/from_table_factor.rs:77` | 1 |
| `A3063` | `ALGEBRAIZE` | derived table in FROM requires an alias in this iteration | `src/algebraizer/relation/from_table_factor.rs:87` | 1 |
| `A3064` | `ALGEBRAIZE` | unsupported table factor in this iteration: {relation} | `src/algebraizer/relation/from_table_factor.rs:131` | 1 |
| `A3065` | `ALGEBRAIZE` | unsupported set expression in this iteration: {sql_set_expr} | `src/algebraizer/relation/set_ops.rs:90` | 1 |
| `A3066` | `ALGEBRAIZE` | CTE SEARCH/CYCLE clauses are not supported in this iteration | `src/algebraizer/relation/cte.rs:45` | 2 |
| `A3067` | `ALGEBRAIZE` | recursive CTE seed term must be SELECT-compatible in this iteration | `src/algebraizer/relation/cte.rs:180` | 1 |
| `A3068` | `ALGEBRAIZE` | unsupported unary operator in this iteration: {op} | `src/algebraizer/expression/mod.rs:61` | 1 |
| `A3069` | `ALGEBRAIZE` | unsupported binary operator in this iteration: {operator} | `src/algebraizer/expression/mod.rs:273` | 1 |
| `A3070` | `ALGEBRAIZE` | unsupported scalar expression in this iteration: {expr} | `src/algebraizer/expression/mod.rs:249` | 1 |
| `A3071` | `ALGEBRAIZE` | multiple FROM items are not supported in this iteration | `src/algebraizer/relation/from_join.rs:33` | 1 |
| `A3073` | `ALGEBRAIZE` | unsupported literal in this iteration: {value} | `src/algebraizer/expression/literal.rs:57` | 1 |

## 6.4 INFER Codes

| Code | Phase | Reason | Primary Source | Occurrences |
| --- | --- | --- | --- | --- |
| `I4101` | `INFER` | unknown slot reference: {slot_id} | `src/infer/expression/slot.rs:14` | 1 |
| `I4102` | `INFER` | unsupported aggregate function: {name} | `src/infer/expression/mod.rs:234` | 1 |
| `I4103` | `INFER` | unsupported window function: {name} | `src/infer/expression/mod.rs:250` | 1 |
| `I4104` | `INFER` | operator '{}' is not defined for {:?} and {:?} | `src/infer/expression/type_rules.rs:35` | 1 |
| `I4105` | `INFER` | subquery expression expects exactly one column, got {} | `src/infer/expression/subquery.rs:22` | 1 |
| `I4106` | `INFER` | invalid correlated reference depth {depth} for slot {slot_id} | `src/infer/expression/slot.rs:33` | 1 |
| `I4107` | `INFER` | unknown correlated slot reference: slot {slot_id}, depth {depth} | `src/infer/expression/slot.rs:45` | 1 |
| `I4108` | `INFER` | function expects text argument at a specific position | `src/infer/expression/function.rs:43` | 1 |
| `I4109` | `INFER` | function expects numeric argument at a specific position | `src/infer/expression/function.rs:43` | 1 |
| `I4201` | `INFER` | projection column alias was not assigned during planning | `src/infer/relation/projection.rs:27` | 1 |
| `I4202` | `INFER` | set operation column count mismatch: left {}, right {} | `src/infer/relation/set_ops.rs:23` | 1 |
| `I4203` | `INFER` | invalid cardinality interval [{min:?}, {max:?}] at {location} | `src/infer/model/cardinality.rs:31` | 1 |
