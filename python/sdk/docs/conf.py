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

from sphinx.ext import apidoc

sys.path.insert(0, os.path.abspath(".."))


# -- Project information -----------------------------------------------------
project = "XAIN SDK"
copyright = "2020, XAIN FL Contributors"
author = "XAIN Contributors"

_version = {}
with open("../xain_sdk/__version__.py") as fp:
    exec(fp.read(), _version)
    version = _version["__version__"]


# -- General configuration ---------------------------------------------------

# Add any Sphinx extension module names here, as strings. They can be
# extensions coming with Sphinx (named 'sphinx.ext.*') or your custom
# ones.
extensions = [
    "m2r",
    "sphinx.ext.napoleon",
    "sphinx.ext.autodoc",
    "sphinx.ext.intersphinx",
    "sphinx.ext.mathjax",
    "sphinxcontrib.mermaid",
    "sphinx.ext.todo",
    "sphinx_autodoc_typehints",
    "sphinx.ext.autosectionlabel",
]

source_suffix = {".rst": "restructuredtext"}

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


# intersphinx configuration
intersphinx_mapping = {
    "python": ("https://docs.python.org/3", None),
    "numpy": ("http://docs.scipy.org/doc/numpy/", None),


}


def run_apidoc(_):
    exclude = []

    argv = [
        "--doc-project",
        "Code Reference XAIN SDK",
        "-M",
        "-f",
        "-d",
        "3",
        "--tocfile",
        "index",
        "-o",
        "./_api_reference/",
        "../xain_sdk/",
    ] + exclude

    apidoc.main(argv)


def setup(app):
    app.connect("builder-inited", run_apidoc)
