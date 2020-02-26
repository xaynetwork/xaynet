with import <nixpkgs> { };

stdenv.mkDerivation rec {
  name = "xain-fl";
  buildInputs = [
    pkgs.latest.rustChannels.nightly.rust
    pkgs.cargo-edit
    pkgs.rustfmt
    python37Packages.numpy
    python37Packages.ipython
    python37Packages.virtualenv
  ];
  RUST_BACKTRACE = 1;
  src = null;
  shellHook = ''
    # Allow the use of wheels.
    SOURCE_DATE_EPOCH=$(date +%s)

    VENV=.${name}
    if test ! -d $VENV; then
      virtualenv $VENV
    fi
    source ./$VENV/bin/activate
    # pip install -U -e './python/[dev]'

    export PYTHONPATH=`pwd`/$VENV/${python.sitePackages}/:$PYTHONPATH
    export LD_LIBRARY_PATH=${lib.makeLibraryPath [ stdenv.cc.cc ]}
  '';
}
