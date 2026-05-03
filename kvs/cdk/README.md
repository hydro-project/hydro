# KVS CDK Deployment

Deploys the distributed KVS to AWS ECS Fargate using the Hydro manifest.

## Prerequisites

- AWS CLI configured with credentials
- Node.js 18+
- Docker (for building container images)
- CDK CLI: `npm install -g aws-cdk`

## Usage

### 1. Generate the Hydro manifest

From the workspace root:

```sh
cargo run -p kvs --example kvs -- --mode export --output kvs/hydro-assets
```

### 2. Install CDK dependencies

```sh
cd kvs/cdk
npm install
```

### 3. Deploy

```sh
npx cdk deploy
```

The stack creates:
- A VPC with public + private isolated subnets
- VPC endpoints (ECR, ECS, CloudWatch, EC2) so tasks in private subnets can reach AWS APIs without NAT
- An ECS Fargate cluster with one service per Hydro process/cluster
- An internet-facing NLB exposing the router ports (gRPC on port 80, WebSocket on port 81)
- IAM roles with ECS/EC2 describe permissions for Hydro's runtime service discovery

### 4. Test

Once deployed, the NLB DNS name is in the stack outputs. Use the gRPC or WebSocket client to connect.

### 5. Tear down

```sh
npx cdk destroy
```

## Customization

Override cluster sizes or manifest path via CDK context:

```sh
npx cdk deploy \
  -c manifestPath=/path/to/hydro-manifest.json \
  -c dockerContext=/path/to/workspace/root
```
