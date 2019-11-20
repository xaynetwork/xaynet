#!/bin/bash
set -e  # Exit immediately if a command exits with a non-zero status
DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" >/dev/null 2>&1 && pwd )"

cd $DIR/../

PROJECT_DIR="$( pwd | awk 'BEGIN { FS = "/" } ; { print $NF }' )"
UNAME_OUT="$(uname -s)"

# Check if `git` is installed
if ! [ -x "$(command -v git)" ]; then
  echo 'Error: git is not installed.' >&2
  exit 1
fi

# Check if `brew` is installed on macOS
if [ $UNAME_OUT == "Darwin" ]; then
    if ! [ -x "$(command -v brew)" ]; then
        echo 'Error: git is not installed.' >&2
        exit 1
    fi
fi

case $UNAME_OUT in
    Linux*)     CONFIG_FILE=".bashrc";;
    Darwin*)    CONFIG_FILE=".zshenv";;  # macOS 10.15+ defaults to ZSH
    *)          exit 1
esac

# Install for macOS
if [ $UNAME_OUT == "Darwin" ]; then
    brew install openssl readline sqlite3 xz zlib
    # sudo installer -pkg /Library/Developer/CommandLineTools/Packages/macOS_SDK_headers_for_macOS_10.14.pkg -allowUntrusted -target /
fi

# Install for Ubuntu/Debian/Mint
if [ $UNAME_OUT == "Linux" ]; then
    su -c "apt-get update"
    su -c "DEBIAN_FRONTEND=noninteractive apt-get install -yq --no-install-recommends make build-essential libssl-dev zlib1g-dev libbz2-dev libreadline-dev libsqlite3-dev wget curl llvm libncurses5-dev xz-utils tk-dev libxml2-dev libxmlsec1-dev libffi-dev liblzma-dev"
fi

# Setup pyenv
git clone https://github.com/pyenv/pyenv.git ~/.pyenv
echo 'export PYENV_ROOT="$HOME/.pyenv"' >> ~/$CONFIG_FILE
echo 'export PATH="$PYENV_ROOT/bin:$PATH"' >> ~/$CONFIG_FILE
echo -e 'if command -v pyenv 1>/dev/null 2>&1; then\n  eval "$(pyenv init -)"\nfi' >> ~/$CONFIG_FILE

PYENV_ROOT="$HOME/.pyenv"
PATH="$PYENV_ROOT/bin:$PATH"

# Setup pyenv-virtualenv
git clone https://github.com/pyenv/pyenv-virtualenv.git $(pyenv root)/plugins/pyenv-virtualenv
echo 'eval "$(pyenv virtualenv-init -)"' >> ~/$CONFIG_FILE

# Install relevant CPython versions
pyenv install 3.6.9
pyenv install 3.7.5
pyenv install 3.8.0

# Creates project specific virtualenv
pyenv virtualenv 3.6.9 ${PROJECT_DIR}-3.6.9

# Create .python-version project file
echo "${PROJECT_DIR}-3.6.9" >> .python-version
