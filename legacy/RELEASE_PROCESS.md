# Release Process

### Versioning Schema

This project follows the python form of Semantic Versioning as detailed in [PEP
440](https://www.python.org/dev/peps/pep-0440/).

### Versioning in Git History

For git tags we use the version as describe in [Versioning
Schema](#versioning-schema) preceded by a `v`.

A release on git is just a tagged commit on the `master` branch.

### How to do a Github Release

Here we detail the process of creating a new Github release.

1. Create and merge a pull request that:
   - increases the version number in
     [`xain_fl/__version__.py`](https://github.com/xainag/xain-fl/blob/master/xain_fl/__version__.py)
     according the versioning schema.
   - updates the
     [`CHANGELOG.md`](https://github.com/xainag/xain-fl/blob/master/CHANGELOG.md)
     with all notable changes for the release.
   - possibly update the `Development Status` classifiers in the
     [`setup.py`](https://github.com/xainag/xain-fl/blob/master/setup.py). You
     can check supported classifiers in the [pypi
     website](https://pypi.org/classifiers/).
2. Got to the [Github Releases tab](https://github.com/xainag/xain-fl/releases)
   and create a new release:
   - for the tag version use the version defined in 1. preceded by a `v`, e.g.
     v0.3.2, and target master.
   - for the release title use the same as the tag version.
   - for the release description, copy the section from the
     [`CHANGELOG.md`](https://github.com/xainag/xain-fl/blob/master/CHANGELOG.md)
     related to this version.
   - possibly check the `This is a pre-release` check box.
   - Publish the release.

### How to publish a new release to PyPi

Here we detail the process of building and pushing a python package to PyPi.
You can check more information in the [Python Packaging User
Guide](https://packaging.python.org/tutorials/packaging-projects/).

1. Checkout the current git tag e.g.
   ```bash
   $ git checkout v0.3.2
   ```
2. Generate the distribution archives:
   ```bash
   $ python setup.py sdist bdist_wheel
   ```
3. Upload the distribution archives using the correct PyPi credentials:
   ```bash
   $ python -m twine upload dist/*
   ```
