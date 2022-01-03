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
        "pandas==1.3.5",
        "scikit-learn==1.0.2",
        "tensorflow==2.7.0",
        "numpy>=1.19.2,<1.23.0",
        "tabulate~=0.8.7",
    ],
    entry_points={
        "console_scripts": [
            "run-participant=keras_house_prices.participant:main",
            "split-data=keras_house_prices.data_handlers.regression_data:main",
        ]
    },
)
