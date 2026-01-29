/**
 * Normalize CloudFormation template for reproducible comparisons.
 * Replaces dynamic values (image hashes, IP addresses, CDK qualifiers) with placeholders.
 */
export function normalizeTemplate(obj: any): any {
  if (typeof obj === 'string') {
    // Replace Docker image hashes (64-char hex strings in ECR image URIs)
    if (obj.match(/:[a-f0-9]{64}$/)) {
      return obj.replace(/:[a-f0-9]{64}$/, ':<IMAGE_HASH>');
    }
    // Replace CDK bootstrap qualifier (username-based) with placeholder
    const cdkQualifierPattern = /cdk-[a-zA-Z0-9_-]+-(?=container-assets|assets|deploy-role|cfn-exec-role|lookup-role)/g;
    if (cdkQualifierPattern.test(obj)) {
      return obj.replace(/cdk-[a-zA-Z0-9_-]+-(?=container-assets|assets|deploy-role|cfn-exec-role|lookup-role)/g, 'cdk-<QUALIFIER>-');
    }
    // Replace /cdk-bootstrap/<qualifier>/ paths
    if (obj.includes('/cdk-bootstrap/')) {
      return obj.replace(/\/cdk-bootstrap\/[a-zA-Z0-9_-]+\//g, '/cdk-bootstrap/<QUALIFIER>/');
    }
    // Replace CDK Analytics metadata (contains environment-specific compressed data)
    if (obj.startsWith('v2:deflate64:')) {
      return '<CDK_ANALYTICS>';
    }
    // Replace IP addresses in CIDR notation (except VPC CIDRs)
    if (obj.match(/^\d{1,3}\.\d{1,3}\.\d{1,3}\.\d{1,3}\/\d{1,2}$/)) {
      if (!obj.startsWith('10.') && !obj.startsWith('0.0.0.0')) {
        return '<EXTERNAL_IP>/32';
      }
    }
    // Replace IP addresses in descriptions
    if (obj.includes('deployer IP')) {
      return obj.replace(/\d{1,3}\.\d{1,3}\.\d{1,3}\.\d{1,3}/, '<EXTERNAL_IP>');
    }
    return obj;
  }
  if (Array.isArray(obj)) {
    return obj.map(normalizeTemplate);
  }
  if (typeof obj === 'object' && obj !== null) {
    const result: any = {};
    for (const [key, value] of Object.entries(obj)) {
      result[key] = normalizeTemplate(value);
    }
    return result;
  }
  return obj;
}
