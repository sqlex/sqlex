/// Generates an example TypeScript script demonstrating the script generator API
pub fn generate_example_script() -> String {
    r#"// Example TypeScript script for sqlex code generation
// This script demonstrates how to use the script generator API

// Access project tables
console.log(`Found ${project.tables.length} tables`);

for (const table of project.tables) {
    console.log(`Processing table: ${table.name}`);

    // Generate a simple model file for each table
    const className = toPascalCase(table.name);
    const fileName = toSnakeCase(table.name);

    let content = `// Generated model for ${table.name}\n\n`;
    content += `export interface ${className} {\n`;

    for (const column of table.columns) {
        const fieldName = toCamelCase(column.name);
        const fieldType = getTypeScriptType(column.data_type);
        const optional = column.nullability ? '?' : '';
        content += `    ${fieldName}${optional}: ${fieldType};\n`;
    }

    content += `}\n`;

    // Write the generated file
    writer.write(`models/${fileName}.ts`, content);
}

// Access project queries
console.log(`Found ${project.queries.length} queries`);

for (const query of project.queries) {
    console.log(`Processing query: ${query.name}`);
    // You can generate query-related code here
}

// Helper function to map database types to TypeScript types
function getTypeScriptType(dataType: any): string {
    if (typeof dataType === 'string') {
        switch (dataType) {
            case 'Text': return 'string';
            case 'Boolean': return 'boolean';
            case 'Float':
            case 'Double':
            case 'Decimal': return 'number';
            case 'Date':
            case 'Time':
            case 'Timestamp': return 'Date';
            case 'Uuid': return 'string';
            case 'Json': return 'any';
            case 'Blob': return 'Buffer';
            default: return 'unknown';
        }
    }

    // Handle object types like { Int: boolean }
    if (typeof dataType === 'object') {
        if ('Int' in dataType || 'BigInt' in dataType || 'SmallInt' in dataType) {
            return 'number';
        }
    }

    return 'unknown';
}

console.log('Code generation completed!');
"#
    .to_string()
}
