/**
 *import { runAllTests as runVisStateTests } from './VisState.test.js';
import { runAllTests as runConstantsTests } from './constants.test.js';
import { runAllTests as runJSONParserTests } from './JSONParser.test.js';
import { runAllTests as runSymmetricInverseTests } from './symmetricInverse.test.js';
import { runAllTests as runEdgeIndexEncapsulationTests } from './edgeIndexEncapsulation.test.js';
import { runAllTests as runSimpleGroundingTests } from './simpleGroundingTest.js';
import { runAllTests as runIntegrationTests } from '../alpha/__tests__/integration.test.js';
import { runAllTests as runLayoutIntegrationTests } from '../alpha/layout/__tests__/integration.test.js';
import { runAllTests as runRenderTests } from '../alpha/__tests__/render.test.js';
import { runAllBridgeTests } from '../bridges/__tests__/index.test.js';er for Vis Components
 * 
 * Runs all tests for the visualization system
 */

import { runAllTests as runVisStateTests } from './VisState.test.js';
import { runAllTests as runConstantsTests } from './constants.test.js';
import { runAllTests as runJSONParserTests } from './JSONParser.test.js';
import { runAllTests as runSymmetricInverseTests } from './symmetricInverse.test.js';
import { runAllTests as runEdgeIndexEncapsulationTests } from './edgeIndexEncapsulation.test.js';
import { runAllTests as runSimpleGroundingTests } from './simpleGroundingTest.js';
import { runAllTests as runIntegrationTests } from './integration.test.js';
import { runAllTests as runLayoutIntegrationTests } from '../alpha/layout/__tests__/integration.test.js';
import { runAllTests as runRenderTests } from '../alpha/__tests__/render.test.js';
import { runAllBridgeTests } from '../bridges/__tests__/index.test.js';

console.log('🧪 Running Vis Component Test Suite\n');
console.log('=====================================\n');

async function runAllTests(): Promise<void> {
  let totalTests = 0;
  let passedTests = 0;
  
  try {
    console.log('\n🎨 Running Render Component Tests...');
    await runRenderTests();
    passedTests++;
    totalTests++;
    
    console.log('\n📊 Running Constants Tests...');
    await runConstantsTests();
    passedTests++;
    totalTests++;
    
    console.log('\n📈 Running VisualizationState Tests...');
    await runVisStateTests();
    passedTests++;
    totalTests++;
    
    console.log('\n📄 Running JSONParser Tests...');
    await runJSONParserTests();
    passedTests++;
    totalTests++;
    
    console.log('\n🔄 Running Symmetric Inverse Tests...');
    await runSymmetricInverseTests();
    passedTests++;
    totalTests++;
    
    console.log('\n🔗 Running Edge Index Encapsulation Tests...');
    await runEdgeIndexEncapsulationTests();
    passedTests++;
    totalTests++;
    
    console.log('\n🧪 Running Simple Grounding Tests...');
    await runSimpleGroundingTests();
    passedTests++;
    totalTests++;
    
    console.log('\n🔧 Running Integration Tests...');
    await runIntegrationTests();
    passedTests++;
    totalTests++;
    
    console.log('\n🎨 Running Layout Integration Tests...');
    await runLayoutIntegrationTests();
    passedTests++;
    totalTests++;
    
    console.log('\n🌉 Running Bridge Tests...');
    runAllBridgeTests();
    passedTests++;
    totalTests++;
    
    console.log('\n=====================================');
    console.log(`🎉 Test Suite Complete: ${passedTests}/${totalTests} test modules passed`);
    console.log('All visualization components are working correctly!');
    console.log('✅ All symmetric function pairs verified as mathematical inverses!');
    console.log('✅ All integration tests with real data passed!');
    console.log('✅ All render components tested and working!');
    console.log('✅ All bridge components tested and working!');
    console.log('\n💡 To run fuzz tests separately: node __tests__/fuzzTest.js');
    
  } catch (error: unknown) {
    totalTests++;
    console.error('\n=====================================');
    console.error(`❌ Test Suite Failed: ${passedTests}/${totalTests} test modules passed`);
    console.error('Error:', error instanceof Error ? error.message : String(error));
    process.exit(1);
  }
}

// Run all tests
runAllTests();
