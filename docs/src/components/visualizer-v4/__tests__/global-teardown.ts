/**
 * Global Setup/Teardown - displays final status messages after ALL tests complete
 * 
 * This runs after every test in the entire test suite has finished,
 * ensuring the message appears at the very end and is visible to developers.
 */

export function setup() {
  // No setup needed
}

export function teardown() {
  console.log('🧹 Global teardown starting...');
  
  // Check if fuzz tests were skipped 
  const shouldRunFuzzTests = process.env.ENABLE_FUZZ_TESTS === 'true';
  
  if (!shouldRunFuzzTests) {
    console.log('\n' + '='.repeat(80));
    console.log('⏭️  COMPREHENSIVE FUZZ TESTS SKIPPED');
    console.log('   Set ENABLE_FUZZ_TESTS=true to run comprehensive stress tests');
    console.log('   for investigating subtle visualization bugs.');
    console.log('='.repeat(80) + '\n');
  }
  
  console.log('🧹 Global teardown complete.');
}
