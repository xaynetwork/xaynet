import subprocess
from time import strftime

from absl import app
from faker import Faker
from faker.providers import person

from xain.helpers import project

fake = Faker()
fake.add_provider(person)

root_dir = project.root()


def get_image_name(tag: str):
    """Returns docker image name by joining repo with tag

    Args:
        tag (str): docker image tag to be used
    """
    ECR_REPO = "693828385217.dkr.ecr.eu-central-1.amazonaws.com/xain"
    return f"{ECR_REPO}:{tag}"


def generate_unique_tag():
    """Return a unique string with utc time and human readable part.
    If passed it will include group as substring in the middle"""

    utc_time = strftime("%Y%m%dT%H%M")
    # pylint: disable=no-member
    fake_name = fake.name().lower().replace(" ", "_")

    return f"{utc_time}_{fake_name}"


def build(should_push: bool = False):
    """Build xain docker container and tag it uniquely
    If image already exists a new tag will be added anyway

    Args:
        tag (str): docker image tag to be used
    """
    tag = generate_unique_tag()
    image_name_unique = get_image_name(tag)
    image_name_latest = get_image_name("latest")

    command = ["docker", "build", ".", "-t", image_name_latest, "-t", image_name_unique]
    subprocess.run(command, cwd=root_dir).check_returncode()

    if should_push:
        push()

    return image_name_unique


def push(tag: str = "latest"):
    """Push xain docker container

    Args:
        tag (str): docker image tag to be used
    """
    # User to get the docker login command with the AWS SDK
    get_login_command = ["aws", "ecr", "get-login", "--no-include-email"]
    push_command = ["docker", "push", get_image_name(tag)]

    # Get docker login command and decode into utf-8. Afterwards split
    # to pass it further to the next subprocess call
    docker_login_command = (
        subprocess.check_output(get_login_command, cwd=root_dir)
        .decode("utf-8")
        .strip()
        .split(" ")
    )

    subprocess.run(docker_login_command, cwd=root_dir).check_returncode()
    subprocess.run(push_command, cwd=root_dir).check_returncode()


# For convenience allow calling module to build and push latest image
if __name__ == "__main__":
    app.run(main=lambda _: build(should_push=True))
