/**
 * CDK synth integration test.
 * 
 * This test verifies that `cdk synth` produces consistent CloudFormation templates
 * by comparing against a pre-checked-in fixture. The test runs the full CDK synth
 * including cross-compilation of Rust binaries.
 * 
 * To update the fixture after intentional changes: npm run generate-cloudformation-fixture
 */

import { execSync } from 'child_process';
import * as fs from 'fs';
import * as path from 'path';
import * as os from 'os';
import { normalizeTemplate } from '../lib/normalize-template';

const fixtureDir = path.join(__dirname, 'fixtures');
const fixtureTemplatePath = path.join(fixtureDir, 'cloudformation-template.json');
const cdkExampleDir = path.resolve(__dirname, '..');

describe('CDK synth integration', () => {
  // Increase timeout for cross-compilation (10 minutes)
  jest.setTimeout(600000);

  test('generated CloudFormation template matches committed fixture', () => {
    // Skip if fixture doesn't exist yet
    if (!fs.existsSync(fixtureTemplatePath)) {
      console.warn('CloudFormation fixture not found. Run: npm run generate-cloudformation-fixture');
      return;
    }

    // Create a temp directory for CDK output
    const tempCdkOut = fs.mkdtempSync(path.join(os.tmpdir(), 'hydro-cdk-synth-test-'));
    
    try {
      // Run CDK synth with debug builds for faster compilation
      // The manifest is now generated automatically by the HydroDeployment construct
      console.log('Running CDK synth (cross-compiling binaries in debug mode)...');
      const synthCmd = `npx cdk synth --quiet --output ${tempCdkOut}`;
      
      try {
        execSync(synthCmd, {
          cwd: cdkExampleDir,
          stdio: 'pipe',
          env: {
            ...process.env,
            HYDRO_DEBUG_BUILD: '1',
          },
        });
      } catch (error: any) {
        const stderr = error.stderr?.toString() || '';
        const stdout = error.stdout?.toString() || '';
        throw new Error(`CDK synth failed:\n${stderr}\n${stdout}`);
      }

      // Find the generated template
      const templateFiles = fs.readdirSync(tempCdkOut).filter(f => 
        f === 'HydroStack.template.json'
      );

      expect(templateFiles.length).toBeGreaterThan(0);
      
      const templatePath = path.join(tempCdkOut, templateFiles[0]);
      const generatedTemplate = JSON.parse(fs.readFileSync(templatePath, 'utf-8'));
      const expectedTemplate = JSON.parse(fs.readFileSync(fixtureTemplatePath, 'utf-8'));

      // Normalize both templates
      const normalizedGenerated = normalizeTemplate(generatedTemplate);

      // Compare
      expect(normalizedGenerated).toEqual(expectedTemplate);
      
      console.log('CloudFormation template matches fixture!');
    } finally {
      // Cleanup temp directory
      fs.rmSync(tempCdkOut, { recursive: true, force: true });
    }
  });
});
