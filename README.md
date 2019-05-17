# AutoFL

AutoFL demonstrates automated architecture search in federated learning environments.

## Quick Start

### Clone Repository

```bash
$ git clone https://gitlab.com/xainag/autofl
$ cd autofl
```

### Verify Installation

AutoFL requires the following tools to be installed:

- [Python 3.6.8](https://python.org/)
- [Pantsbuild 1.16.0rc0](https://www.pantsbuild.org/)

Verify Python installation:

```bash
$ python3 --version
Python 3.6.8
```

Use any `./pants` command to trigger the initial setup:

```bash
$ ./pants run src/python/autofl:bin
```

## Packages

- AutoFL: `src/python/autofl`
- CIFAR-10F: `src/python/cifar10f`
