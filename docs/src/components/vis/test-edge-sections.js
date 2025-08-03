/**
 * Quick test to verify edge sections are working
 */

import { parseGraphJSON } from './core/JSONParser.js';
import { ELKBridge } from './bridges/ELKBridge.js';
import fs from 'fs';

// Read the chat.json file
const chatJsonData = JSON.parse(fs.readFileSync('../../../../shared_data/chat.json', 'utf8'));

// Parse and layout
console.log('🧪 Testing edge sections preservation...');

const result = parseGraphJSON(chatJsonData, 'location');
const state = result.state;

const elkBridge = new ELKBridge();
await elkBridge.layoutVisState(state);

// Check edges for sections
const edges = state.visibleEdges;
let sectionsFound = 0;

for (const edge of edges) {
  const layout = state.getEdgeLayout(edge.id);
  if (layout?.sections && layout.sections.length > 0) {
    sectionsFound++;
    console.log(`✅ Edge ${edge.id}: ${layout.sections.length} sections`);
  } else {
    console.log(`❌ Edge ${edge.id}: no sections`);
  }
}

console.log(`\n📊 Results: ${sectionsFound}/${edges.length} edges have sections`);

if (sectionsFound > 0) {
  console.log('🎉 SUCCESS: Edge sections are being preserved!');
} else {
  console.log('💥 FAILURE: No edge sections found');
}
