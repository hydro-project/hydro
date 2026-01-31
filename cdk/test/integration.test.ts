/**
 * Integration tests for Hydro CDK deployment.
 * 
 * These tests deploy the stack, connect to the running services via TCP,
 * send test messages, and verify the responses match expected values.
 * 
 * Run with: npm run test:integration
 */

import * as net from 'net';
import {
  ECSClient,
  ListTasksCommand,
  DescribeTasksCommand,
} from '@aws-sdk/client-ecs';
import {
  EC2Client,
  DescribeNetworkInterfacesCommand,
} from '@aws-sdk/client-ec2';
import * as fs from 'fs';
import * as path from 'path';
import { HydroManifest } from '../lib/hydro-manifest';

// Timeout for integration tests (stack should already be deployed)
jest.setTimeout(120000); // 120 seconds total

const MANIFEST_PATH = path.join(__dirname, '../hydro-assets/hydro-manifest.json');

/**
 * Length-delimited JSON protocol helper.
 * Hydro uses a 4-byte big-endian length prefix followed by JSON-serialized data.
 */
class JsonConnection {
  private socket: net.Socket;
  private buffer: Buffer = Buffer.alloc(0);
  private pendingReads: Array<{
    resolve: (data: Buffer) => void;
    reject: (err: Error) => void;
  }> = [];

  constructor(socket: net.Socket) {
    this.socket = socket;
    
    socket.on('data', (chunk) => {
      this.buffer = Buffer.concat([this.buffer, chunk]);
      this.processBuffer();
    });

    socket.on('error', (err) => {
      for (const pending of this.pendingReads) {
        pending.reject(err);
      }
      this.pendingReads = [];
    });
  }

  private processBuffer() {
    // Length-delimited codec uses 4-byte big-endian length prefix
    while (this.buffer.length >= 4) {
      const length = this.buffer.readUInt32BE(0);
      if (this.buffer.length < 4 + length) {
        break; // Wait for more data
      }

      const data = this.buffer.subarray(4, 4 + length);
      this.buffer = this.buffer.subarray(4 + length);

      const pending = this.pendingReads.shift();
      if (pending) {
        pending.resolve(data);
      }
    }
  }

  /**
   * Send a u32 value using JSON serialization
   */
  async sendU32(value: number): Promise<void> {
    // JSON serialize the number
    const payload = Buffer.from(JSON.stringify(value), 'utf-8');

    // Length prefix (4 bytes big-endian)
    const frame = Buffer.alloc(4 + payload.length);
    frame.writeUInt32BE(payload.length, 0);
    payload.copy(frame, 4);

    return new Promise((resolve, reject) => {
      this.socket.write(frame, (err) => {
        if (err) reject(err);
        else resolve();
      });
    });
  }

  /**
   * Receive and parse a u32 response using JSON deserialization
   */
  async receiveU32(timeoutMs: number = 30000): Promise<number> {
    const data = await this.receiveRaw(timeoutMs);
    const jsonStr = data.toString('utf-8');
    return JSON.parse(jsonStr) as number;
  }

  /**
   * Receive raw bytes (one length-delimited frame)
   */
  receiveRaw(timeoutMs: number = 30000): Promise<Buffer> {
    // Check if we already have data buffered
    if (this.buffer.length >= 4) {
      const length = this.buffer.readUInt32BE(0);
      if (this.buffer.length >= 4 + length) {
        const data = this.buffer.subarray(4, 4 + length);
        this.buffer = this.buffer.subarray(4 + length);
        return Promise.resolve(data);
      }
    }

    return new Promise((resolve, reject) => {
      const timeout = setTimeout(() => {
        const idx = this.pendingReads.findIndex(p => p.resolve === resolve);
        if (idx !== -1) {
          this.pendingReads.splice(idx, 1);
        }
        reject(new Error(`Timeout waiting for response after ${timeoutMs}ms`));
      }, timeoutMs);

      this.pendingReads.push({
        resolve: (data) => {
          clearTimeout(timeout);
          resolve(data);
        },
        reject: (err) => {
          clearTimeout(timeout);
          reject(err);
        },
      });
    });
  }

  close() {
    this.socket.destroy();
  }
}

/**
 * Get the public IP of an ECS task by task family name
 */
