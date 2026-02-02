#!/usr/bin/env node
import * as cdk from 'aws-cdk-lib/core';
import { HydroStack } from '../lib/hydro-stack';

const app = new cdk.App();

// Stack name can be customized via context
const stackName = app.node.tryGetContext('stackName') ?? 'HydroStack';

new HydroStack(app, stackName, {});
