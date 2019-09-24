import glob
import pathlib
import sys

from setuptools import find_packages, setup
from setuptools.command.develop import develop

if sys.version_info < (3, 6):
    sys.exit("Please use Python version 3.6 or higher.")


# get readme
with open("README.md", "r") as fp:
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

        proto_files = glob.glob("./protobuf/xain/grpc/*.proto")
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
    "typing-extensions==3.7.4",  # PSF
    "numpy==1.15.4",  # BSD
    "absl-py==0.7.1",  # Apache 2.0
    "matplotlib==3.1.1",  # PSF
    "requests==2.22.0",  # Apache 2.0
    "botocore==1.12.220",  # Apache License 2.0
    "boto3==1.9.220",  # Apache License 2.0
    "awscli==1.16.230",  # Apache License 2.0
    "faker==2.0.0",  # MIT License
    "grpcio==1.23.0",  # Apache License 2.0
    "protobuf==3.9.1",  # 3-Clause BSD License
    "numproto==0.2.0",  # Apache License 2.0
    "grpcio-tools==1.23.0",  # Apache License 2.0
    "mypy-protobuf==1.15",  # Apache License 2.0
]

cpu_require = ["tensorflow==1.14.0"]  # Apache 2.0

gpu_require = ["tensorflow-gpu==1.14.0"]  # Apache 2.0

dev_require = [
    "black==19.3b0",  # MIT
    "mypy==0.720",  # MIT License
    "pylint==2.3.1",  # GPL
    "astroid<=2.2.5",  # LGPL
    "isort==4.3.20",  # MIT
    "rope==0.14.0",  # GNU GPL
    "pip-licenses==1.15.2",  # MIT License
]

tests_require = [
    "pytest==4.6.2",  # MIT license
    "pytest-cov==2.7.1",  # MIT
    "pytest-watch==4.2.0",  # MIT
]

setup(
    name="xain",
    version="0.1.0",
    description="XAIN demonstrates automated architecture search in federated learning environments.",
    long_description=readme,
    long_description_content_type="text/markdown",
    url="https://github.com/xainag/xain",
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
        "Intended Audience :: Any Industry",
        "Topic :: ML :: Machine Learning :: AI",
        "License :: OSI Approved :: Apache Software License",
        "Programming Language :: Python :: 3 :: Only",
        "Programming Language :: Python :: 3.6",
        "Programming Language :: Python :: 3.7",
        "Operating System :: MacOS :: MacOS X",
        "Operating System :: POSIX :: Linux",
    ],
    packages=find_packages(exclude=["*_test.py"]),
    install_requires=install_requires,
    tests_require=tests_require,
    extras_require={
        "test": tests_require,
        "cpu": cpu_require,
        "gpu": gpu_require,
        "dev": dev_require + tests_require,
    },
    cmdclass={"develop": CustomDevelopCommand},
    entry_points={
        "console_scripts": [
            "pull_results=xain.ops.__main__:download",
            "train_remote=xain.benchmark.__main__:main",
            "aggregate=xain.benchmark.aggregation.__main__:app_run_aggregate",
        ]
    },
)
