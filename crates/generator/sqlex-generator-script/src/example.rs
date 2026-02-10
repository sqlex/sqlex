/// Generates an example TypeScript script demonstrating the script generator API
pub fn generate_example_script() -> String {
    r#"/// <reference path="./sqlex-types.d.ts" />

// Example TypeScript script for sqlex code generation
// This script demonstrates how to use the script generator API

// Access project tables
console.log(`Found ${project.tables.length} tables`);

for (const table of project.tables) {
    console.log(`Processing table: ${table.name}`);

    // Generate a simple model file for each table
    const className = toPascalCase(table.name);
    const fileName = toSnakeCase(table.name);

    let content = `# Generated model for ${table.name}\n\n`;
    content += `class ${className}:\n`;
    content += `    def __init__(self):\n`;

    for (const column of table.columns) {
        const fieldName = toSnakeCase(column.name);
        const fieldType = getPythonType(column.data_type);
        const optional = column.nullability ? ` | None` : '';
        content += `        self.${fieldName}: ${fieldType}${optional}\n`;
    }

    // Write the generated file
    writer.write(`models/${fileName}.py`, content);
}

// Access project queries
console.log(`Found ${project.queries.length} queries`);

for (const query of project.queries) {
    console.log(`Processing query: ${query.name}`);
    console.log(`  Package: ${query.package.join('.')}`);
    console.log(`  Module: ${query.module}`);
    console.log(`  Cardinality: ${query.cardinality}`);
    // You can generate query-related code here
}

// Helper function to map database types to Python types
function getPythonType(dataType: string): string {
    switch (dataType) {
        case 'Bool': return 'bool';
        case 'TinyInt':
        case 'UnsignedTinyInt':
        case 'SmallInt':
        case 'UnsignedSmallInt':
        case 'Int':
        case 'UnsignedInt':
        case 'BigInt':
        case 'UnsignedBigInt': return 'int';
        case 'Float':
        case 'Double':
        case 'Decimal': return 'float';
        case 'Char':
        case 'Varchar':
        case 'Text': return 'str';
        case 'Date':
        case 'Time':
        case 'DateTime':
        case 'Timestamp': return 'datetime';
        case 'Uuid': return 'str';
        case 'Json': return 'dict';
        case 'Binary': return 'bytes';
        default: return 'Any';
    }
}

console.log('Code generation completed!');
"#
    .to_string()
}
