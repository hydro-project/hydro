/**
 * Layout Types Tests (TypeScript)
 * 
 * Tests for layout type interfaces, type validation,
 * and type compatibility across the layout system.
 */

import assert from 'assert';
import {
  LayoutConfig,
  LayoutPosition,
  LayoutDimensions,
  LayoutResult,
  PositionedNode,
  PositionedEdge,
  PositionedContainer,
  PositionedHyperEdge,
  LayoutEngine,
  AdvancedLayoutEngine,
  LayoutEngineOptions,
  LayoutValidationResult,
  LayoutValidationError,
  LayoutValidationWarning,
  LayoutStatistics,
  LayoutEventData,
  LayoutEventCallback
} from '../types.js';
import { GraphNode, GraphEdge, Container, HyperEdge } from '../../shared/types.js';

/**
 * Test basic layout types structure
 */
function testBasicLayoutTypes(): void {
  console.log('Testing basic layout types...');
  
  // Test LayoutPosition
  const position: LayoutPosition = { x: 100, y: 200 };
  assert.strictEqual(position.x, 100, 'Position x should be correct');
  assert.strictEqual(position.y, 200, 'Position y should be correct');
  assert(typeof position.x === 'number', 'Position x should be number');
  assert(typeof position.y === 'number', 'Position y should be number');
  
  // Test LayoutDimensions
  const dimensions: LayoutDimensions = { width: 180, height: 60 };
  assert.strictEqual(dimensions.width, 180, 'Dimensions width should be correct');
  assert.strictEqual(dimensions.height, 60, 'Dimensions height should be correct');
  assert(typeof dimensions.width === 'number', 'Dimensions width should be number');
  assert(typeof dimensions.height === 'number', 'Dimensions height should be number');
  
  // Test LayoutConfig
  const config: LayoutConfig = {
    algorithm: 'layered',
    direction: 'RIGHT',
    spacing: 50,
    nodeSize: { width: 180, height: 60 }
  };
  assert.strictEqual(config.algorithm, 'layered', 'Config algorithm should be correct');
  assert.strictEqual(config.direction, 'RIGHT', 'Config direction should be correct');
  assert.strictEqual(config.spacing, 50, 'Config spacing should be correct');
  assert.deepStrictEqual(config.nodeSize, { width: 180, height: 60 }, 'Config nodeSize should be correct');
  
  console.log('✅ Basic layout types test passed');
}

/**
 * Test positioned element types
 */
