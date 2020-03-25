# pylint: disable=invalid-name
import os.path
import sys

from setuptools import find_packages, setup

project_dir = os.path.dirname(os.path.abspath(__file__))
version_file_path = os.path.join(project_dir, "xain_sdk/__version__.py")
readme_file_path = os.path.join(project_dir, "README.md")

version = {}
with open(version_file_path) as fp:
    exec(fp.read(), version)

with open(readme_file_path, "r") as fp:
    readme = fp.read()

install_requires = ["requests", "numpy"]

dev_require = [
    "black",
    "mypy",
    "pylint",
    "isort",
    "pip-licenses",
]

tests_require = [
    "pytest",
]

docs_require = []

setup(
    name="xain-sdk",
    version=version["__version__"],
    description="XAIN is an open source framework for federated learning.",
    long_description=readme,
    long_description_content_type="text/markdown",
    url="https://github.com/xainag/xain-sdk",
    author=["XAIN AG"],
    author_email="engineering@xain.io",
    license="Apache License Version 2.0",
    python_requires=">=3.6",
    install_requires=install_requires,
    packages=find_packages(),
    tests_require=tests_require,
    extras_require={
        "test": tests_require,
        "docs": docs_require,
        "dev": dev_require + tests_require + docs_require,
    },
)
