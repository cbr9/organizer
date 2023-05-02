{ pkgs ? import <nixpkgs> { } }:
pkgs.mkShell {
  nativeBuildInputs = with pkgs; [
    cargo
    tokei
    hyperfine
    mdbook
    rustc
    rustup
    gcc
    rust-analyzer
    taplo
    lldb
  ];
}
