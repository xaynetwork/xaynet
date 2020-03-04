# pylint: disable=invalid-name
import sys

from setuptools import find_packages, setup

setup(
    name="keras_house_prices",
    version="0.1",
    description="XAIN is an open source framework for federated learning.",
    long_description_content_type="text/markdown",
    url="https://github.com/xainag/xain-sdk",
    author=["XAIN AG"],
    author_email="services@xain.io",
    license="Apache License Version 2.0",
    python_requires=">=3.6",
    packages=find_packages(),
    install_requires=[
        "joblib==0.14.1",
        "pandas==1.0.1",
        "py7zr==0.4.4",
        "scikit-learn==0.22.1",
        "scipy==1.4.1",
        "tensorflow==1.15.2",
    ],
    extras_require={"dev": ["black", "mypy", "pylint", "isort", "pip-licenses",]},
    entry_points={
        "console_scripts": [
            "keras-house-prices-participant=keras_house_prices.participant:main"
        ]
    },
)
