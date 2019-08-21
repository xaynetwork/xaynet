import os
from time import strftime

from faker import Faker
from faker.providers import person

fake = Faker()
fake.add_provider(person)

utc_time = strftime("%Y%m%dT%H%M")
fake_name = fake.name().replace(" ", "_").lower()

if os.environ.get("BENCHMARK_GROUP"):
    benchmark_group = f"_{os.environ['BENCHMARK_GROUP']}_"
else:
    benchmark_group = "_"

print(f"{utc_time}{benchmark_group}{fake_name}")
