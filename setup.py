import glob
import os.path
import pathlib
import sys

from setuptools import find_packages, setup
from setuptools.command.develop import develop

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


# Handle protobuf
class CustomDevelopCommand(develop):
    def run(self):
        # we need to import this here or else these packages would have to be
        # installed in the system before we could run the setup.py
        import numproto
        import grpc_tools
        from grpc_tools import protoc

        develop.run(self)

        # get the path of the numproto protofiles
        # this will give us the path to the site-packages where numproto is
        # installed
        numproto_path = pathlib.Path(numproto.__path__[0]).parent

        # get the path of grpc_tools protofiles
        grpc_path = grpc_tools.__path__[0]

        proto_files = glob.glob("./protobuf/xain_fl/grpc/*.proto")
        command = [
            "grpc_tools.protoc",
            # path to numproto .proto files
            f"--proto_path={numproto_path}",
            # path to google .proto fiels
            f"--proto_path={grpc_path}/_proto",
            "--proto_path=./protobuf",
            "--python_out=./",
            "--grpc_python_out=./",
            "--mypy_out=./",
        ] + proto_files

        print("Building proto_files {}".format(proto_files))
        if protoc.main(command) != 0:
            raise Exception("error: {} failed".format(command))


# License comments according to `pip-licenses`

install_requires = [
    "typing-extensions~=3.7",  # PSF
    "numpy~=1.15",  # BSD
    "absl-py~=0.8",  # Apache 2.0
    "grpcio~=1.23",  # Apache License 2.0
    "protobuf~=3.9",  # 3-Clause BSD License
    "numproto~=0.3",  # Apache License 2.0
    "requests==2.22.0",  # Apache 2.0  # TODO(XP-185) remove
    "tensorflow==1.14.0",  # Apache 2.0  # TODO(XP-131) remove
]

benchmarks_require = [
    "matplotlib==3.1.1",  # PSF
    "botocore==1.12.220",  # Apache License 2.0
    "boto3==1.9.220",  # Apache License 2.0
    "awscli==1.16.230",  # Apache License 2.0
    "faker==2.0.0",  # MIT License
]

gpu_require = ["tensorflow-gpu==1.14.0"]  # Apache 2.0

dev_require = [
    "grpcio-tools~=1.23",  # Apache License 2.0
    "black==19.3b0",  # MIT
    "mypy==0.720",  # MIT License
    "pylint==2.3.1",  # GPL
    "astroid<=2.2.5",  # LGPL
    "isort==4.3.20",  # MIT
    "rope==0.14.0",  # GNU GPL
    "pip-licenses==1.15.2",  # MIT License
    "mypy-protobuf==1.15",  # Apache License 2.0
    "twine==2.0.0",  # Apache License 2.0
    "wheel==0.33.6",  # MIT
]

examples_require = ["tensorflow==1.14.0"]  # Apache 2.0

tests_require = [
    "pytest==4.6.2",  # MIT license
    "pytest-cov==2.7.1",  # MIT
    "pytest-watch==4.2.0",  # MIT
]

docs_require = ["Sphinx==2.2.0", "recommonmark==0.6.0", "sphinxcontrib-mermaid==0.3.1"]

setup(
    name="xain_fl",
    version=version["__version__"],
    description="XAIN is an open source framework for federated learning.",
    long_description=readme,
    long_description_content_type="text/markdown",
    url="https://github.com/xainag/xain-fl",
    author=[
        "Daniel J. Beutel <daniel.beutel@xain.io>",
        "Taner Topal <taner.topal@xain.io>",
    ],
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
    packages=find_packages(
        where=".", exclude=["benchmarks", "benchmarks.*", "*_test.py"]
    ),
    install_requires=install_requires,
    tests_require=tests_require,
    extras_require={
        "test": tests_require,
        "gpu": gpu_require,
        "docs": docs_require,
        "benchmarks": benchmarks_require,
        "examples": examples_require,
        "dev": dev_require
        + tests_require
        + benchmarks_require
        + docs_require
        + examples_require,
    },
    cmdclass={"develop": CustomDevelopCommand},
    entry_points={
        "console_scripts": [
            "train_remote=benchmarks.train_remote:main",
            "pull_results=xain_fl.ops.__main__:download",
            "aggregate=benchmarks.aggregate:main",
        ]
    },
)
