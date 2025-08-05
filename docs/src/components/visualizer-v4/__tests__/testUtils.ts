/**
 * @fileoverview Shared Test Utilities
 * 
 * Common utilities for testing the visualization components
 */

import fs from 'fs';
import path from 'path';
import { parseGraphJSON } from '../core/JSONParser';
import { createVisualizationState } from '../core/VisState';
import type { VisualizationState } from '../core/VisState';

export interface TestDataResult {
  rawData: any;
  state: VisualizationState;
  metadata: any;
}

/**
 * Load and parse the chat.json test data
 * @param grouping - Optional grouping to apply (e.g., 'location', 'filename')
 * @returns Parsed test data or null if file not found
 */
export function loadChatJsonTestData(grouping: string | null = null): TestDataResult | null {
  try {
    const chatJsonPath = path.join(__dirname, '../test-data/chat.json');
    const chatJsonContent = fs.readFileSync(chatJsonPath, 'utf8');
    const rawData = JSON.parse(chatJsonContent);
    
    const result = parseGraphJSON(rawData, grouping);
    
    return {
      rawData,
      state: result.state,
      metadata: result.metadata
    };
  } catch (error) {
    console.warn('chat.json not found, skipping test that requires test data');
    return null;
  }
}

/**
 * Skip test with message if chat.json is not available
 */
export function skipIfNoTestData(testData: TestDataResult | null, testName: string = 'test') {
  if (!testData) {
    console.log(`⚠️  Skipping ${testName}: chat.json not available`);
    return true;
  }
  return false;
}

/**
 * Create a mock VisualizationState with container hierarchy for testing
 * Useful for unit tests that don't need the full chat.json data
 */
export function createMockVisStateWithContainers() {
  const state = createVisualizationState()
    // Create nodes
    .setGraphNode('node_0', { label: 'Node 0' })
    .setGraphNode('node_1', { label: 'Node 1' })
    .setGraphNode('node_2', { label: 'Node 2' })
    .setGraphNode('node_3', { label: 'Node 3' })
    .setGraphNode('node_4', { label: 'Node 4' })
    // Create edges
    .setGraphEdge('edge_0_1', { source: 'node_0', target: 'node_1' })
    .setGraphEdge('edge_1_2', { source: 'node_1', target: 'node_2' })
    .setGraphEdge('edge_2_3', { source: 'node_2', target: 'node_3' })
    .setGraphEdge('edge_3_4', { source: 'node_3', target: 'node_4' })
    // Create containers with minimal setup - let ELK calculate dimensions
    .setContainer('container_a', { 
      children: ['node_0', 'node_1'], 
      collapsed: false 
    })
    .setContainer('container_b', { 
      children: ['node_2', 'node_3', 'node_4'], 
      collapsed: false 
    });
    
  return state;
}
