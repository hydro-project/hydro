#!/usr/bin/env npx ts-node
/**
 * Generate CloudFormation template fixture for CDK synth tests.
 * This runs the full CDK synth (including cross-compilation) and normalizes
 * the output for reproducible test comparisons.
 * 
 * Usage: npm run generate-cloudformation-fixture
 */

import { execSync } from 'child_process';
import * as path from 'path';
import * as fs from 'fs';
import { normalizeTemplate } from '../lib/normalize-template';

const cdkDir = path.resolve(__dirname, '..');
const fixtureDir = path.resolve(__dirname, '../test/fixtures');

console.log('Generating CloudFormation template fixture...');
console.log(`  CDK Dir: ${cdkDir}`);
console.log(`  Fixture Dir: ${fixtureDir}`);

// Ensure directories exist
fs.mkdirSync(fixtureDir, { recursive: true });

// Run CDK synth with debug builds for faster compilation
console.log('\nRunning CDK synth (this will cross-compile binaries in debug mode)...');
const synthCmd = 'npx cdk synth --quiet';
try {
  execSync(synthCmd, {
    cwd: cdkDir,
    stdio: 'inherit',
    env: {
      ...process.env,
      HYDRO_DEBUG_BUILD: '1',
    },
  });
} catch (error) {
  console.error('Failed to run CDK synth:', error);
  process.exit(1);
}

// Find and normalize the generated template
console.log('\nNormalizing CloudFormation template...');
const cdkOutDir = path.join(cdkDir, 'cdk.out');
const templateFiles = fs.readdirSync(cdkOutDir).filter(f => 
  f === 'HydroStack.template.json'
);

if (templateFiles.length === 0) {
  console.error('No HydroStack.template.json found');
  console.error('Available templates:', fs.readdirSync(cdkOutDir).filter(f => f.endsWith('.template.json')));
  process.exit(1);
}

const templatePath = path.join(cdkOutDir, templateFiles[0]);
console.log(`  Found template: ${templateFiles[0]}`);

const template = JSON.parse(fs.readFileSync(templatePath, 'utf-8'));
const normalizedTemplate = normalizeTemplate(template);

// Write the normalized fixture
const fixtureTemplatePath = path.join(fixtureDir, 'cloudformation-template.json');
fs.writeFileSync(fixtureTemplatePath, JSON.stringify(normalizedTemplate, null, 2) + '\n');

console.log(`\nFixture generated successfully!`);
console.log(`  CloudFormation template: ${fixtureTemplatePath}`);
console.log(`  Resources: ${Object.keys(normalizedTemplate.Resources || {}).length}`);
