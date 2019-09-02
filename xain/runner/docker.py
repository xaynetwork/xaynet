from time import strftime

from faker import Faker
from faker.providers import person

fake = Faker()
fake.add_provider(person)


def generate_tag(group: str = ""):
    """Return a unique string with utc time and human readable part.
    If passed it will include group as substring in the middle"""

    utc_time = strftime("%Y%m%dT%H%M")
    # pylint: disable=no-member
    fake_name = fake.name().lower().replace(" ", "_")

    if group:
        return f"{utc_time}_{group}_{fake_name}"

    return f"{utc_time}_{fake_name}"
