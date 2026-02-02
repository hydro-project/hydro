import * as cdk from 'aws-cdk-lib';
import * as ec2 from 'aws-cdk-lib/aws-ec2';
import * as ecs from 'aws-cdk-lib/aws-ecs';
import * as iam from 'aws-cdk-lib/aws-iam';
import * as logs from 'aws-cdk-lib/aws-logs';
import * as ecr_assets from 'aws-cdk-lib/aws-ecr-assets';
import { Construct } from 'constructs';
import { HydroManifest, HydroBuildConfig, HydroLocationKey, HydroPortInfo } from './hydro-manifest';
import { execSync } from 'child_process';
import * as fs from 'fs';
import * as path from 'path';

/**
 * Get the current user's external IP address by querying checkip.amazonaws.com.
 * This is used to restrict security group access to only the deployer's IP.
 */
function getExternalIpAddress(): string {
  try {
    const result = execSync('curl -s https://checkip.amazonaws.com', {
      encoding: 'utf-8',
      timeout: 10000,
    });
    const ip = result.trim();
    // Validate it looks like an IP address
    if (!/^\d{1,3}\.\d{1,3}\.\d{1,3}\.\d{1,3}$/.test(ip)) {
      throw new Error(`Invalid IP address format: ${ip}`);
    }
    return ip;
  } catch (error) {
    throw new Error(
      `Failed to get external IP address. Make sure you have internet connectivity. Error: ${error}`
    );
  }
}

export interface HydroDeploymentProps {
  outputDir?: string;
  vpc?: ec2.IVpc;
  clusterSizes?: Record<string, number>;
  cpu?: number;
  memoryMiB?: number;
  assignPublicIp?: boolean;
  releaseMode?: boolean;
  cargoPackage?: string;
  cargoExample?: string;
  workspaceRoot?: string;
}

/**
 * CDK L2 Construct for deploying Hydro applications to ECS Fargate.
 * 
 * This construct generates the Hydro manifest by invoking cargo directly,
 * then creates the necessary ECS infrastructure to run the distributed application.
 */
export class HydroDeployment extends Construct {
  public readonly cluster: ecs.Cluster;
  public readonly vpc: ec2.IVpc;
  public readonly services: Map<string, ecs.FargateService> = new Map();
  public readonly manifest: HydroManifest;

  private readonly outputDir: string;
  private readonly dockerImageCache: Map<string, ecr_assets.DockerImageAsset> = new Map();
  private readonly releaseMode: boolean;
  private readonly cargoPackage: string;
  private readonly cargoExample: string;
  private readonly workspaceRoot: string;
  private readonly deploymentName: string;

