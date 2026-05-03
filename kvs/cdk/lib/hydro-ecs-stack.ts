import { CfnOutput, Duration, RemovalPolicy, Stack, StackProps } from "aws-cdk-lib";
import * as ec2 from "aws-cdk-lib/aws-ec2";
import * as ecs from "aws-cdk-lib/aws-ecs";
import * as elbv2 from "aws-cdk-lib/aws-elasticloadbalancingv2";
import * as iam from "aws-cdk-lib/aws-iam";
import * as logs from "aws-cdk-lib/aws-logs";
import { Construct } from "constructs";
import * as fs from "node:fs";
import * as path from "node:path";

// ---------------------------------------------------------------------------
// Manifest types — mirrors the JSON emitted by Hydro's EcsDeploy::export()
// ---------------------------------------------------------------------------

interface PortInfo {
    protocol: "tcp";
    port: number;
}

interface BuildConfig {
    project_dir: string;
    target_dir: string;
    bin_name: string;
    package_name: string;
    features: string[];
}

interface ProcessManifest {
    build: BuildConfig;
    ports: Record<string, PortInfo>;
    task_family: string;
}

interface ClusterManifest {
    build: BuildConfig;
    ports: Record<string, PortInfo>;
    default_count: number;
    task_family_prefix: string;
}

interface HydroManifest {
    processes: Record<string, ProcessManifest>;
    clusters: Record<string, ClusterManifest>;
}

// ---------------------------------------------------------------------------
// Stack properties
// ---------------------------------------------------------------------------

export interface HydroEcsStackProps extends StackProps {
    /** Path to hydro-manifest.json. */
    manifestPath: string;
    /**
     * Path to the Docker build context directory (containing Dockerfile and bin/).
     * Built by build.sh.
     */
    dockerContext: string;
    /** Override cluster sizes by name (key = cluster name from manifest). */
    clusterSizes?: Record<string, number>;
    /** Fargate task CPU (default 256). */
    cpu?: number;
    /** Fargate task memory in MiB (default 512). */
    memoryMiB?: number;
    /** NLB listener port for external access (default 80). */
    nlbPort?: number;
}

// ---------------------------------------------------------------------------
// Stack
// ---------------------------------------------------------------------------

export class HydroEcsStack extends Stack {
    public readonly cluster: ecs.ICluster;
    public readonly vpc: ec2.IVpc;
    public readonly services: Map<string, ecs.FargateService> = new Map();
    public readonly nlb: elbv2.INetworkLoadBalancer;

