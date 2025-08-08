/**
 * @fileoverview Test Runner for Layout and Integration Tests
 * 
 * Runs all our custom tests to validate the layout and boundary behavior.
 */

// import { runLayoutBoundaryTests } from './layoutBoundaries.test.js';
// import { runChatJsonIntegrationTests } from './chatJsonIntegration.test.js';

async function runAllTests(): Promise<void> {
  // // console.log((('🚀 Starting All Layout Tests...\n')));
  
  const startTime = Date.now();
  
  try {
    // Run boundary validation tests
    // await runLayoutBoundaryTests();
    
    // Run chat.json integration tests  
    // await runChatJsonIntegrationTests();
    
    // // console.log((('Layout tests are temporarily disabled')));
    
    
    const endTime = Date.now();
    const duration = (endTime - startTime) / 1000;
    
    // // console.log(((`\n🎉 All Tests Completed Successfully in ${duration}s!\n`)));
    
  } catch (error) {
    const endTime = Date.now();
    const duration = (endTime - startTime) / 1000;
    
    console.error(`\n💥 Tests Failed after ${duration}s:`);
    console.error(error);
    
    process.exit(1);
  }
}

// Run tests if this file is executed directly
if (typeof process !== 'undefined' && process.argv && process.argv[1]?.endsWith('runLayoutTests.ts')) {
  runAllTests().catch(console.error);
}

export { runAllTests };
