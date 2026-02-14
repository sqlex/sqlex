/// Generates TypeScript type definitions for the script generator API
pub fn generate_type_definitions() -> String {
    r#"// TypeScript type definitions for sqlex script generator

/**
 * Column information in a table
 */
interface Column {
    name: string;
    data_type: DataType;
    nullability: boolean;
}

/**
 * Database table structure
 */
interface Table {
    name: string;
    columns: Column[];
}

/**
 * SQL query descriptor
 */
interface Query {
    name: string;
    package: string[];
    module: string;
    sql: string;
    params: Column[];
    cardinality: Cardinality;
    result_columns: Column[];
}

/**
 * Query result cardinality
 */
type Cardinality =
    | "ExactlyZero"
    | "ExactlyOne"
    | "AtMostOne"
    | "OneOrMore"
    | "ZeroOrMore";

/**
 * Data type union
 */
type DataType =
    | "Bool"
    | "TinyInt"
    | "UnsignedTinyInt"
    | "SmallInt"
    | "UnsignedSmallInt"
    | "Int"
    | "UnsignedInt"
    | "BigInt"
    | "UnsignedBigInt"
    | "Float"
    | "Double"
    | "Decimal"
    | "Char"
    | "Varchar"
    | "Text"
    | "Date"
    | "Time"
    | "DateTime"
    | "Timestamp"
    | "Uuid"
    | "Json"
    | "Binary";

/**
 * Project structure containing tables and queries
 */
interface Project {
    tables: Table[];
    queries: Query[];
}

/**
 * File writer for generating output files
 */
interface Writer {
    /**
     * Write content to a file
     * @param path - Relative path to the output file
     * @param content - Content to write
     */
    write(path: string, content: string): void;
}

// Global variables available in scripts
declare const project: Project;
declare const writer: Writer;

// String utility functions
declare function toSnakeCase(str: string): string;
declare function toCamelCase(str: string): string;
declare function toPascalCase(str: string): string;
declare function toKebabCase(str: string): string;
declare function toScreamingSnakeCase(str: string): string;
"#
    .to_string()
}
