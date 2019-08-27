import glob
import sys

from setuptools import find_packages, setup

if sys.version_info < (3, 6):
    sys.exit("Please use Python version 3.6 or higher.")

install_requires = [
    "typing-extensions==3.7.4",
    "numpy==1.15.4",
    "absl-py==0.7.1",
    "matplotlib==3.1.1",
    "requests==2.22.0",
]

cpu_require = ["tensorflow==1.14.0"]

gpu_require = ["tensorflow-gpu==1.14.0"]

dev_require = [
    "black==19.3b0",
    "mypy==0.720",
    "pylint==2.3.1",
    "astroid<=2.2",
    "isort==4.3.20",
    "rope==0.14.0",
    "faker==2.0.0",
    "awscli==1.16.210",
]

tests_require = ["pytest==4.6.2", "pytest-cov==2.7.1", "pytest-watch==4.2.0"]

setup(
    name="xain",
    version="0.1.0",
    description="AutoFL demonstrates automated architecture search in federated learning environments.",
    url="https://gitlab.com/xainag/autofl",
    author=[
        "Daniel J. Beutel <daniel.beutel@xain.io>",
        "Taner Topal <taner.topal@xain.io>",
    ],
    author_email="daniel.beutel@xain.io",
    license="MIT",
    zip_safe=False,
    python_requires=">=3.6",
    classifiers=[
        "Development Status :: 3 - Alpha",
        "Intended Audience :: Any Industry",
        "Topic :: ML :: Machine Learning :: AI",
        "License :: MIT",
        "Programming Language :: Python :: 3 :: Only",
        "Programming Language :: Python :: 3.6",
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
    entry_points={
        "console_scripts": [
            "agent=xain.agent.agent:gym_autofl",
            "fedml_individual=xain.fedml.fedml:individual",
            "fedml_round_robin=xain.fedml.fedml:round_robin",
            "fedml_federated_learning=xain.fedml.fedml:federated_learning",
            "train=xain.benchmark.__main__:main_wrapper",
        ]
    },
)
