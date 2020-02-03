with import <nixpkgs> {};

stdenv.mkDerivation {
  name = "coordinator";
  buildInputs = [
    pkgs.latest.rustChannels.nightly.rust
    pkgs.cargo-edit
    pkgs.rustfmt
  ];
  RUST_BACKTRACE = 1;
}
