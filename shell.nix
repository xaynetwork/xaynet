with import <nixpkgs> {};

stdenv.mkDerivation {
  name = "coordinator";
  buildInputs = [
    pkgs.latest.rustChannels.nightly.rust
    pkgs.cargo-edit
    pkgs.rustfmt
    python37Packages.ipython
    python37Packages.numpy
  ];
  RUST_BACKTRACE = 1;
}
