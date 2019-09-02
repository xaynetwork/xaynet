import glob
import sys

from setuptools import find_packages, setup

if sys.version_info < (3, 6):
    sys.exit("Please use Python version 3.6 or higher.")

# License comments according to `pip-licenses`

install_requires = [
    "typing-extensions==3.7.4",  # PSF
    "numpy==1.15.4",  # BSD
    "absl-py==0.7.1",  # Apache 2.0
    "matplotlib==3.1.1",  # PSF
    "requests==2.22.0",  # Apache 2.0
    "boto3==1.9.218",  # Apache 2.0
]

cpu_require = ["tensorflow==1.14.0"]  # Apache 2.0

gpu_require = ["tensorflow-gpu==1.14.0"]  # Apache 2.0

dev_require = [
    "black==19.3b0",  # MIT
    "mypy==0.720",  # MIT License
    "pylint==2.3.1",  # GPL
    "astroid<=2.2",  # LGPL
    "isort==4.3.20",  # MIT
    "rope==0.14.0",  # GNU GPL
    "faker==2.0.0",  # MIT License
    "awscli==1.16.210",  # Apache License 2.0
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
    cmdclass={},
    entry_points={"console_scripts": ["train=xain.benchmark.__main__:main_wrapper"]},
)