  constructor(scope: Construct, id: string, props: HydroDeploymentProps = {}) {
    super(scope, id);

    this.outputDir = props.outputDir ?? path.resolve(__dirname, '../hydro-assets');
    this.releaseMode = props.releaseMode ?? true;
    this.cargoPackage = props.cargoPackage ?? 'hydro_test';
    this.cargoExample = props.cargoExample ?? 'distributed_echo';
    this.workspaceRoot = props.workspaceRoot ?? path.resolve(__dirname, '../..');
    // Use the construct ID as the deployment name for resource naming
    this.deploymentName = id.toLowerCase().replace(/[^a-z0-9-]/g, '-');

    // Generate the manifest by invoking cargo
    this.manifest = this.generateManifest();

    // Create or use existing VPC
    this.vpc = props.vpc ?? new ec2.Vpc(this, 'Vpc', {
      maxAzs: 2,
      natGateways: 1,
    });

    // Create ECS Cluster
    this.cluster = new ecs.Cluster(this, 'Cluster', {
      vpc: this.vpc,
      clusterName: `hydro-${this.deploymentName}`,
      containerInsightsV2: ecs.ContainerInsights.ENABLED,
    });

    // Security Group - allow all internal traffic within VPC
    const securityGroup = new ec2.SecurityGroup(this, 'SecurityGroup', {
      vpc: this.vpc,
      description: 'Security group for Hydro ECS tasks',
      allowAllOutbound: true,
    });
    // Allow all TCP traffic within the security group (for inter-service communication)
    securityGroup.addIngressRule(
      securityGroup,
      ec2.Port.allTcp(),
      'Allow all TCP traffic between Hydro services'
    );
    // Allow external traffic only from the deployer's IP address
    const externalIp = getExternalIpAddress();
    console.log(`Restricting external access to IP: ${externalIp}`);
    securityGroup.addIngressRule(
      ec2.Peer.ipv4(`${externalIp}/32`),
      ec2.Port.allTcp(),
      `Allow TCP traffic from deployer IP (${externalIp})`
    );

    // Task Execution Role (for pulling images, writing logs)
    const executionRole = new iam.Role(this, 'ExecutionRole', {
      assumedBy: new iam.ServicePrincipal('ecs-tasks.amazonaws.com'),
      managedPolicies: [
        iam.ManagedPolicy.fromAwsManagedPolicyName('service-role/AmazonECSTaskExecutionRolePolicy'),
      ],
    });

    // Task Role (for ECS API access - service discovery at runtime)
    const taskRole = new iam.Role(this, 'TaskRole', {
      assumedBy: new iam.ServicePrincipal('ecs-tasks.amazonaws.com'),
    });
    taskRole.addToPolicy(new iam.PolicyStatement({
      effect: iam.Effect.ALLOW,
      actions: [
        'ecs:ListTasks',
        'ecs:DescribeTasks',
        'ec2:DescribeNetworkInterfaces',
      ],
      resources: ['*'],
    }));

    // Log Group for all Hydro services
    const logGroup = new logs.LogGroup(this, 'LogGroup', {
      logGroupName: `/ecs/hydro-${this.deploymentName}`,
      retention: logs.RetentionDays.ONE_WEEK,
      removalPolicy: cdk.RemovalPolicy.DESTROY,
    });

    const cpu = props.cpu ?? 256;
    const memoryMiB = props.memoryMiB ?? 512;
    const assignPublicIp = props.assignPublicIp ?? true;

    // Create services for processes
    for (const [name, process] of Object.entries(this.manifest.processes)) {
      const dockerImage = this.buildDockerImage(name, process.build);
      
      const service = this.createService({
        name: process.task_family,
        dockerImage,
        locationKey: process.location_key,
        ports: process.ports,
        clusterName: this.cluster.clusterName,
        desiredCount: 1,
        executionRole,
        taskRole,
        securityGroup,
        logGroup,
        cpu,
        memoryMiB,
        assignPublicIp,
      });
      this.services.set(name, service);
    }

    // Create services for clusters
    for (const [name, cluster] of Object.entries(this.manifest.clusters)) {
      const count = props.clusterSizes?.[name] ?? cluster.default_count;
      const dockerImage = this.buildDockerImage(name, cluster.build);

      const service = this.createService({
        name: cluster.task_family_prefix,
        dockerImage,
        locationKey: cluster.location_key,
        ports: cluster.ports,
        clusterName: this.cluster.clusterName,
        desiredCount: count,
        executionRole,
        taskRole,
        securityGroup,
        logGroup,
        cpu,
        memoryMiB,
        assignPublicIp,
      });
      this.services.set(name, service);
    }

    // Output useful information
    new cdk.CfnOutput(this, 'ClusterName', {
      value: this.cluster.clusterName,
      description: 'ECS Cluster Name',
    });

    new cdk.CfnOutput(this, 'DeploymentName', {
      value: this.deploymentName,
      description: 'Hydro Deployment Name',
    });
  }

  private generateManifest(): HydroManifest {
    console.log('Generating Hydro manifest...');
    console.log(`  Workspace: ${this.workspaceRoot}`);
    console.log(`  Package: ${this.cargoPackage}`);
    console.log(`  Example: ${this.cargoExample}`);
    console.log(`  Output: ${this.outputDir}`);

    // Ensure output directory exists
    fs.mkdirSync(this.outputDir, { recursive: true });

    const cargoCmd = `cargo run --locked -p ${this.cargoPackage} --features ecs --example ${this.cargoExample} -- --mode cdk-export --output ${this.outputDir}`;

    try {
      execSync(cargoCmd, {
        cwd: this.workspaceRoot,
        stdio: 'inherit',
        env: {
          ...process.env,
          RUST_LOG: 'info',
        },
      });
    } catch (error) {
      throw new Error(`Failed to generate Hydro manifest: ${error}`);
    }

    const manifestPath = path.join(this.outputDir, 'hydro-manifest.json');
    if (!fs.existsSync(manifestPath)) {
      throw new Error(`Manifest was not generated at expected path: ${manifestPath}`);
    }

    console.log(`Manifest generated successfully: ${manifestPath}`);
    const manifest: HydroManifest = JSON.parse(fs.readFileSync(manifestPath, 'utf-8'));

    console.log(`\nProcesses: ${Object.keys(manifest.processes).length}`);
    console.log(`Clusters: ${Object.keys(manifest.clusters).length}`);

    return manifest;
  }

