# Configuration file for the Sphinx documentation builder.
#
# This file only contains a selection of the most common options. For a full
# list see the documentation:
# https://www.sphinx-doc.org/en/master/usage/configuration.html

# -- Path setup --------------------------------------------------------------

# If extensions (or modules to document with autodoc) are in another directory,
# add these directories to sys.path here. If the directory is relative to the
# documentation root, use os.path.abspath to make it absolute, like shown here.

import os
import sys

import toml
from sphinx.ext import apidoc

sys.path.insert(0, os.path.abspath(".."))

# get version
version = toml.load("../rust/Cargo.toml")["package"]["version"]

# -- Project information -----------------------------------------------------

project = "XAIN FL"
copyright = "2020, XAIN FL Contributors"
author = "XAIN FL Contributors"

# The major project version, used as the replacement for |version|. For example,
# for the Python documentation, this may be something like 2.6.
# The short X.Y version
version = version
# The full version, including alpha/beta/rc tags
# If you don’t need the separation provided between version and release, just set them both to the same value.
release = version


# -- General configuration ---------------------------------------------------

# Add any Sphinx extension module names here, as strings. They can be
# extensions coming with Sphinx (named 'sphinx.ext.*') or your custom
# ones.
extensions = [
    "m2r",
    "sphinx.ext.mathjax",
    "sphinxcontrib.mermaid",
    "sphinx.ext.todo",
    "sphinxcontrib.openapi",
]

source_suffix = {".rst": "restructuredtext", ".md": "markdown"}

# Add any paths that contain templates here, relative to this directory.
# templates_path = ['_templates']

# List of patterns, relative to source directory, that match files and
# directories to ignore when looking for source files.
# This pattern also affects html_static_path and html_extra_path.
exclude_patterns = ["_build", "Thumbs.db", ".DS_Store"]


# -- Options for HTML output -------------------------------------------------

# The theme to use for HTML and HTML Help pages.  See the documentation for
# a list of builtin themes.

html_theme = "alabaster"

# https://alabaster.readthedocs.io/en/latest/customization.html
html_theme_options = {
    "logo": "brainy.svg",
    "github_banner": True,
    "github_user": "xainag",
    "github_repo": "xain-fl",
    "github_button": False,
    "sidebar_collapse": False,
}

# Add any paths that contain custom static files (such as style sheets) here,
# relative to this directory. They are copied after the builtin static files,
# so a file named "default.css" will overwrite the builtin "default.css".
html_static_path = ["_static"]
