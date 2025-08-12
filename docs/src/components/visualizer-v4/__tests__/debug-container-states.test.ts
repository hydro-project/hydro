/**
 * Debug test to check container collapse states and visibility
 */

import { describe, it, expect } from 'vitest';
import { parseGraphJSON } from '../core/JSONParser';
import paxosFlippedData from '../test-data/paxos-flipped.json';

describe('Debug Container States', () => {
  it('should show container collapse and visibility states', async () => {
    const parseResult = parseGraphJSON(paxosFlippedData);
    const visState = parseResult.state;
    
    console.log('🔍 BEFORE COLLAPSE:');
    console.log(`Total containers: ${Array.from((visState as any)._collections.containers.keys()).length}`);
    console.log(`Visible containers: ${visState.visibleContainers.length}`);
    
    // Get the first 3 containers like the test does
    const containerIds = Array.from((visState as any)._collections.containers.keys()) as string[];
    const containersToCollapse = containerIds.slice(0, Math.min(3, containerIds.length));
    
    console.log(`📦 About to collapse: ${containersToCollapse.join(', ')}`);
    
    // Check their initial states
    for (const containerId of containersToCollapse) {
      const container = visState.getContainer(containerId);
      console.log(`Container ${containerId} BEFORE: collapsed=${container.collapsed}, hidden=${container.hidden}`);
    }
    
    // Collapse them
    for (const containerId of containersToCollapse) {
      console.log(`Collapsing ${containerId}...`);
      visState.setContainerState(containerId, { collapsed: true });
      
      const container = visState.getContainer(containerId);
      console.log(`Container ${containerId} AFTER: collapsed=${container.collapsed}, hidden=${container.hidden}`);
    }
    
    console.log('🔍 AFTER COLLAPSE:');
    console.log(`Visible containers: ${visState.visibleContainers.length}`);
    
    // Check which collapsed containers are in visibleContainers
    const visibleContainers = visState.visibleContainers;
    const collapsedVisibleContainers = visibleContainers.filter(c => c.collapsed);
    
    console.log(`Collapsed visible containers: ${collapsedVisibleContainers.length}`);
    collapsedVisibleContainers.forEach(container => {
      console.log(`  - ${container.id}: collapsed=${container.collapsed}, hidden=${container.hidden}`);
    });
    
    // Check if bt_81 and bt_98 are in visibleContainers
    const bt81 = visibleContainers.find(c => c.id === 'bt_81');
    const bt98 = visibleContainers.find(c => c.id === 'bt_98');
    const bt12 = visibleContainers.find(c => c.id === 'bt_12');
    
    console.log(`bt_81 in visibleContainers: ${bt81 ? `YES (collapsed=${bt81.collapsed}, hidden=${bt81.hidden})` : 'NO'}`);
    console.log(`bt_98 in visibleContainers: ${bt98 ? `YES (collapsed=${bt98.collapsed}, hidden=${bt98.hidden})` : 'NO'}`);
    console.log(`bt_12 in visibleContainers: ${bt12 ? `YES (collapsed=${bt12.collapsed}, hidden=${bt12.hidden})` : 'NO'}`);
  });
});
