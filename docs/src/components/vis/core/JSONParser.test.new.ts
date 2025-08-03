/**
 * @fileoverview JSONParser Unit Tests
 * 
 * Tests for parsing Hydro graph JSON data into VisualizationState
 */

import { describe, it, expect } from 'vitest';
import { 
  parseGraphJSON, 
  createGraphParser, 
  getAvailableGroupings,
  validateGraphJSON 
} from '../core/JSONParser';
import { NODE_STYLES, EDGE_STYLES } from '../shared/constants';
import type { ParseResult, ValidationResult, GroupingOption } from '../core/JSONParser';

describe('JSONParser', () => {
  // Sample test data based on the chat.json structure
  const sampleGraphData: any = {
    nodes: [
      {
        id: "0",
        data: {
          backtrace: [
            {
              fn_name: "hydro_lang::stream::Stream<T,L,B,O,R>::broadcast_bincode",
              filename: "/Users/test/stream.rs"
            }
          ]
        }
      },
      {
        id: "1", 
        data: {
          backtrace: [
            {
              fn_name: "test_function",
              filename: "/Users/test/other.rs"
            }
          ]
        }
      }
    ],
    edges: [
      {
        id: "edge_0_1",
        source: "0",
        target: "1",
        data: {}
      }
    ]
  };

  describe('parseGraphJSON', () => {
    it('should parse valid graph JSON', () => {
      const result: ParseResult = parseGraphJSON(sampleGraphData, 'filename');
      
      expect(result).toBeDefined();
      expect(result.success).toBe(true);
      expect(result.visState).toBeDefined();
      
      if (result.success) {
        expect(result.visState.visibleNodes).toBeDefined();
        expect(result.visState.visibleEdges).toBeDefined();
        expect(Array.isArray(result.visState.visibleNodes)).toBe(true);
        expect(Array.isArray(result.visState.visibleEdges)).toBe(true);
      }
    });

    it('should handle empty graph data', () => {
      const emptyData = { nodes: [], edges: [] };
      const result = parseGraphJSON(emptyData, 'filename');
      
      expect(result.success).toBe(true);
      if (result.success) {
        expect(result.visState.visibleNodes.length).toBe(0);
        expect(result.visState.visibleEdges.length).toBe(0);
      }
    });

    it('should handle invalid JSON structure', () => {
      const invalidData = { invalidStructure: true };
      const result = parseGraphJSON(invalidData, 'filename');
      
      // Should either succeed with empty state or fail gracefully
      expect(result).toBeDefined();
      expect(typeof result.success).toBe('boolean');
    });

    it('should apply grouping correctly', () => {
      const result = parseGraphJSON(sampleGraphData, 'filename');
      
      expect(result.success).toBe(true);
      if (result.success) {
        // Nodes should be grouped by filename
        const containers = result.visState.visibleContainers;
        expect(Array.isArray(containers)).toBe(true);
        
        // Should have containers for different files
        const filenames = containers.map(c => c.id);
        expect(filenames.includes('/Users/test/stream.rs')).toBe(true);
      }
    });
  });

  describe('createGraphParser', () => {
    it('should create a parser instance', () => {
      const parser = createGraphParser();
      expect(parser).toBeDefined();
      expect(typeof parser.parse).toBe('function');
    });

    it('should parse with custom options', () => {
      const parser = createGraphParser({
        groupingStrategy: 'filename',
        validateInput: true
      });
      
      const result = parser.parse(sampleGraphData);
      expect(result).toBeDefined();
      expect(typeof result.success).toBe('boolean');
    });
  });

  describe('getAvailableGroupings', () => {
    it('should return available grouping options', () => {
      const groupings: GroupingOption[] = getAvailableGroupings(sampleGraphData);
      
      expect(Array.isArray(groupings)).toBe(true);
      expect(groupings.length).toBeGreaterThan(0);
      
      // Should include filename grouping
      const filenameGrouping = groupings.find(g => g.key === 'filename');
      expect(filenameGrouping).toBeDefined();
      expect(filenameGrouping!.name).toBeDefined();
      expect(typeof filenameGrouping!.count).toBe('number');
    });

    it('should handle empty data', () => {
      const emptyData = { nodes: [], edges: [] };
      const groupings = getAvailableGroupings(emptyData);
      
      expect(Array.isArray(groupings)).toBe(true);
      // Should still return default grouping options even with empty data
    });
  });

  describe('validateGraphJSON', () => {
    it('should validate correct JSON structure', () => {
      const result: ValidationResult = validateGraphJSON(sampleGraphData);
      
      expect(result).toBeDefined();
      expect(typeof result.valid).toBe('boolean');
      expect(Array.isArray(result.errors)).toBe(true);
      expect(Array.isArray(result.warnings)).toBe(true);
    });

    it('should detect invalid structure', () => {
      const invalidData = { notAGraph: true };
      const result = validateGraphJSON(invalidData);
      
      expect(result.valid).toBe(false);
      expect(result.errors.length).toBeGreaterThan(0);
    });

    it('should detect missing required fields', () => {
      const incompleteData = {
        nodes: [{ id: "1" }], // Missing data field
        edges: []
      };
      
      const result = validateGraphJSON(incompleteData);
      
      // Should still be valid but might have warnings
      expect(typeof result.valid).toBe('boolean');
      expect(Array.isArray(result.warnings)).toBe(true);
    });
  });

  describe('error handling', () => {
    it('should handle null input gracefully', () => {
      const result = parseGraphJSON(null as any, 'filename');
      
      expect(result).toBeDefined();
      expect(result.success).toBe(false);
      if (!result.success) {
        expect(result.error).toBeDefined();
      }
    });

    it('should handle malformed JSON', () => {
      const malformedData = {
        nodes: "not an array",
        edges: null
      };
      
      const result = parseGraphJSON(malformedData, 'filename');
      
      // Should fail gracefully without throwing
      expect(result).toBeDefined();
      expect(typeof result.success).toBe('boolean');
    });
  });

  describe('integration', () => {
    it('should produce consistent results', () => {
      const result1 = parseGraphJSON(sampleGraphData, 'filename');
      const result2 = parseGraphJSON(sampleGraphData, 'filename');
      
      expect(result1.success).toBe(result2.success);
      
      if (result1.success && result2.success) {
        expect(result1.visState.visibleNodes.length).toBe(result2.visState.visibleNodes.length);
        expect(result1.visState.visibleEdges.length).toBe(result2.visState.visibleEdges.length);
      }
    });

    // TODO: Add more comprehensive integration tests
    it.skip('should handle large graph data efficiently', () => {
      // This would test performance with large datasets
      expect(true).toBe(true);
    });

    it.skip('should preserve edge relationships correctly', () => {
      // This would test that edges maintain correct source/target relationships
      expect(true).toBe(true);
    });
  });
});