    constructor(scope: Construct, id: string, props: HydroEcsStackProps) {
        super(scope, id, props);

        // --- Read manifest ---------------------------------------------------
        const manifest: HydroManifest = JSON.parse(
            fs.readFileSync(props.manifestPath, "utf8")
        );

        const cpu = props.cpu ?? 256;
        const memoryMiB = props.memoryMiB ?? 512;
        const nlbPort = props.nlbPort ?? 80;

        // --- Container image (built from local Dockerfile) -------------------
        const containerImage = ecs.ContainerImage.fromAsset(props.dockerContext);

        // --- VPC -------------------------------------------------------------
        this.vpc = new ec2.Vpc(this, "Vpc", {
            maxAzs: 2,
            natGateways: 0,
            subnetConfiguration: [
                {
                    name: "Public",
                    subnetType: ec2.SubnetType.PUBLIC,
                },
                {
                    name: "Private",
                    subnetType: ec2.SubnetType.PRIVATE_ISOLATED,
                },
            ],
        });

        // --- VPC Endpoints for private subnets (no NAT) ----------------------
        this.vpc.addInterfaceEndpoint("EcrApiEndpoint", {
            service: ec2.InterfaceVpcEndpointAwsService.ECR,
        });
        this.vpc.addInterfaceEndpoint("EcrDkrEndpoint", {
            service: ec2.InterfaceVpcEndpointAwsService.ECR_DOCKER,
        });
        this.vpc.addGatewayEndpoint("S3Endpoint", {
            service: ec2.GatewayVpcEndpointAwsService.S3,
        });
        this.vpc.addInterfaceEndpoint("EcsEndpoint", {
            service: ec2.InterfaceVpcEndpointAwsService.ECS,
        });
        this.vpc.addInterfaceEndpoint("EcsAgentEndpoint", {
            service: ec2.InterfaceVpcEndpointAwsService.ECS_AGENT,
        });
        this.vpc.addInterfaceEndpoint("EcsTelemetryEndpoint", {
            service: ec2.InterfaceVpcEndpointAwsService.ECS_TELEMETRY,
        });
        this.vpc.addInterfaceEndpoint("LogsEndpoint", {
            service: ec2.InterfaceVpcEndpointAwsService.CLOUDWATCH_LOGS,
        });
        this.vpc.addInterfaceEndpoint("Ec2Endpoint", {
            service: ec2.InterfaceVpcEndpointAwsService.EC2,
        });

        // --- ECS Cluster -----------------------------------------------------
        this.cluster = new ecs.Cluster(this, "Cluster", {
            vpc: this.vpc,
            containerInsights: true,
        });

        // --- Security Groups -------------------------------------------------
        const taskSg = new ec2.SecurityGroup(this, "TaskSecurityGroup", {
            vpc: this.vpc,
            description: "Hydro ECS tasks",
            allowAllOutbound: true,
        });
        taskSg.addIngressRule(
            taskSg,
            ec2.Port.allTcp(),
            "Allow TCP between Hydro services"
        );
        taskSg.addIngressRule(
            ec2.Peer.anyIpv4(),
            ec2.Port.allTcp(),
            "Allow NLB-forwarded traffic"
        );

        // --- IAM Roles -------------------------------------------------------
        const executionRole = new iam.Role(this, "ExecutionRole", {
            assumedBy: new iam.ServicePrincipal("ecs-tasks.amazonaws.com"),
            managedPolicies: [
                iam.ManagedPolicy.fromAwsManagedPolicyName(
                    "service-role/AmazonECSTaskExecutionRolePolicy"
                ),
            ],
        });

        // Task role: ECS + EC2 describe for Hydro runtime service discovery
        const taskRole = new iam.Role(this, "TaskRole", {
            assumedBy: new iam.ServicePrincipal("ecs-tasks.amazonaws.com"),
        });
        taskRole.addToPolicy(
            new iam.PolicyStatement({
                effect: iam.Effect.ALLOW,
                actions: [
                    "ecs:ListTasks",
                    "ecs:DescribeTasks",
                    "ec2:DescribeNetworkInterfaces",
                ],
                resources: ["*"],
            })
        );

        // --- Log Group -------------------------------------------------------
        const logGroup = new logs.LogGroup(this, "LogGroup", {
            retention: logs.RetentionDays.ONE_WEEK,
            removalPolicy: RemovalPolicy.DESTROY,
        });

        // --- NLB (internet-facing) -------------------------------------------
        this.nlb = new elbv2.NetworkLoadBalancer(this, "Nlb", {
            vpc: this.vpc,
            internetFacing: true,
            crossZoneEnabled: false,
        });

        let nextListenerPort = nlbPort;

        // --- Create Fargate services for each process ------------------------
        for (const [name, proc] of Object.entries(manifest.processes)) {
            const service = this.createService({
                name: proc.task_family,
                image: containerImage,
                binaryName: proc.build.bin_name,
                ports: proc.ports,
                desiredCount: 1,
                executionRole,
                taskRole,
                securityGroup: taskSg,
                logGroup,
                cpu,
                memoryMiB,
            });
            this.services.set(name, service);

            for (const [portKey, portInfo] of Object.entries(proc.ports)) {
                const listener = this.nlb.addListener(
                    `Listener-${proc.task_family}-${portKey}`,
                    { port: nextListenerPort, protocol: elbv2.Protocol.TCP }
                );
                listener.addTargets(`Target-${proc.task_family}-${portKey}`, {
                    port: portInfo.port,
                    targets: [service],
                    protocol: elbv2.Protocol.TCP,
                    healthCheck: { protocol: elbv2.Protocol.TCP, port: String(portInfo.port) },
                });
                new CfnOutput(this, `NlbEndpoint-${proc.task_family}-${portKey}`, {
                    value: `${this.nlb.loadBalancerDnsName}:${nextListenerPort}`,
                });
                nextListenerPort++;
            }
        }

        // --- Create Fargate services for each cluster ------------------------
        for (const [name, cluster] of Object.entries(manifest.clusters)) {
            const count = props.clusterSizes?.[name] ?? cluster.default_count;

            const service = this.createService({
                name: cluster.task_family_prefix,
                image: containerImage,
                binaryName: cluster.build.bin_name,
                ports: cluster.ports,
                desiredCount: count,
                executionRole,
                taskRole,
                securityGroup: taskSg,
                logGroup,
                cpu,
                memoryMiB,
            });
            this.services.set(name, service);

            for (const [portKey, portInfo] of Object.entries(cluster.ports)) {
                const listener = this.nlb.addListener(
                    `Listener-${cluster.task_family_prefix}-${portKey}`,
                    { port: nextListenerPort, protocol: elbv2.Protocol.TCP }
                );
                listener.addTargets(`Target-${cluster.task_family_prefix}-${portKey}`, {
                    port: portInfo.port,
                    targets: [service],
                    protocol: elbv2.Protocol.TCP,
                    healthCheck: { protocol: elbv2.Protocol.TCP, port: String(portInfo.port) },
                });
                new CfnOutput(this, `NlbEndpoint-${cluster.task_family_prefix}-${portKey}`, {
                    value: `${this.nlb.loadBalancerDnsName}:${nextListenerPort}`,
                });
                nextListenerPort++;
            }
        }

        // --- NLB DNS output --------------------------------------------------
        new CfnOutput(this, "NlbDnsName", {
            value: this.nlb.loadBalancerDnsName,
            description: "NLB DNS name for external access",
        });
    }