function testPositionedElementTypes(): void {
  console.log('Testing positioned element types...');
  
  // Create base elements
  const baseNode: GraphNode = {
    id: 'node1',
    label: 'Test Node',
    style: 'default',
    hidden: false
  };
  
  const baseEdge: GraphEdge = {
    id: 'edge1',
    source: 'node1',
    target: 'node2',
    style: 'default',
    hidden: false
  };
  
  const baseContainer: Container = {
    id: 'container1',
    children: new Set(['node1', 'node2']),
    expandedDimensions: { width: 400, height: 300 },
    collapsed: false,
    hidden: false
  };
  
  const baseHyperEdge: HyperEdge = {
    id: 'hyperedge1',
    source: 'container1',
    target: 'node3',
    style: 'default',
    aggregatedEdges: []
  };
  
  // Test PositionedNode
  const positionedNode: PositionedNode = {
    ...baseNode,
    x: 100,
    y: 200,
    width: 180,
    height: 60
  };
  
  assert.strictEqual(positionedNode.id, 'node1', 'PositionedNode should inherit base properties');
  assert.strictEqual(positionedNode.label, 'Test Node', 'PositionedNode should inherit base properties');
  assert.strictEqual(positionedNode.x, 100, 'PositionedNode should have position');
  assert.strictEqual(positionedNode.y, 200, 'PositionedNode should have position');
  assert.strictEqual(positionedNode.width, 180, 'PositionedNode should have dimensions');
  assert.strictEqual(positionedNode.height, 60, 'PositionedNode should have dimensions');
  
  // Test PositionedEdge
  const positionedEdge: PositionedEdge = {
    ...baseEdge,
    points: [{ x: 0, y: 0 }, { x: 100, y: 100 }]
  };
  
  assert.strictEqual(positionedEdge.id, 'edge1', 'PositionedEdge should inherit base properties');
  assert.strictEqual(positionedEdge.source, 'node1', 'PositionedEdge should inherit base properties');
  assert.strictEqual(positionedEdge.target, 'node2', 'PositionedEdge should inherit base properties');
  assert(Array.isArray(positionedEdge.points), 'PositionedEdge should have points array');
  assert.strictEqual(positionedEdge.points?.length, 2, 'PositionedEdge should have correct points');
  
  // Test PositionedContainer
  const positionedContainer: PositionedContainer = {
    ...baseContainer,
    x: 50,
    y: 100,
    width: 400,
    height: 300
  };
  
  assert.strictEqual(positionedContainer.id, 'container1', 'PositionedContainer should inherit base properties');
  assert(positionedContainer.children instanceof Set, 'PositionedContainer should inherit base properties');
  assert(positionedContainer.children.has('node1'), 'PositionedContainer should have node1');
  assert(positionedContainer.children.has('node2'), 'PositionedContainer should have node2');
  assert.strictEqual(positionedContainer.x, 50, 'PositionedContainer should have position');
  assert.strictEqual(positionedContainer.y, 100, 'PositionedContainer should have position');
  assert.strictEqual(positionedContainer.width, 400, 'PositionedContainer should have dimensions');
  assert.strictEqual(positionedContainer.height, 300, 'PositionedContainer should have dimensions');
  
  // Test PositionedHyperEdge
  const positionedHyperEdge: PositionedHyperEdge = {
    ...baseHyperEdge,
    points: [{ x: 200, y: 300 }, { x: 400, y: 500 }]
  };
  
  assert.strictEqual(positionedHyperEdge.id, 'hyperedge1', 'PositionedHyperEdge should inherit base properties');
  assert.strictEqual(positionedHyperEdge.source, 'container1', 'PositionedHyperEdge should inherit base properties');
  assert.strictEqual(positionedHyperEdge.target, 'node3', 'PositionedHyperEdge should inherit base properties');
  assert(Array.isArray(positionedHyperEdge.points), 'PositionedHyperEdge should have points array');
  assert.strictEqual(positionedHyperEdge.points?.length, 2, 'PositionedHyperEdge should have correct points');
  
  console.log('✅ Positioned element types test passed');
}

/**
 * Test LayoutResult type
 */
