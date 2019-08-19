import os
from time import strftime

from faker import Faker
from faker.providers import person

fake = Faker()
fake.add_provider(person)

utc_time = strftime("%Y%m%dT%H%M%S")
fake_name = fake.name().replace(" ", "_").lower()

if os.environ.get("BENCHMARK_GROUP"):
    benchmark_group = f"{os.environ['BENCHMARK_GROUP']}_"
else:
    benchmark_group = ""

print(f"{benchmark_group}{utc_time}_{fake_name}")
