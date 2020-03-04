with import <nixpkgs> { };

stdenv.mkDerivation rec {
  name = "keras-house-prices";
  buildInputs = [
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

    export PYTHONPATH=`pwd`/$VENV/${python.sitePackages}/:$PYTHONPATH
    export LD_LIBRARY_PATH=${lib.makeLibraryPath [ stdenv.cc.cc ]}
  '';
}