function testLayoutResultType(): void {
  console.log('Testing LayoutResult type...');
  
  // Create a complete LayoutResult
  const layoutResult: LayoutResult = {
    nodes: [
      {
        id: 'node1',
        label: 'Node 1',
        style: 'default',
        hidden: false,
        x: 100,
        y: 200,
        width: 180,
        height: 60
      },
      {
        id: 'node2',
        label: 'Node 2',
        style: 'default',
        hidden: false,
        x: 300,
        y: 200,
        width: 180,
        height: 60
      }
    ],
    edges: [
      {
        id: 'edge1',
        source: 'node1',
        target: 'node2',
        style: 'default',
        hidden: false,
        points: [{ x: 280, y: 230 }, { x: 300, y: 230 }]
      }
    ],
    containers: [
      {
        id: 'container1',
        children: new Set(['node1', 'node2']),
        expandedDimensions: { width: 400, height: 300 },
        collapsed: false,
        hidden: false,
        x: 50,
        y: 150,
        width: 400,
        height: 200
      }
    ],
    hyperEdges: [
      {
        id: 'hyperedge1',
        source: 'container1',
        target: 'external',
        style: 'default',
        aggregatedEdges: [],
        points: [{ x: 450, y: 250 }, { x: 500, y: 250 }]
      }
    ]
  };
  
  // Verify structure
  assert(Array.isArray(layoutResult.nodes), 'LayoutResult should have nodes array');
  assert(Array.isArray(layoutResult.edges), 'LayoutResult should have edges array');
  assert(Array.isArray(layoutResult.containers), 'LayoutResult should have containers array');
  assert(Array.isArray(layoutResult.hyperEdges), 'LayoutResult should have hyperEdges array');
  
  // Verify content
  assert.strictEqual(layoutResult.nodes.length, 2, 'Should have correct number of nodes');
  assert.strictEqual(layoutResult.edges.length, 1, 'Should have correct number of edges');
  assert.strictEqual(layoutResult.containers.length, 1, 'Should have correct number of containers');
  assert.strictEqual(layoutResult.hyperEdges.length, 1, 'Should have correct number of hyperEdges');
  
  // Verify nodes have position and dimensions
  layoutResult.nodes.forEach((node, index) => {
    assert('x' in node, `Node ${index} should have x position`);
    assert('y' in node, `Node ${index} should have y position`);
    assert('width' in node, `Node ${index} should have width`);
    assert('height' in node, `Node ${index} should have height`);
    assert(typeof node.x === 'number', `Node ${index} x should be number`);
    assert(typeof node.y === 'number', `Node ${index} y should be number`);
    assert(typeof node.width === 'number', `Node ${index} width should be number`);
    assert(typeof node.height === 'number', `Node ${index} height should be number`);
  });
  
  console.log('✅ LayoutResult type test passed');
}

/**
 * Test validation types
 */
function testValidationTypes(): void {
  console.log('Testing validation types...');
  
  // Test LayoutValidationError
  const validationError: LayoutValidationError = {
    type: 'containment',
    message: 'Node extends beyond container bounds',
    nodeId: 'node1',
    containerId: 'container1',
    details: { overhang: 50 }
  };
  
  assert.strictEqual(validationError.type, 'containment', 'Error should have correct type');
  assert.strictEqual(validationError.message, 'Node extends beyond container bounds', 'Error should have message');
  assert.strictEqual(validationError.nodeId, 'node1', 'Error should have nodeId');
  assert.strictEqual(validationError.containerId, 'container1', 'Error should have containerId');
  assert.deepStrictEqual(validationError.details, { overhang: 50 }, 'Error should have details');
  
  // Test LayoutValidationWarning
  const validationWarning: LayoutValidationWarning = {
    type: 'performance',
    message: 'Layout may be slow with many nodes',
    suggestion: 'Consider using hierarchical layout',
    details: { nodeCount: 1000 }
  };
  
  assert.strictEqual(validationWarning.type, 'performance', 'Warning should have correct type');
  assert.strictEqual(validationWarning.message, 'Layout may be slow with many nodes', 'Warning should have message');
  assert.strictEqual(validationWarning.suggestion, 'Consider using hierarchical layout', 'Warning should have suggestion');
  assert.deepStrictEqual(validationWarning.details, { nodeCount: 1000 }, 'Warning should have details');
  
  // Test LayoutValidationResult
  const validationResult: LayoutValidationResult = {
    isValid: false,
    errors: [validationError],
    warnings: [validationWarning]
  };
  
  assert.strictEqual(validationResult.isValid, false, 'ValidationResult should have isValid');
  assert(Array.isArray(validationResult.errors), 'ValidationResult should have errors array');
  assert(Array.isArray(validationResult.warnings), 'ValidationResult should have warnings array');
  assert.strictEqual(validationResult.errors.length, 1, 'Should have correct number of errors');
  assert.strictEqual(validationResult.warnings.length, 1, 'Should have correct number of warnings');
  
  console.log('✅ Validation types test passed');
}

/**
 * Test layout engine types
 */
