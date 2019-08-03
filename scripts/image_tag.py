from time import strftime

from faker import Faker
from faker.providers import person

fake = Faker()
fake.add_provider(person)

print(strftime("%Y%m%dT%H%M%S") + "_" + fake.name().replace(" ", "_").lower())
