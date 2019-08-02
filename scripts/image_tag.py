from time import strftime

from faker import Faker
from faker.providers import person

fake = Faker()
fake.add_provider(person)

print(fake.name().replace(" ", "_").lower() + "_" + strftime("%Y%m%dT%H%M%S"))