function testLayoutEngineTypes(): void {
  console.log('Testing layout engine types...');
  
  // Test LayoutEngineOptions
  const engineOptions: LayoutEngineOptions = {
    enableCaching: true,
    enableValidation: true,
    logLevel: 'info'
  };
  
  assert.strictEqual(engineOptions.enableCaching, true, 'Options should have enableCaching');
  assert.strictEqual(engineOptions.enableValidation, true, 'Options should have enableValidation');
  assert.strictEqual(engineOptions.logLevel, 'info', 'Options should have logLevel');
  
  // Test LayoutStatistics
  const layoutStats: LayoutStatistics = {
    totalNodes: 10,
    totalEdges: 15,
    totalContainers: 2,
    layoutDuration: 150,
    validationResult: {
      isValid: true,
      errors: [],
      warnings: []
    },
    cacheStats: {
      hits: 5,
      misses: 3,
      size: 8
    }
  };
  
  assert.strictEqual(layoutStats.totalNodes, 10, 'Stats should have totalNodes');
  assert.strictEqual(layoutStats.totalEdges, 15, 'Stats should have totalEdges');
  assert.strictEqual(layoutStats.totalContainers, 2, 'Stats should have totalContainers');
  assert.strictEqual(layoutStats.layoutDuration, 150, 'Stats should have layoutDuration');
  assert(layoutStats.validationResult, 'Stats should have validationResult');
  assert(layoutStats.cacheStats, 'Stats should have cacheStats');
  
  // Test LayoutEventData
  const eventData: LayoutEventData = {
    type: 'complete',
    progress: 100,
    statistics: layoutStats
  };
  
  assert.strictEqual(eventData.type, 'complete', 'EventData should have type');
  assert.strictEqual(eventData.progress, 100, 'EventData should have progress');
  assert.strictEqual(eventData.statistics, layoutStats, 'EventData should have statistics');
  
  // Test LayoutEventCallback
  const eventCallback: LayoutEventCallback = (data: LayoutEventData) => {
    assert(data, 'Callback should receive data');
    assert('type' in data, 'Callback data should have type');
  };
  
  // Test callback execution
  eventCallback(eventData);
  
  console.log('✅ Layout engine types test passed');
}

/**
 * Test interface compatibility
 */
function testInterfaceCompatibility(): void {
  console.log('Testing interface compatibility...');
  
  // Create a mock basic layout engine
  const basicEngine: LayoutEngine = {
    async layout(nodes, edges, containers, hyperEdges, config) {
      return {
        nodes: nodes.map(node => ({
          ...node,
          x: Math.random() * 500,
          y: Math.random() * 500,
          width: 180,
          height: 60
        })),
        edges: edges.map(edge => ({ ...edge, points: [] })),
        containers: containers.map(container => ({
          ...container,
          x: Math.random() * 500,
          y: Math.random() * 500,
          width: 400,
          height: 300
        })),
        hyperEdges: hyperEdges.map(hyperEdge => ({ ...hyperEdge, points: [] }))
      };
    }
  };
  
  // Test that basic engine satisfies interface
  assert(typeof basicEngine.layout === 'function', 'BasicEngine should have layout method');
  
  // Create a mock advanced layout engine
  const advancedEngine: AdvancedLayoutEngine = {
    ...basicEngine,
    setOptions(options: LayoutEngineOptions) {},
    getOptions() {
      return { enableCaching: false, enableValidation: false, logLevel: 'none' };
    },
    clearCache() {},
    getCacheStatistics() {
      return { size: 0 };
    },
    validateLayout(result: LayoutResult) {
      return { isValid: true, errors: [], warnings: [] };
    },
    on(event: 'layout', callback: LayoutEventCallback) {},
    off(event: 'layout', callback: LayoutEventCallback) {},
    getLastLayoutStatistics() {
      return null;
    }
  };
  
  // Test that advanced engine satisfies interface
  assert(typeof advancedEngine.layout === 'function', 'AdvancedEngine should have layout method');
  assert(typeof advancedEngine.setOptions === 'function', 'AdvancedEngine should have setOptions method');
  assert(typeof advancedEngine.getOptions === 'function', 'AdvancedEngine should have getOptions method');
  assert(typeof advancedEngine.clearCache === 'function', 'AdvancedEngine should have clearCache method');
  assert(typeof advancedEngine.getCacheStatistics === 'function', 'AdvancedEngine should have getCacheStatistics method');
  assert(typeof advancedEngine.validateLayout === 'function', 'AdvancedEngine should have validateLayout method');
  assert(typeof advancedEngine.on === 'function', 'AdvancedEngine should have on method');
  assert(typeof advancedEngine.off === 'function', 'AdvancedEngine should have off method');
  assert(typeof advancedEngine.getLastLayoutStatistics === 'function', 'AdvancedEngine should have getLastLayoutStatistics method');
  
  console.log('✅ Interface compatibility test passed');
}

