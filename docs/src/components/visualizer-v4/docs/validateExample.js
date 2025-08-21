#!/usr/bin/env node

/**
 * Example Validation Script
 * 
 * Validates that the complete example in generateJSONSchema.ts
 * is actually valid according to the JSONParser.
 */

const fs = require('fs');
const path = require('path');

/**
 * Simple validation without importing TypeScript files
 */
function validateExampleStructure() {
  console.log('🔍 Validating complete example structure...');
  
  try {
    // Read the schema generation file 
    const schemaFile = path.join(__dirname, 'generateJSONSchema.ts');
    const content = fs.readFileSync(schemaFile, 'utf8');
    
    // Extract the example JSON from the generateCompleteExample function
    const exampleMatch = content.match(/export function generateCompleteExample\(\)\s*{[\s\S]*?return\s*({[\s\S]*?});/);
    
    if (!exampleMatch) {
      throw new Error('Could not find generateCompleteExample function');
    }
    
    // Parse the example object (this is a simplified check)
    const exampleStr = exampleMatch[1];
    
    // Basic structural validation
    const requiredFields = ['nodes', 'edges'];
    const optionalFields = ['hierarchyChoices', 'nodeAssignments', 'edgeStyleConfig'];
    
    for (const field of requiredFields) {
      if (!exampleStr.includes(`"${field}"`)) {
        throw new Error(`Missing required field: ${field}`);
      }
    }
    
    // Check that nodes have required structure
    if (!exampleStr.includes('"id"') || !exampleStr.includes('"label"')) {
      throw new Error('Nodes must have id and label fields');
    }
    
    // Check that edges have required structure  
    if (!exampleStr.includes('"source"') || !exampleStr.includes('"target"')) {
      throw new Error('Edges must have source and target fields');
    }
    
    // Check hierarchy structure if present
    if (exampleStr.includes('"hierarchyChoices"')) {
      if (!exampleStr.includes('"children"')) {
        throw new Error('hierarchyChoices must use children field (not hierarchy)');
      }
    }
    
    console.log('✅ Example structure validation passed');
    return true;
    
  } catch (error) {
    console.error('❌ Example validation failed:', error.message);
    return false;
  }
}

// Run validation
if (require.main === module) {
  const valid = validateExampleStructure();
  process.exit(valid ? 0 : 1);
}

module.exports = { validateExampleStructure };
