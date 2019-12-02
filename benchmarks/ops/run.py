import os
import subprocess
from typing import Dict

import boto3
from absl import flags, logging

from xain_fl.helpers import project

from .ec2 import user_data

FLAGS = flags.FLAGS
root_dir = project.root()

# Note:
# We actually would like to use the m5.large up to m5.24xlarge
# but AWS is not easily willing to give us the increase without
# asking again and again and again for limit increases.
# Therefore we switches to using c4 which have higher default limits
cores: Dict[int, str] = {
    2: "c4.large",
    4: "c4.xlarge",
    8: "c4.2xlarge",
    16: "c4.4xlarge",
    32: "c4.8xlarge",
}


def docker(image: str, timeout: int = 300, instance_cores=2, **kwargs):
    """Run train in docker while accepting an arbitrary
    number of absl flags to be passed to the docker container

    Args:
        image (str): docker image name
        timeout (int): timeout in minutes
        instance_cores (int): number of cpu cores to be used, if num is to high os.cpu_count()
                              will be used
        **kwargs: Will be turned into "--{arg}={kwargs[arg]" format and
                  passed to docker container
    """
    instance_cores = (
        instance_cores if instance_cores <= os.cpu_count() else os.cpu_count()
    )

    command = [
        "docker",
        "run",
        "-d",
        f"--stop-timeout={timeout}",
        f"--cpus={instance_cores}",
        "-e",
        "AWS_ACCESS_KEY_ID",
        "-e",
        "AWS_SECRET_ACCESS_KEY",
        "-e",
        f"S3_RESULTS_BUCKET={FLAGS.S3_results_bucket}",
        image,
        "python",
        "-m",
        "benchmarks.benchmark.exec",
    ]

    for arg in kwargs:
        if kwargs[arg] is None:
            # Don't pass flags where arg has value None
            continue
        command.append(f"--{arg}={kwargs[arg]}")

    subprocess.run(command, cwd=root_dir)


def ec2(image: str, timeout: int = 300, instance_cores=2, **kwargs):
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
        image (str): docker image name
        timeout (int): timeout in minutes
        instance_cores (int): number of EC2 instance cpu cores
        **kwargs: Will be turned into "--{arg}={kwargs[arg]" format and passed to docker container
    """
    assert (
        instance_cores in cores
    ), f"instance_cores {instance_cores} not in {cores.keys()}"
    instance_type = cores[instance_cores]

    absl_flags = ""  # Will be passed to docker run in EC2 instance

    for arg in kwargs:
        if kwargs[arg] is None:
            # Don't pass flags where arg has value None
            continue
        absl_flags += f"--{arg}={kwargs[arg]} "

    absl_flags = absl_flags.strip()

    instance_name = (
        f"{kwargs['group_name']}_{kwargs['task_name']}"
    )  # Will be used to make the instance easier identifyable

    udata = user_data(
        image=image,
        timeout=timeout,
        S3_results_bucket=FLAGS.S3_results_bucket,
        flags=absl_flags,
    )

    client = boto3.client("ec2")
    run_response = client.run_instances(
        ImageId="ami-08806c999be9493f1",
        MinCount=1,
        MaxCount=1,
        InstanceType=instance_type,
        KeyName="xain-ec2-remote-training",
        SubnetId="subnet-1bc3c466",
        IamInstanceProfile={"Name": "XainEC2RemoteTraining"},
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
    logging.info({"InstanceId": instance_id, "Name": instance_name})
