import subprocess

import boto3
from absl import flags

from xain.helpers import project
from xain.ops.docker import get_image_name
from xain.ops.ec2 import user_data

FLAGS = flags.FLAGS
root_dir = project.root()

client = boto3.client("ec2")


def docker(tag: str = "latest", **kwargs):
    """Run train in docker while accepting an arbitrary
    number of absl flags to be passed to the docker container

    Args:
        tag (str): docker image tag to be used
        **kwargs: Will be turned into "--{arg}={kwargs[arg]" format and passed to docker container
    """

    command = ["docker", "run", "--rm", get_image_name(tag), "train"]

    for arg in kwargs:
        if arg not in FLAGS:
            raise Exception(f"{arg} is not a valid absl flags")
        command.append(f"--{arg}={kwargs[arg]}")

    subprocess.run(command, cwd=root_dir)


def ec2(image: str, timeout: int = 300, instance_type="m5.large", **kwargs):
    """Runs job on EC2 instead of a local machine

    Possible options for instance_type (CPU only) are:
    - m5.large:     2 vCPU,   8 GB RAM
    - m5.xlarge:    4 vCPU,  16 GB RAM
    - m5.2xlarge:   8 vCPU,  32 GB RAM
    - m5.4xlarge:  16 vCPU,  64 GB RAM
    - m5.8xlarge:  32 vCPU, 128 GB RAM
    - m5.12xlarge: 48 vCPU, 192 GB RAM
    - m5.16xlarge: 64 vCPU, 256 GB RAM
    - m5.24xlarge: 96 vCPU, 384 GB RAM

    Args:
        instance_type (str): EC2 instance size to be used
        **kwargs: Will be turned into "--{arg}={kwargs[arg]" format and passed to docker container
    """

    absl_flags = ""  # Will be passed to docker run in EC2 instance
    instance_name = ""  # Will be used to make the instance easier identifyable

    for arg in kwargs:
        if arg not in FLAGS:
            raise Exception(f"{arg} is not a valid absl flags")
        absl_flags += f"--{arg}={kwargs[arg]} "
        instance_name += f"{arg}={kwargs[arg]} "

    udata = user_data(image=image, timeout=timeout, flags=absl_flags)

    run_response = client.run_instances(
        ImageId="ami-08806c999be9493f1",
        MinCount=1,
        MaxCount=1,
        InstanceType=instance_type,
        KeyName="autofl_job",
        SubnetId="subnet-1bc3c466",
        IamInstanceProfile={"Name": "AutoFLJob"},
        SecurityGroupIds=["sg-01ff10b690dffbaf5", "sg-01207b671ffadadf5"],
        InstanceInitiatedShutdownBehavior="terminate",
        UserData=udata,
        TagSpecifications=[
            {
                "ResourceType": "instance",
                "Tags": [{"Key": "Name", "Value": instance_name}],
            }
        ],
        AdditionalInfo=absl_flags,  # Helpful to identify instance in EC2 UI
    )

    instance_id = run_response["Instances"][0]["InstanceId"]
    # desc_response = client.describe_instances(InstanceIds=[instance_id])

    print({"InstanceId": instance_id, "Name": instance_name})


if __name__ == "__main__":
    ec2(
        image="693828385217.dkr.ecr.eu-central-1.amazonaws.com/xain:latest",
        benchmark_name="fashion-mnist-100p-iid-balanced",
        benchmark_type="fl",
    )
