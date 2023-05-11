{ pkgs ? import <nixpkgs> { } }:
pkgs.mkShell {
  nativeBuildInputs = with pkgs; [
    cargo
    tokei
    hyperfine
    sqlite
    rust-analyzer
    mdbook
    rustc
    rustup
    gcc
    taplo
    clippy
    lldb
    crate2nix
  ];
}
