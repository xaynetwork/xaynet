# Minimal makefile for Sphinx documentation
#

# You can set these variables from the command line, and also
# from the environment for the first two.
SPHINXOPTS    ?=
SPHINXBUILD   ?= sphinx-build
SOURCEDIR     = .
BUILDDIR      = _build

# Put it first so that "make" without argument is like "make help".
help:
	@$(SPHINXBUILD) -M help "$(SOURCEDIR)" "$(BUILDDIR)" $(SPHINXOPTS) $(O)

.PHONY: help Makefile

docs:
	sphinx-apidoc --doc-project "Code Reference XAIN" -f -d 3 --tocfile index -o ./_code_reference_xain/ ../xain/ ../xain/**_test.py
	sphinx-apidoc --doc-project "Code Reference Benchmarks" -f -d 3 --tocfile index -o ./_code_reference_benchmarks/ ../benchmarks/ ../benchmarks/**_test.py
	make html

# Catch-all target: route all unknown targets to Sphinx using the new
# "make mode" option.  $(O) is meant as a shortcut for $(SPHINXOPTS).
%: Makefile
	@$(SPHINXBUILD) -M $@ "$(SOURCEDIR)" "$(BUILDDIR)" $(SPHINXOPTS) $(O)
