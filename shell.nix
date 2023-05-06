{ pkgs ? import <nixpkgs> { } }:
pkgs.mkShell {
  nativeBuildInputs = with pkgs; [
    cargo
    tokei
    hyperfine
    sqlite
    mdbook
    rustc
    rustup
    gcc
    rust-analyzer
    taplo
    lldb
  ];
}
