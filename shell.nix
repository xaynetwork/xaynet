with import <nixpkgs> { };

stdenv.mkDerivation rec {
  name = "xain-fl";
  buildInputs = [
    pkgs.latest.rustChannels.nightly.rust
    pkgs.cargo-edit
    pkgs.rustfmt
    pkgs.openssl
    pkgs.pkg-config
    pkgs.gperftools
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
    export LIBTCMALLOC_PATH=${lib.makeLibraryPath [ pkgs.gperftools ]}/libtcmalloc.so
    
    # This is to profile memory usage in the aggregator. See:
    # https://stackoverflow.com/questions/38254937/how-do-i-debug-a-memory-issue-in-rust
    #
    # LD_PRELOAD="$LIBTCMALLOC_PATH" HEAPPROFILE=./profile ./target/debug/aggregator -c configs/dev-aggregator.toml 
    # pprof --gv ./path/to/exe /tmp/profile/profile.0100.heap

  '';
}
