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
INSTANCE_PHYSICAL_ID=""
INSTANCE_PHYSICAL_ID=$(aws cloudformation list-stack-resources --stack-name $STACK_NAME --query "StackResourceSummaries[?LogicalResourceId=='GtfsRtInstance'].PhysicalResourceId" --output text)
# Look up the hostname of the instance by physical ID
INSTANCE_HOSTNAME=""
INSTANCE_HOSTNAME=$(aws ec2 describe-instances --instance-ids $INSTANCE_PHYSICAL_ID --query "Reservations[*].Instances[*].PublicDnsName" --output text)

# Run the playbook! :-)
export ANSIBLE_HOST_KEY_CHECKING=False # If it's a new host, ssh known_hosts not having the key fingerprint will cause an error. Silence it
ansible-galaxy collection install datadog.dd
ansible-playbook -v -i $INSTANCE_HOSTNAME, -u ubuntu --private-key ~/.ssh/gtfs-rt-rater.pem playbook.yml