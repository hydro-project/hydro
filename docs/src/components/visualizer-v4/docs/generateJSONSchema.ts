/**
 * JSON Schema Documentation Generator
 * 
 * AUTO-GENERATED - DO NOT EDIT MANUALLY
 * Last updated: 2025-08-22T21:40:38.538Z
 * Source: JSONParser.ts interfaces
 */

// Generated from JSONParser.ts interfaces
export interface GroupingOptionSchema {
  id: string;
  name: string;
}

export interface ParseResultSchema {
  state: any;
  metadata: {
  selectedGrouping: string | null;
  nodeCount: number;
  edgeCount: number;
  containerCount: number;
  availableGroupings: any[];
  styleConfig?: Record<string, any>;
  edgeStyleConfig?: {
  propertyMappings: Record<string, any>;
  defaultStyle?: any;
  combinationRules?: any;
  };
  nodeTypeConfig?: {
  defaultType?: string;
  types?: Array<{
  id: string;
  label: string;
  colorIndex: number;
  }>;
  };
  };
}

export interface ValidationResultSchema {
  isValid: boolean;
  errors: string[];
  warnings: string[];
  nodeCount: number;
  edgeCount: number;
  hierarchyCount: number;
}

export interface ParserOptionsSchema {
  validateData?: boolean;
  strictMode?: boolean;
  defaultNodeStyle?: any;
  defaultEdgeStyle?: any;
}

export interface RawNodeSchema {
  id: string;
  semanticTags?: string[];
  [key: string]: any;
}

export interface RawEdgeSchema {
  id: string;
  source: string;
  target: string;
  semanticTags?: string[];
  edgeProperties?: string[];
  [key: string]: any;
}

export interface RawHierarchySchema {
  id: string;
  name: string;
  groups: Record<string, string[]>;
}

export interface RawHierarchyChoiceSchema {
  id: string;
  name: string;
  children?: RawHierarchyItemSchema[];  // Direct children, no wrapper
}

export interface RawHierarchyItemSchema {
  id: string;
  name: string;
  children?: RawHierarchyItemSchema[];
}

export interface RawGraphDataSchema {
  nodes: RawNodeSchema[];
  edges: RawEdgeSchema[];
  hierarchies?: RawHierarchy[];
  hierarchyChoices?: RawHierarchyChoiceSchema[];
  nodeAssignments?: Record<string, Record<string, string>>;
  styleConfig?: {
  edgeStyles?: Record<string, any>;
  nodeStyles?: Record<string, any>;
  };
  edgeStyleConfig?: {
  propertyMappings: Record<string, any>;
  defaultStyle?: any;
  combinationRules?: any;
  };
  nodeTypeConfig?: {
  defaultType?: string;
  types?: Array<{
  id: string;
  label: string;
  colorIndex: number;
  }>;
  };
  metadata?: Record<string, any>;
}

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
  const requiredExample = `{
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
}`;

  const optionalExample = `{
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
}`;

  const description = `
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

Generated: 2025-08-22T21:40:38.538Z
`;

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
export const LAST_UPDATED = "2025-08-22T21:40:38.538Z";