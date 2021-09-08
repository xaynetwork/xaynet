# pylint: disable=invalid-name
from setuptools import find_packages, setup

setup(
    name="keras_house_prices",
    version="0.1",
    author=["Xayn Engineering"],
    author_email="engineering@xaynet.dev",
    license="Apache License Version 2.0",
    python_requires=">=3.7.1, <=3.8",
    packages=find_packages(),
    install_requires=[
        "pandas==1.3.1",
        "scikit-learn==0.24.2",
        "tensorflow==2.6.0",
        "numpy>=1.19.2,<1.22.0",
        "tabulate~=0.8.7",
    ],
    entry_points={
        "console_scripts": [
            "run-participant=keras_house_prices.participant:main",
            "split-data=keras_house_prices.data_handlers.regression_data:main",
        ]
    },
)
