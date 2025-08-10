/**
 * @fileoverview Initial Layout Regression Tests
 * 
 * These tests ensure that the initial ELK layout call receives all containers
 * as expanded, allowing ELK to compute proper container sizes before any
 * smart collapse optimizations are applied.
 */

import { describe, test, expect, vi, beforeEach } from 'vitest';
import { parseGraphJSON } from '../JSONParser';
import { VisualizationEngine } from '../VisualizationEngine';
import { ELKBridge } from '../../bridges/ELKBridge';

// Mock ELKBridge to capture what gets passed to ELK
vi.mock('../../bridges/ELKBridge');

describe('Initial Layout Regression Tests', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  test('should pass all containers as expanded to initial ELK layout call', async () => {
    // Create mock data similar to paxos-flipped.json structure
    const mockData = {
      nodes: [
        { id: 'node1', label: 'Node 1' },
        { id: 'node2', label: 'Node 2' },
        { id: 'node3', label: 'Node 3' },
        { id: 'node4', label: 'Node 4' },
        { id: 'node5', label: 'Node 5' }
      ],
      edges: [
        { id: 'e1', source: 'node1', target: 'node2' },
        { id: 'e2', source: 'node2', target: 'node3' }
      ],
      hierarchyChoices: [{
        id: 'testGrouping',
        name: 'Test Grouping',
        hierarchy: [
          {
            id: 'container1',
            name: 'Container 1',
            children: [
              { id: 'container2', name: 'Container 2', children: [] }
            ]
          },
          {
            id: 'container3', 
            name: 'Container 3',
            children: []
          }
        ]
      }],
      nodeAssignments: {
        testGrouping: {
          'node1': 'container1',
          'node2': 'container2', 
          'node3': 'container2',
          'node4': 'container3',
          'node5': 'container3'
        }
      }
    };

    // Parse the data
    const parseResult = parseGraphJSON(mockData, 'testGrouping');
    const visState = parseResult.state;

    // Verify that containers are created as expanded
    const expandedContainers = visState.expandedContainers;
    expect(expandedContainers.length).toBeGreaterThan(0);
    
    // All containers should be expanded initially
    expandedContainers.forEach(container => {
      expect(container.collapsed).toBe(false);
    });

    // Mock the ELK Bridge layoutVisState method to capture calls
    const mockLayoutVisState = vi.fn().mockResolvedValue(undefined);
    const MockedELKBridge = ELKBridge as any;
    MockedELKBridge.mockImplementation(() => ({
      layoutVisState: mockLayoutVisState
    }));

    // Create VisualizationEngine (this will create the mocked ELKBridge)
    const engine = new VisualizationEngine(visState);
    
    // Trigger layout
    await engine.runLayout();

    // Verify ELK was called
    expect(mockLayoutVisState).toHaveBeenCalled();
    
    // Get the VisState that was passed to ELK
    const elkCall = mockLayoutVisState.mock.calls[0];
    const visStatePassedToElk = elkCall[0];
    
    // Verify that ELK received expanded containers
    const expandedContainersForElk = visStatePassedToElk.expandedContainers;
    expect(expandedContainersForElk.length).toBeGreaterThan(0);
    
    // REGRESSION TEST: Ensure no containers are collapsed in initial ELK call
    expandedContainersForElk.forEach(container => {
      expect(container.collapsed).toBe(false);
    });

    // Verify that visible nodes are available for ELK to layout
    const visibleNodes = visStatePassedToElk.visibleNodes;
    expect(visibleNodes.length).toBeGreaterThan(0);

    console.log(`✅ Initial ELK layout received ${expandedContainersForElk.length} expanded containers and ${visibleNodes.length} visible nodes`);
  });

  test('should not have empty ELK input on initial layout', async () => {
    // Create test data with containers that have many children (potential dimension explosion)
    const mockData = {
      nodes: Array.from({ length: 25 }, (_, i) => ({ 
        id: `node${i}`, 
        label: `Node ${i}` 
      })),
      edges: [
        { id: 'e1', source: 'node0', target: 'node1' }
      ],
      hierarchyChoices: [{
        id: 'testGrouping',
        name: 'Test Grouping', 
        hierarchy: [
          {
            id: 'largeContainer',
            name: 'Large Container',
            children: []
          }
        ]
      }],
      nodeAssignments: {
        testGrouping: Object.fromEntries(
          Array.from({ length: 25 }, (_, i) => [`node${i}`, 'largeContainer'])
        )
      }
    };

    const parseResult = parseGraphJSON(mockData, 'testGrouping');
    const visState = parseResult.state;

    // Mock ELK Bridge
    const mockLayoutVisState = vi.fn().mockResolvedValue(undefined);
    const MockedELKBridge = ELKBridge as any;
    MockedELKBridge.mockImplementation(() => ({
      layoutVisState: mockLayoutVisState
    }));

    // Create VisualizationEngine  
    const engine = new VisualizationEngine(visState);
    
    // Trigger layout
    await engine.runLayout();

    // Verify ELK was called
    expect(mockLayoutVisState).toHaveBeenCalled();

    // Get the VisState passed to ELK
    const visStatePassedToElk = mockLayoutVisState.mock.calls[0][0];
    
    // REGRESSION TEST: ELK should never receive empty input
    const expandedContainers = visStatePassedToElk.expandedContainers;
    const visibleNodes = visStatePassedToElk.visibleNodes;
    
    // Either containers or nodes should be visible to ELK
    const totalElementsForElk = expandedContainers.length + visibleNodes.length;
    expect(totalElementsForElk).toBeGreaterThan(0);

    console.log(`✅ ELK received ${totalElementsForElk} total elements (${expandedContainers.length} containers + ${visibleNodes.length} nodes)`);
  });

  test('should create containers as expanded in JSONParser regardless of child count', () => {
    // Test data with varying container sizes
    const mockData = {
      nodes: Array.from({ length: 30 }, (_, i) => ({ 
        id: `node${i}`, 
        label: `Node ${i}` 
      })),
      edges: [],
      hierarchyChoices: [{
        id: 'testGrouping',
        name: 'Test Grouping',
        hierarchy: [
          { id: 'smallContainer', name: 'Small Container', children: [] },
          { id: 'mediumContainer', name: 'Medium Container', children: [] },
          { id: 'largeContainer', name: 'Large Container', children: [] }
        ]
      }],
      nodeAssignments: {
        testGrouping: {
          // Small container: 2 children
          'node0': 'smallContainer',
          'node1': 'smallContainer',
          // Medium container: 10 children  
          ...Object.fromEntries(Array.from({ length: 10 }, (_, i) => [`node${i + 2}`, 'mediumContainer'])),
          // Large container: 18 children (would trigger dimension explosion prevention)
          ...Object.fromEntries(Array.from({ length: 18 }, (_, i) => [`node${i + 12}`, 'largeContainer']))
        }
      }
    };

    const parseResult = parseGraphJSON(mockData, 'testGrouping');
    const visState = parseResult.state;

    // REGRESSION TEST: All containers should be created as expanded by JSONParser
    const allContainers = ['smallContainer', 'mediumContainer', 'largeContainer'];
    
    allContainers.forEach(containerId => {
      const container = visState.getContainer(containerId);
      expect(container).toBeTruthy();
      expect(container.collapsed).toBe(false);
    });

    // The expandedContainers getter should initially return all containers  
    const expandedContainers = visState.expandedContainers;
    expect(expandedContainers.length).toBe(3);

    console.log(`✅ JSONParser created ${expandedContainers.length}/3 containers as expanded`);
  });
});
