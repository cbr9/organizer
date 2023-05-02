{ pkgs ? import <nixpkgs> { } }:
pkgs.mkShell {
  nativeBuildInputs = with pkgs; [
    cargo
    mdbook
    rustc
    rustup
    gcc
    rust-analyzer
    taplo
    lldb
  ];
}
