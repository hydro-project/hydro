#!/usr/bin/env node

/**
 * Schema Documentation Sync Script
 * 
 * Automatically extracts TypeScript interfaces from JSONParser.ts
 * and updates the schema documentation to keep it in sync.
 * 
 * Usage: npm run sync-schema-docs
 */

const fs = require('fs');
const path = require('path');

const PARSER_FILE = path.join(__dirname, '../core/JSONParser.ts');
const SCHEMA_FILE = path.join(__dirname, 'generateJSONSchema.ts');

/**
 * Extract interface definitions from TypeScript file
 */
function extractInterfaces(content) {
  const interfaces = {};
  
  // Find all interface declarations
  const interfaceStartRegex = /interface\s+(\w+)\s*{/g;
  let match;
  
  while ((match = interfaceStartRegex.exec(content)) !== null) {
    const interfaceName = match[1];
    const startPos = match.index + match[0].length;
    
    // Find the matching closing brace
    let braceCount = 1;
    let pos = startPos;
    
    while (pos < content.length && braceCount > 0) {
      const char = content[pos];
      if (char === '{') {
        braceCount++;
      } else if (char === '}') {
        braceCount--;
      }
      pos++;
    }
    
    if (braceCount === 0) {
      const body = content.substring(startPos, pos - 1);
      
      // Clean up and properly format the interface body
      const cleanedBody = body
        .trim()
        .split('\n')
        .map(line => line.trim())
        .filter(line => line.length > 0)
        .map(line => `  ${line}`)
        .join('\n');
      
      // Replace type references with Schema versions or 'any'
      const schemaBody = cleanedBody
        .replace(/\b(VisualizationState|GroupingOption|NodeStyle|EdgeStyle)\b/g, 'any')
        .replace(/\b(RawNode|RawEdge|RawHierarchyChoice|RawHierarchyItem)\[\]/g, (match, typeName) => `${typeName}Schema[]`)
        .replace(/\b(RawNode|RawEdge|RawHierarchyChoice|RawHierarchyItem)\b/g, (match, typeName) => `${typeName}Schema`);
      
      interfaces[interfaceName] = schemaBody;
    }
  }
  
  return interfaces;
}

/**
 * Generate updated schema documentation
 */
function generateUpdatedSchema(interfaces) {
  const timestamp = new Date().toISOString();
  
  // Extract key interfaces we care about
  const nodeInterface = interfaces.RawNode || '';
  const edgeInterface = interfaces.RawEdge || '';
  const hierarchyInterface = interfaces.RawHierarchy || '';
  
  return `/**
 * JSON Schema Documentation Generator
 * 
 * AUTO-GENERATED - DO NOT EDIT MANUALLY
 * Last updated: ${timestamp}
 * Source: JSONParser.ts interfaces
 */

// Generated from JSONParser.ts interfaces
${Object.entries(interfaces).map(([name, body]) => 
  `export interface ${name}Schema {\n${body}\n}`
).join('\n\n')}

/**
 * Generate JSON schema documentation from TypeScript interfaces
 * AUTO-GENERATED from actual parser interfaces
 */
export function generateSchemaDocumentation(): {
  requiredExample: string;
  optionalExample: string;
  completeExample: string;
  description: string;
} {
  // These examples are derived from the actual TypeScript interfaces
  const requiredExample = \`{
  "nodes": [
    {
      "id": "string",           // Required: unique identifier
      "nodeType": "string",     // Optional: node type for styling
      "fullLabel": "string",    // Optional: detailed label
      "shortLabel": "string",   // Optional: abbreviated label
      "semanticTags": ["..."],  // Optional: to be mapped to styling below
      "data": {                 // Optional: application-specific metadata
                                // For example, Hydro stores code locations and function backtrace info for rendering
      }
      ...                       // Additional properties preserved
    }
  ],
  "edges": [
    {
      "id": "string",           // Required: unique identifier  
      "source": "string",       // Required: source node id
      "target": "string",       // Required: target node id
      "label": "string",        // Optional: edge label
      "semanticTags": ["..."],  // Optional: to be mapped to styling below
      "edgeProperties": ["..."], // Optional: edge properties (alternative to semanticTags)
      ...                       // Additional properties preserved
    }
  ]
}\`;

  const optionalExample = \`{
  "hierarchyChoices": [         // Supports multiple hierarchies, to be chosen from a menu
    {
      "id": "string",
      "name": "string",
      "children": [              // nest your hierarchy here
        { "id": "string", "name": "string", "children": [...] }
      ]
    }
  ],
  "nodeAssignments": {          // Node-to-hierarchy id mappings; place the node in each hierarchy separately
    "hierarchyId": { "nodeId": "groupId" }
  },
  "edgeStyleConfig": {          // Edge styling configuration
    "propertyMappings": { "property": "styleTag" },
    "defaultStyle": { "stroke": "#666", "strokeWidth": 2 },
    "combinationRules": { "priority": ["tag1", "tag2"] }
  },
  "nodeTypeConfig": {           // Node type configuration
    "defaultType": "default",
    "types": [
      { "id": "string", "label": "string", "colorIndex": 0 }
    ]
  },
  "legend": {                   // Legend configuration (display only)
    "title": "string",
    "items": [
      { "label": "string", "type": "string" }
    ]
  }
}\`;

  const description = \`
This JSON format is used by the Hydro visualization system to represent graph data.
The schema is automatically maintained to stay in sync with the JSONParser implementation.

Key features:
- Nodes and edges are required (minimal: id, source, target)
- Your own rich node metadata via 'data' field (nodeType, labels, backtrace, location info)
- Edge properties and styling via semanticTags or edgeProperties
- Support for multiple nesting hierarchies
- Extensive styling and configuration options
- Node type configuration for visual categorization
- Legend support for documentation (display only)
- Semantic tags for flexible categorization
- All additional properties are preserved by the parser

Generated: ${timestamp}
\`;

  const completeExample = JSON.stringify(generateCompleteExample(), null, 2);

  return {
    requiredExample,
    optionalExample,
    completeExample,
    description: description.trim()
  };
}

/**
 * Generate a complete, valid example JSON that can be used as a working sample
 */
export function generateCompleteExample() {
  return {
    "nodes": [
      {
        "id": "web_server",
        "nodeType": "Transform",
        "fullLabel": "Web Server (HTTP Handler)",
        "shortLabel": "WebSrv",
        "semanticTags": ["critical", "backend"],
        "data": {
          "locationId": 1,
          "locationType": "Process",
          "backtrace": [
            {
              "file": "src/server.rs",
              "fn": "handle_request",
              "line": 42
            },
            {
              "file": "main.rs",
              "fn": "main",
              "line": 15
            }
          ]
        }
      },
      {
        "id": "database",
        "nodeType": "Sink",
        "fullLabel": "PostgreSQL Database",
        "shortLabel": "DB",
        "label": "Database",
        "semanticTags": ["critical", "storage"],
        "data": {
          "locationId": 2,
          "locationType": "Process"
        }
      },
      {
        "id": "cache",
        "nodeType": "Transform",
        "fullLabel": "Redis Cache (In-Memory)",
        "shortLabel": "Cache",
        "semanticTags": ["performance", "storage"],
      },
      {
        "id": "load_balancer",
        "nodeType": "Source",
        "fullLabel": "HAProxy Load Balancer",
        "shortLabel": "LB",
        "semanticTags": ["network", "frontend"],
      }
    ],
    "edges": [
      {
        "id": "e1",
        "source": "load_balancer",
        "target": "web_server",
        "label": "HTTP",
        "semanticTags": ["Network", "TotalOrder", "Unbounded"]
      },
      {
        "id": "e2", 
        "source": "web_server",
        "target": "database",
        "label": "SQL Query",
        "semanticTags": ["Bounded", "TotalOrder"]
      },
      {
        "id": "e3",
        "source": "web_server", 
        "target": "cache",
        "label": "Cache Check",
        "semanticTags": ["Keyed", "NoOrder", "Unbounded"]
      }
    ],
    "hierarchyChoices": [
      {
        "id": "by_layer",
        "name": "Architecture Layer",
        "children": [
          {
            "id": "frontend",
            "name": "Frontend Layer"
          },
          {
            "id": "backend", 
            "name": "Backend Layer"
          },
          {
            "id": "storage",
            "name": "Storage Layer"
          }
        ]
      },
      {
        "id": "by_criticality",
        "name": "Criticality",
        "children": [
          {
            "id": "critical_services",
            "name": "Critical Services"
          },
          {
            "id": "support_services",
            "name": "Support Services"
          }
        ]
      }
    ],
    "nodeAssignments": {
      "by_layer": {
        "load_balancer": "frontend",
        "web_server": "backend",
        "database": "storage",
        "cache": "storage"
      },
      "by_criticality": {
        "web_server": "critical_services",
        "database": "critical_services", 
        "cache": "support_services",
        "load_balancer": "support_services"
      }
    },
    "edgeStyleConfig": {
      "propertyMappings": {
        "Bounded": "thick-stroke",
        "Keyed": "double-line",
        "Network": "dashed-animated",
        "NoOrder": "wavy-line",
        "TotalOrder": "smooth-line",
        "Unbounded": "thin-stroke"
      },
      "combinationRules": {
        "mutualExclusions": [],
        "visualGroups": {}
      }
    },
    "nodeTypeConfig": {
      "defaultType": "Transform",
      "types": [
        {
          "id": "Source",
          "label": "Source",
          "colorIndex": 0
        },
        {
          "id": "Transform",
          "label": "Transform",
          "colorIndex": 1
        },
        {
          "id": "Sink",
          "label": "Sink", 
          "colorIndex": 2
        },
        {
          "id": "Network",
          "label": "Network",
          "colorIndex": 3
        }
      ]
    },
    "legend": {
      "title": "Node Types",
      "items": [
        {
          "label": "Source",
          "type": "Source"
        },
        {
          "label": "Transform",
          "type": "Transform"
        },
        {
          "label": "Sink",
          "type": "Sink"
        },
        {
          "label": "Network",
          "type": "Network"
        }
      ]
    }
  };
}

/**
 * Version information for schema tracking
 */
export const SCHEMA_VERSION = "v4.0.0";
export const LAST_UPDATED = "${timestamp}";`;
}

/**
 * Validate the complete example structure
 */
function validateCompleteExample() {
  const { validateExampleStructure } = require('./validateExample.js');
  
  console.log('🔍 Validating complete example...');
  const isValid = validateExampleStructure();
  
  if (!isValid) {
    console.error('❌ Complete example validation failed');
    return false;
  }
  
  console.log('✅ Complete example validation passed');
  return true;
}

/**
 * Main sync function
 */
function syncSchemaDocumentation() {
  try {
    console.log('🔄 Syncing schema documentation...');
    
    // Read the parser file
    const parserContent = fs.readFileSync(PARSER_FILE, 'utf8');
    
    // Extract interfaces
    const interfaces = extractInterfaces(parserContent);
    
    console.log(`📋 Found ${Object.keys(interfaces).length} interfaces:`, Object.keys(interfaces));
    
    // Generate updated schema
    const updatedSchema = generateUpdatedSchema(interfaces);
    
    // Write back to schema file
    fs.writeFileSync(SCHEMA_FILE, updatedSchema);
    
    console.log('✅ Schema documentation updated successfully');
    console.log(`📄 Updated: ${SCHEMA_FILE}`);
    
    // Validate the complete example
    const exampleValid = validateCompleteExample();
    if (!exampleValid) {
      console.error('❌ Schema sync failed: Complete example is invalid');
      process.exit(1);
    }
    
  } catch (error) {
    console.error('❌ Error syncing schema documentation:', error);
    process.exit(1);
  }
}

// Run if called directly
if (require.main === module) {
  syncSchemaDocumentation();
}

module.exports = { syncSchemaDocumentation };
