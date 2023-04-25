{ pkgs ? import <nixpkgs> {} }:
  pkgs.mkShell {
    nativeBuildInputs = with pkgs; [
      cargo
      rustc
      rustup
      gcc
      rust-analyzer
      taplo
      lldb
    ];
}