async function getTaskPublicIp(
  clusterName: string,
  taskFamily: string,
  region: string = 'us-east-1'
): Promise<string> {
  const ecsClient = new ECSClient({ region });
  const ec2Client = new EC2Client({ region });

  // Find task ARN
  console.log(`Looking for task with family: ${taskFamily} in cluster: ${clusterName}`);
  
  let taskArn: string | undefined;
  for (let i = 0; i < 30; i++) {
    const listResult = await ecsClient.send(
      new ListTasksCommand({
        cluster: clusterName,
        family: taskFamily,
      })
    );

    if (listResult.taskArns && listResult.taskArns.length > 0) {
      taskArn = listResult.taskArns[0];
      break;
    }

    console.log(`Waiting for task ${taskFamily}...`);
    await sleep(5000);
  }

  if (!taskArn) {
    throw new Error(`Task not found for family: ${taskFamily}`);
  }
  console.log(`Found task: ${taskArn}`);

  // Get ENI ID from task
  let eniId: string | undefined;
  for (let i = 0; i < 30; i++) {
    const describeResult = await ecsClient.send(
      new DescribeTasksCommand({
        cluster: clusterName,
        tasks: [taskArn],
      })
    );

    const task = describeResult.tasks?.[0];
    if (task?.lastStatus === 'RUNNING') {
      const eni = task.attachments
        ?.flatMap((a) => a.details ?? [])
        .find((d) => d.name === 'networkInterfaceId');
      
      if (eni?.value) {
        eniId = eni.value;
        break;
      }
    }

    console.log(`Waiting for task ${taskFamily} to be RUNNING (current: ${task?.lastStatus})...`);
    await sleep(5000);
  }

  if (!eniId) {
    throw new Error(`ENI not found for task: ${taskArn}`);
  }
  console.log(`Found ENI: ${eniId}`);

  // Get public IP from ENI
  let publicIp: string | undefined;
  for (let i = 0; i < 30; i++) {
    const eniResult = await ec2Client.send(
      new DescribeNetworkInterfacesCommand({
        NetworkInterfaceIds: [eniId],
      })
    );

    publicIp = eniResult.NetworkInterfaces?.[0]?.Association?.PublicIp;
    if (publicIp) {
      break;
    }

    console.log(`Waiting for public IP on ENI ${eniId}...`);
    await sleep(5000);
  }

  if (!publicIp) {
    throw new Error(`Public IP not found for ENI: ${eniId}`);
  }
  console.log(`Found public IP: ${publicIp}`);

  return publicIp;
}

function sleep(ms: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

/**
 * Connect to a TCP endpoint with retries
 */
async function connectWithRetry(
  host: string,
  port: number,
  maxRetries: number = 10
): Promise<net.Socket> {
  for (let i = 0; i < maxRetries; i++) {
    try {
      const socket = await new Promise<net.Socket>((resolve, reject) => {
        const s = net.createConnection({ host, port }, () => {
          resolve(s);
        });
        s.on('error', reject);
      });
      return socket;
    } catch (err) {
      console.log(`Connection attempt ${i + 1}/${maxRetries} failed, retrying...`);
      await sleep(2000);
    }
  }
  throw new Error(`Failed to connect to ${host}:${port} after ${maxRetries} attempts`);
}

describe('Hydro CDK Integration Tests', () => {
  let manifest: HydroManifest;
  // Cluster name is based on the CDK construct ID (HydroDeployment -> hydro-hydrodeployment)
  const clusterName = 'hydro-hydrodeployment';

  beforeAll(async () => {
    // Load the manifest to get deployment info
    if (!fs.existsSync(MANIFEST_PATH)) {
      throw new Error(`Manifest not found at ${MANIFEST_PATH}. Run 'npx cdk deploy' first.`);
    }
    manifest = JSON.parse(fs.readFileSync(MANIFEST_PATH, 'utf-8'));
    
    console.log(`Using cluster: ${clusterName}`);
  });

  afterAll(async () => {
    // Never cleanup - stack management is done separately
    console.log('Test complete. Stack cleanup should be done manually with: npx cdk destroy');
  });

  it('should have deployed all processes', () => {
    expect(manifest.processes).toBeDefined();
    expect(Object.keys(manifest.processes).length).toBeGreaterThan(0);
    console.log('Processes:', Object.keys(manifest.processes));
  });

  it('should have deployed all clusters', () => {
    expect(manifest.clusters).toBeDefined();
    expect(Object.keys(manifest.clusters).length).toBeGreaterThan(0);
    console.log('Clusters:', Object.keys(manifest.clusters));
  });

  it('should respond to echo messages through bidi port on P1', async () => {
    // Find P1 process (has the bidi port for external communication)
    const p1Entry = Object.entries(manifest.processes).find(([name]) =>
      name.includes('P1')
    );
    
    if (!p1Entry) {
      throw new Error('P1 process not found in manifest');
    }

    const [p1Name, p1Config] = p1Entry;
    
    console.log(`Found P1: ${p1Name}`);
    console.log(`P1 ports:`, p1Config.ports);

    // Get public IP for P1
    const p1Ip = await getTaskPublicIp(clusterName, p1Config.task_family);
    
    // Find the bidi port on P1 (should be the only port)
    const p1Port = Object.entries(p1Config.ports)[0];
    if (!p1Port) {
      throw new Error('No port found on P1');
    }
    const bidiPort = p1Port[1].port;
    
    console.log(`Connecting to P1 bidi port at ${p1Ip}:${bidiPort}`);

    // Connect to the bidi port
    const socket = await connectWithRetry(p1Ip, bidiPort);
    const conn = new JsonConnection(socket);

    try {
      // The echo chain adds 4 to each value:
      // external -> P1 (+1) -> C2 (+1) -> C3 (+1) -> P4 (+1) -> back to P1 -> external
      // So input 0 should return 4, input 1 should return 5, etc.

      console.log('Sending test message: 0');
      await conn.sendU32(0);
      
      const response1 = await conn.receiveU32(60000);
      console.log(`Received response: ${response1}`);
      expect(response1).toBe(4);

      console.log('Sending test message: 1');
      await conn.sendU32(1);
      
      const response2 = await conn.receiveU32(30000);
      console.log(`Received response: ${response2}`);
      expect(response2).toBe(5);

      console.log('Sending test message: 100');
      await conn.sendU32(100);
      
      const response3 = await conn.receiveU32(30000);
      console.log(`Received response: ${response3}`);
      expect(response3).toBe(104);

      console.log('All echo tests passed!');
    } finally {
      conn.close();
    }
  });
});