  /**
   * Build a Rust binary and create a Docker image for it.
   */
  private buildDockerImage(name: string, build: HydroBuildConfig): ecr_assets.DockerImageAsset {
    // Use bin_name as cache key since same binary can be used by multiple services
    const cacheKey = build.bin_name;
    if (this.dockerImageCache.has(cacheKey)) {
      return this.dockerImageCache.get(cacheKey)!;
    }

    // Create a directory for this binary's Docker build context
    const sanitizedName = name.replace(/[^a-zA-Z0-9_-]/g, '_');
    const dockerContextDir = path.join(this.outputDir, 'docker', sanitizedName);
    fs.mkdirSync(dockerContextDir, { recursive: true });

    // Build the Rust binary using cargo
    console.log(`Building binary: ${build.bin_name}`);
    const featuresArg = build.features.length > 0 ? `--features ${build.features.join(',')}` : '';
    // Normalize paths to avoid double slashes
    const projectDir = path.normalize(build.project_dir);
    const targetDir = path.normalize(build.target_dir);
    // Run cargo from within the project directory - no -p flag needed when running from crate root
    const releaseArg = this.releaseMode ? '--release' : '';
    const profileDir = this.releaseMode ? 'release' : 'debug';
    const cargoCmd = `cargo build --locked ${releaseArg} --example ${build.bin_name} --target-dir ${targetDir} ${featuresArg} --target x86_64-unknown-linux-gnu`;
    
    try {
      execSync(cargoCmd, {
        cwd: projectDir,
        stdio: 'inherit',
        env: {
          ...process.env,
          STAGELEFT_TRYBUILD_BUILD_STAGED: '1',
          // Cross-compilation settings for Linux
          CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_LINKER: 'x86_64-linux-gnu-gcc',
        },
      });
    } catch (error) {
      throw new Error(`Failed to build binary ${build.bin_name}: ${error}`);
    }

    // Copy the built binary to the Docker context
    const binaryPath = path.join(targetDir, 'x86_64-unknown-linux-gnu', profileDir, 'examples', build.bin_name);
    const destBinaryPath = path.join(dockerContextDir, 'app');
    fs.copyFileSync(binaryPath, destBinaryPath);
    fs.chmodSync(destBinaryPath, 0o755);

    // Create Dockerfile
    const dockerfile = `
FROM debian:trixie-slim
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates && rm -rf /var/lib/apt/lists/*
COPY app /app
CMD ["/app"]
`;
    fs.writeFileSync(path.join(dockerContextDir, 'Dockerfile'), dockerfile);

    // Create Docker image asset
    const imageAsset = new ecr_assets.DockerImageAsset(this, `Image-${sanitizedName}`, {
      directory: dockerContextDir,
      platform: ecr_assets.Platform.LINUX_AMD64,
    });

    this.dockerImageCache.set(cacheKey, imageAsset);
    return imageAsset;
  }

  private createService(opts: {
    name: string;
    dockerImage: ecr_assets.DockerImageAsset;
    locationKey: HydroLocationKey;
    ports: Record<string, HydroPortInfo>;
    clusterName: string;
    desiredCount: number;
    executionRole: iam.IRole;
    taskRole: iam.IRole;
    securityGroup: ec2.ISecurityGroup;
    logGroup: logs.ILogGroup;
    cpu: number;
    memoryMiB: number;
    assignPublicIp: boolean;
  }): ecs.FargateService {
    const taskDefinition = new ecs.FargateTaskDefinition(this, `TaskDef-${opts.name}`, {
      cpu: opts.cpu,
      memoryLimitMiB: opts.memoryMiB,
      executionRole: opts.executionRole,
      taskRole: opts.taskRole,
      family: opts.name, // Important: used by ECS API for service discovery
    });

    const container = taskDefinition.addContainer('main', {
      image: ecs.ContainerImage.fromDockerImageAsset(opts.dockerImage),
      logging: ecs.LogDrivers.awsLogs({
        logGroup: opts.logGroup,
        streamPrefix: opts.name,
      }),
      environment: {
        CONTAINER_NAME: opts.name,
        CLUSTER_NAME: opts.clusterName,
        RUST_LOG: 'trace,aws_runtime=info,aws_sdk_ecs=info,aws_smithy_runtime=info,aws_smithy_runtime_api=info,aws_config=info,hyper_util=info,aws_smithy_http_client=info,aws_sigv4=info,dfir_rs=warn',
        RUST_BACKTRACE: '1',
        NO_COLOR: '1',
      },
    });

    for (const portInfo of Object.values(opts.ports)) {
      container.addPortMappings({
        containerPort: portInfo.port,
        protocol: ecs.Protocol.TCP,
      });
    }

    return new ecs.FargateService(this, `Service-${opts.name}`, {
      cluster: this.cluster,
      taskDefinition,
      desiredCount: opts.desiredCount,
      assignPublicIp: opts.assignPublicIp,
      securityGroups: [opts.securityGroup],
      serviceName: opts.name,
    });
  }
}
