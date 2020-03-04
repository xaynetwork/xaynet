# pylint: disable=invalid-name
import sys

from setuptools import find_packages, setup

setup(
    name="data_handlers",
    version="0.1",
    author=["XAIN AG"],
    author_email="services@xain.io",
    license="Apache License Version 2.0",
    python_requires=">=3.6",
    packages=find_packages(),
    install_requires=["pandas==1.0.1", "scikit-learn==0.22.1", "numpy~=1.15",],
    extras_require={"dev": ["black", "mypy", "pylint", "isort", "pip-licenses",]},
    entry_points={
        "console_scripts": [
            "prepare-regression-data=data_handlers.regression_data:main"
        ]
    },
)