    private createService(options: {
        name: string;
        image: ecs.ContainerImage;
        binaryName: string;
        ports: Record<string, PortInfo>;
        desiredCount: number;
        executionRole: iam.IRole;
        taskRole: iam.IRole;
        securityGroup: ec2.ISecurityGroup;
        logGroup: logs.ILogGroup;
        cpu: number;
        memoryMiB: number;
    }): ecs.FargateService {
        const taskDefinition = new ecs.FargateTaskDefinition(
            this,
            `TaskDef-${options.name}`,
            {
                cpu: options.cpu,
                memoryLimitMiB: options.memoryMiB,
                executionRole: options.executionRole,
                taskRole: options.taskRole,
                family: options.name,
            }
        );

        const container = taskDefinition.addContainer("main", {
            image: options.image,
            logging: ecs.LogDrivers.awsLogs({
                logGroup: options.logGroup,
                streamPrefix: options.name,
            }),
            environment: {
                HYDRO_BINARY: options.binaryName,
                CONTAINER_NAME: options.name,
                CLUSTER_NAME: this.cluster.clusterName,
                RUST_LOG: "info,dfir_rs::scheduled=warn,hyper=warn",
                RUST_BACKTRACE: "1",
                NO_COLOR: "1",
            },
        });

        for (const portInfo of Object.values(options.ports)) {
            container.addPortMappings({
                containerPort: portInfo.port,
                protocol: ecs.Protocol.TCP,
            });
        }

        return new ecs.FargateService(this, `Service-${options.name}`, {
            cluster: this.cluster,
            taskDefinition,
            desiredCount: options.desiredCount,
            assignPublicIp: false,
            vpcSubnets: { subnetType: ec2.SubnetType.PRIVATE_ISOLATED },
            securityGroups: [options.securityGroup],
            serviceName: options.name,
        });
    }
}
