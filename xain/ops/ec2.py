import re


def user_data(image: str, timeout: int, flags: str):
    """Generates EC2 instance user data script which is basically
    a bash script which will be executed as soon as the machine is
    up and running

    Args:
        image (str): docker image as `repository:tag`
        timeout (int): timeout in minutes
        flags (str): Will be appended to docker run command e.g. for `--foo=bar --baz=boo`
                    it will result in `docker run IMAGE --foo=bar --baz=boo`

    Returns:
        user_data (str): user_data script for EC2 instance returned as in ascii encoding
    """
    data = [
        "#!/bin/bash",
        "set -x",
        # Set automatic shutdown after 5 hours
        f"sudo shutdown -P {timeout}",
        # Login into ECR. This only works because we assigned
        # the right IAM instance profile to the EC2 instance
        "$(aws ecr get-login --region eu-central-1 --no-include-email)",
        # Pull docker job
        f"docker pull {image}",
        # Start docker job
        "mkdir -p /opt/app/output",
        "cd /opt/app/",
        f"docker run \
            -v $(pwd)/output:/opt/app/output \
            {image} train {flags} >& $(pwd)/output/training.log",
        # Cancel previous shutdown and shutdown 1m after the job finishes
        # The machine is setup to terminate on shutdown
        "shutdown -c",
        "shutdown -P 1",
    ]

    # Replace multiple whitespaces with single whitespaces to allow
    # the use of line breaks in data to increase readability while
    # keeping the final output also readable
    data = [re.sub(r"\s+", " ", s) for s in data]

    return "\n".join(data)
