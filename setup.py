import os.path
import sys

from setuptools import find_packages, setup

if sys.version_info < (3, 6):
    sys.exit("Please use Python version 3.6 or higher.")

project_dir = os.path.dirname(os.path.abspath(__file__))
version_file_path = os.path.join(project_dir, "xain_fl/__version__.py")
readme_file_path = os.path.join(project_dir, "README.md")

# get version
version = {}
with open(version_file_path) as fp:
    exec(fp.read(), version)


# get readme
with open(readme_file_path, "r") as fp:
    readme = fp.read()


# License comments according to `pip-licenses`

install_requires = [
    "numpy==1.15",  # BSD
    "grpcio==1.23",  # Apache License 2.0
    "structlog==19.2.0",  # Apache License 2.0
    "xain-proto==0.5.0",  # Apache License 2.0
    "boto3==1.10.48",  # Apache License 2.0
    "toml==0.10.0",  # MIT
    "schema~=0.7",  # MIT
    "idna==2.8",  # BSD
    "influxdb==5.2.3",  # MIT
    "jsonschema==3.2.0",  # MIT
]

dev_require = [
    "black==19.10b0",  # MIT
    "mypy==0.760",  # MIT License
    "pylint==2.3.1",  # GPL
    "astroid==2.2.5",  # LGPL
    "isort==4.3.21",  # MIT
    "pip-licenses==1.15.2",  # MIT License
    "twine==2.0.0",  # Apache License 2.0
    "wheel==0.33.6",  # MIT
    "typing-extensions==3.7.4.1",  # PSF
]

tests_require = [
    "pytest==5.3.2",  # MIT license
    "pytest-cov==2.8.1",  # MIT
    "pytest-watch==4.2.0",  # MIT
    "xain-sdk==0.5.0",  # Apache License 2.0
]

docs_require = [
    "Sphinx==2.2.1",
    "m2r==0.2.1",
    "sphinxcontrib-mermaid==0.3.1",
    "sphinx-autodoc-typehints==1.10.3",  # MIT
    # docutils is a transitive dependency for docs_requires and
    # install_requires, which are handled sequentially by pip. If the
    # docs_requires dependencies are handled first, `docutils~=0.16`
    # gets installed, whereas install_requires depends on
    # `docutils<0.16`. To prevent this from happening, we just pin the
    # docutils version here.
    "docutils < 0.16",
]

setup(
    name="xain_fl",
    version=version["__version__"],
    description="XAIN is an open source framework for federated learning.",
    long_description=readme,
    long_description_content_type="text/markdown",
    url="https://github.com/xainag/xain-fl",
    author=["XAIN AG"],
    author_email="services@xain.io",
    license="Apache License Version 2.0",
    zip_safe=False,
    python_requires=">=3.6",
    classifiers=[
        "Development Status :: 3 - Alpha",
        "Intended Audience :: Developers",
        "Intended Audience :: Information Technology",
        "Intended Audience :: Science/Research",
        "Topic :: Scientific/Engineering",
        "Topic :: Scientific/Engineering :: Artificial Intelligence",
        "Topic :: Software Development",
        "Topic :: Software Development :: Libraries",
        "Topic :: Software Development :: Libraries :: Application Frameworks",
        "Topic :: Software Development :: Libraries :: Python Modules",
        "License :: OSI Approved :: Apache Software License",
        "Programming Language :: Python :: 3 :: Only",
        "Programming Language :: Python :: 3.6",
        "Programming Language :: Python :: 3.7",
        "Programming Language :: Python :: 3.8",
        "Operating System :: MacOS :: MacOS X",
        "Operating System :: POSIX :: Linux",
    ],
    packages=find_packages(exclude=["tests"]),
    install_requires=install_requires,
    tests_require=tests_require,
    extras_require={
        "test": tests_require,
        "docs": docs_require,
        "dev": dev_require + tests_require + docs_require,
    },
    entry_points={"console_scripts": ["coordinator=xain_fl.__main__:main"]},
)
