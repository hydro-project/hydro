/**
 * Test Runner for Vis Components
 * 
 * Runs all tests for the visualization system
 */

import { runAllTests as runVisStateTests } from './VisState.test.js';
import { runAllTests as runConstantsTests } from './constants.test.js';
import { runAllTests as runJSONParserTests } from './JSONParser.test.js';
import { runAllTests as runSymmetricInverseTests } from './symmetricInverse.test.js';
import { runAllTests as runEdgeIndexEncapsulationTests } from './edgeIndexEncapsulation.test.js';

console.log('🧪 Running Vis Component Test Suite\n');
console.log('=====================================\n');

async function runAllTests() {
  let totalTests = 0;
  let passedTests = 0;
  
  try {
    console.log('📊 Running Constants Tests...');
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
    
    console.log('\n=====================================');
    console.log(`🎉 Test Suite Complete: ${passedTests}/${totalTests} test modules passed`);
    console.log('All visualization components are working correctly!');
    console.log('✅ All symmetric function pairs verified as mathematical inverses!');
    console.log('\n💡 To run integration/fuzz tests: node --experimental-modules integration.test.js');
    console.log('💡 To run fuzz tests: node --experimental-modules fuzzTest.js');
    
  } catch (error) {
    totalTests++;
    console.error('\n=====================================');
    console.error(`❌ Test Suite Failed: ${passedTests}/${totalTests} test modules passed`);
    console.error('Error:', error.message);
    process.exit(1);
  }
}

// Run all tests
runAllTests();
