#!/bin/bash
set -e  # Exit immediately if a command exits with a non-zero status
DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" >/dev/null 2>&1 && pwd )"
cd $DIR/../

AWS_PROFILE=xain-xain aws ec2 describe-instances  --filters Name=instance-state-code,Values=16 | jq '.Reservations[].Instances[] | "\(.Tags[].Value), \(.PublicIpAddress)"'
