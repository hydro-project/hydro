/**
 * Unit Tests for Individual Render Components  
 * 
 * Tests for specific render directory components that can be tested in isolation
 */

import * as assert from 'assert';
import { createNodeEventHandlers, createEdgeEventHandlers } from '../render/eventHandlers.js';
import { DEFAULT_RENDER_CONFIG } from '../render/config.js';

// ============ Event Handler Tests ============

console.log('Running Render Component tests...');

function testEventHandlers(): void {
  console.log('Testing event handlers...');
  
  let nodeClickCalled = false;
  let edgeClickCalled = false;
  let nodeDoubleClickCalled = false;
  let contextMenuCalled = false;
  
  // Test node event handlers
  const nodeData = {
    onNodeClick: (id: string) => { nodeClickCalled = true; },
    onNodeDoubleClick: (id: string) => { nodeDoubleClickCalled = true; },
    onNodeContextMenu: (id: string, event: any) => { contextMenuCalled = true; }
  };
  
  const nodeHandlers = createNodeEventHandlers('test-node', nodeData);
  
  // Create mock events
  const mockEvent = {
    stopPropagation: () => {},
    preventDefault: () => {}
  };
  
  nodeHandlers.handleClick(mockEvent as any);
  assert.ok(nodeClickCalled, 'Node click handler should be called');
  
  nodeHandlers.handleDoubleClick(mockEvent as any);
  assert.ok(nodeDoubleClickCalled, 'Node double click handler should be called');
  
  nodeHandlers.handleContextMenu(mockEvent as any);
  assert.ok(contextMenuCalled, 'Node context menu handler should be called');
  
  // Test edge event handlers
  const edgeData = {
    onEdgeClick: (id: string) => { edgeClickCalled = true; }
  };
  
  const edgeHandlers = createEdgeEventHandlers('test-edge', edgeData);
  edgeHandlers.handleClick(mockEvent as any);
  assert.ok(edgeClickCalled, 'Edge click handler should be called');
  
  console.log('✓ Event handler tests passed');
}

// ============ Render Configuration Tests ============

function testRenderConfiguration(): void {
  console.log('Testing render configuration...');
  
  // Test default config
  assert.ok(typeof DEFAULT_RENDER_CONFIG === 'object', 'Default config should be an object');
  assert.ok(typeof DEFAULT_RENDER_CONFIG.enableMiniMap === 'boolean', 'enableMiniMap should be boolean');
  assert.ok(typeof DEFAULT_RENDER_CONFIG.enableControls === 'boolean', 'enableControls should be boolean');
  assert.ok(typeof DEFAULT_RENDER_CONFIG.fitView === 'boolean', 'fitView should be boolean');
  assert.ok(typeof DEFAULT_RENDER_CONFIG.nodesDraggable === 'boolean', 'nodesDraggable should be boolean');
  assert.ok(typeof DEFAULT_RENDER_CONFIG.snapToGrid === 'boolean', 'snapToGrid should be boolean');
  assert.ok(typeof DEFAULT_RENDER_CONFIG.gridSize === 'number', 'gridSize should be number');
  
  // Test that defaults provide sensible values
  assert.strictEqual(DEFAULT_RENDER_CONFIG.enableMiniMap, true, 'MiniMap should be enabled by default');
  assert.strictEqual(DEFAULT_RENDER_CONFIG.enableControls, true, 'Controls should be enabled by default');
  assert.strictEqual(DEFAULT_RENDER_CONFIG.fitView, true, 'fitView should be enabled by default');
  assert.ok(DEFAULT_RENDER_CONFIG.gridSize > 0, 'Grid size should be positive');
  
  console.log('✓ Render configuration tests passed');
}

// ============ Test for Node Event Handler Factory ============

function testNodeEventHandlerFactory(): void {
  console.log('Testing node event handler factory...');
  
  // Test with no data
  const handlersEmpty = createNodeEventHandlers('test-node');
  
  const mockEvent = {
    stopPropagation: () => {},
    preventDefault: () => {}
  };
  
  // Should not throw even with no data
  handlersEmpty.handleClick(mockEvent as any);
  handlersEmpty.handleDoubleClick(mockEvent as any);
  handlersEmpty.handleContextMenu(mockEvent as any);
  
  console.log('✓ Node event handler factory tests passed');
}

// ============ Test for Edge Event Handler Factory ============

function testEdgeEventHandlerFactory(): void {
  console.log('Testing edge event handler factory...');
  
  // Test with no data
  const handlersEmpty = createEdgeEventHandlers('test-edge');
  
  const mockEvent = {
    stopPropagation: () => {},
    preventDefault: () => {}
  };
  
  // Should not throw even with no data
  handlersEmpty.handleClick(mockEvent as any);
  handlersEmpty.handleContextMenu(mockEvent as any);
  
  console.log('✓ Edge event handler factory tests passed');
}

// ============ Test Configuration Completeness ============

function testConfigurationCompleteness(): void {
  console.log('Testing configuration completeness...');
  
  const requiredFields = [
    'enableMiniMap',
    'enableControls', 
    'fitView',
    'nodesDraggable',
    'snapToGrid',
    'gridSize',
    'nodesConnectable',
    'elementsSelectable',
    'enableZoom',
    'enablePan',
    'enableSelection'
  ];
  
  for (const field of requiredFields) {
    assert.ok(field in DEFAULT_RENDER_CONFIG, `Config should have ${field} property`);
    assert.ok(DEFAULT_RENDER_CONFIG[field as keyof typeof DEFAULT_RENDER_CONFIG] !== undefined, 
             `Config ${field} should not be undefined`);
  }
  
  console.log('✓ Configuration completeness tests passed');
}

// ============ Run All Tests ============

export async function runAllTests(): Promise<void> {
  console.log('🎨 Running Render Component Tests\n');
  console.log('======================================\n');
  
  try {
    testEventHandlers();
    testRenderConfiguration();
    testNodeEventHandlerFactory();
    testEdgeEventHandlerFactory();
    testConfigurationCompleteness();
    
    console.log('\n======================================');
    console.log('🎉 All render component tests passed!');
    console.log('✅ Event handlers working correctly');
    console.log('✅ Configuration structure verified');
    console.log('✅ Component factories tested');
    
  } catch (error: unknown) {
    console.error('\n======================================');
    console.error('❌ Render component tests failed');
    console.error('Error:', error instanceof Error ? error.message : String(error));
    throw error;
  }
}

// Run tests if this file is executed directly  
if (typeof require !== 'undefined' && require.main === module) {
  runAllTests().catch(() => process.exit(1));
}