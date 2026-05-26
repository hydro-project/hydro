#!/bin/bash
# Terminate all running/pending EC2 instances in us-east-1.
# Usage: ./ec2_cleanup.sh
# Credentials must be set in environment: AWS_ACCESS_KEY_ID, AWS_SECRET_ACCESS_KEY, AWS_SESSION_TOKEN

set -e

REGION="us-east-1"

IDS=$(aws ec2 describe-instances \
  --region "$REGION" \
  --filters "Name=instance-state-name,Values=running,pending" \
  --query 'Reservations[*].Instances[*].InstanceId' \
  --output text \
  --no-paginate --no-cli-pager 2>/dev/null)

if [ -z "$IDS" ]; then
  echo "No running instances found."
  exit 0
fi

echo "Terminating: $IDS"
aws ec2 terminate-instances \
  --region "$REGION" \
  --instance-ids $IDS \
  --no-paginate --no-cli-pager \
  --query 'TerminatingInstances[*].[InstanceId,CurrentState.Name]' \
  --output table
