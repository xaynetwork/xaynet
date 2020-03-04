with import <nixpkgs> {};

let
  python = python37;
in
pkgs.mkShell rec {
  name = "benchmark-env";
  buildInputs = with pkgs.python37Packages; [
    virtualenv
  ];
  src = null;
  shellHook = ''
    # Allow the use of wheels.
    SOURCE_DATE_EPOCH=$(date +%s)

    VENV=.${name}
    if test ! -d $VENV; then
      virtualenv $VENV >&2
    fi
    source ./$VENV/bin/activate
    export PYTHONPATH=`pwd`/$VENV/${python.sitePackages}/:$PYTHONPATH
    export LD_LIBRARY_PATH=${lib.makeLibraryPath [stdenv.cc.cc]}
  '';
}