/**
 * Test type constraints and edge cases
 */
function testTypeConstraints(): void {
  console.log('Testing type constraints...');
  
  // Test optional properties
  const minimalConfig: LayoutConfig = {};
  assert(typeof minimalConfig === 'object', 'Minimal config should be valid');
  
  const partialConfig: LayoutConfig = {
    spacing: 100
  };
  assert.strictEqual(partialConfig.spacing, 100, 'Partial config should work');
  
  // Test positioned elements without optional properties
  const minimalPositionedEdge: PositionedEdge = {
    id: 'edge1',
    source: 'node1',
    target: 'node2',
    style: 'default',
    hidden: false
    // points is optional
  };
  assert.strictEqual(minimalPositionedEdge.id, 'edge1', 'Minimal positioned edge should work');
  
  const minimalPositionedHyperEdge: PositionedHyperEdge = {
    id: 'hyperedge1',
    source: 'container1',
    target: 'node1',
    style: 'default',
    aggregatedEdges: []
    // points is optional
  };
  assert.strictEqual(minimalPositionedHyperEdge.id, 'hyperedge1', 'Minimal positioned hyperedge should work');
  
  // Test validation result with empty arrays
  const emptyValidation: LayoutValidationResult = {
    isValid: true,
    errors: [],
    warnings: []
  };
  assert.strictEqual(emptyValidation.isValid, true, 'Empty validation should work');
  assert.strictEqual(emptyValidation.errors.length, 0, 'Empty errors should work');
  assert.strictEqual(emptyValidation.warnings.length, 0, 'Empty warnings should work');
  
  console.log('✅ Type constraints test passed');
}

/**
 * Run all layout types tests
 */
export function runAllTests(): Promise<void> {
  console.log('🧪 Running Layout Types Tests');
  console.log('=============================\n');
  
  return new Promise((resolve, reject) => {
    try {
      testBasicLayoutTypes();
      testPositionedElementTypes();
      testLayoutResultType();
      testValidationTypes();
      testLayoutEngineTypes();
      testInterfaceCompatibility();
      testTypeConstraints();
      
      console.log('\n🎉 All layout types tests passed!');
      console.log('✅ Type system is working correctly!');
      resolve();
    } catch (error: unknown) {
      console.error('\n❌ Layout types test failed:', error instanceof Error ? error.message : String(error));
      if (error instanceof Error) {
        console.error(error.stack);
      }
      reject(error);
    }
  });
}

// Export individual test functions for selective testing
export {
  testBasicLayoutTypes,
  testPositionedElementTypes,
  testLayoutResultType,
  testValidationTypes,
  testLayoutEngineTypes,
  testInterfaceCompatibility,
  testTypeConstraints
};

// Run tests if this file is executed directly
if (import.meta.url === `file://${process.argv[1]}`) {
  runAllTests().catch(() => process.exit(1));
}
