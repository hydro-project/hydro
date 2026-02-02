import { Stack, StackProps } from 'aws-cdk-lib/core';
import { Construct } from 'constructs';
import { HydroDeployment } from './hydro-deployment';

export interface HydroStackProps extends StackProps {
}

export class HydroStack extends Stack {
  public readonly hydroDeployment: HydroDeployment;

  constructor(scope: Construct, id: string, props?: HydroStackProps) {
    super(scope, id, props);

    const releaseMode = process.env.HYDRO_DEBUG_BUILD !== '1';
    this.hydroDeployment = new HydroDeployment(this, 'HydroDeployment', {
      releaseMode,
    });
  }
}
