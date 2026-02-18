#!/bin/bash
set -e

export AWS_REGION=us-east-1
export AWS_DEFAULT_REGION=us-east-1
export AWS_PAGER=""

STACK_NAME="gtfs-rt-rater"

# Ensure required secrets are set
if [[ -z "$DD_API_KEY" ]]; then
    echo "Must provide DD_API_KEY in environment to deploy" 1>&2
    exit 1
fi

# Identify the version and commit of the current deploy
GIT_SHA=""
GIT_SHA=$(git rev-parse HEAD)
export GIT_SHA
echo "Deploying version $GIT_SHA"

echo "Deploying GTFS RT Rater..."
echo "View stack log here: https://$AWS_REGION.console.aws.amazon.com/cloudformation/home?region=$AWS_REGION"

aws cloudformation deploy --stack-name $STACK_NAME \
    --tags service=gtfs-rt-rater env=prod \
    --template-file cloudformation.json \
    --capabilities CAPABILITY_NAMED_IAM \
    --no-fail-on-empty-changeset

# Look up the physical ID of the EC2 instance currently associated with the stack
INSTANCE_ID=""
INSTANCE_ID=$(aws cloudformation list-stack-resources --stack-name $STACK_NAME --query "StackResourceSummaries[?LogicalResourceId=='GtfsRtInstance'].PhysicalResourceId" --output text)

# Run the playbook using AWS Systems Manager Session Manager
# Install collections
ansible-galaxy collection install -r requirements.yml

# Run playbook over SSM
ansible-playbook -v -i $INSTANCE_ID, playbook.yml \
  -e ansible_connection=amazon.aws.aws_ssm