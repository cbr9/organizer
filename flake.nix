{
  description = "Build a cargo project";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    naersk.url = "github:nix-community/naersk";
    flake-utils.url = "github:numtide/flake-utils";
    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs =
    {
      nixpkgs,
      naersk,
      flake-utils,
      fenix,
      ...
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        pkgs = (import nixpkgs) {
          inherit system;
          overlays = [
            fenix.overlays.default
          ];
        };
        toolchain = fenix.packages.${system}.complete.withComponents [
          "cargo"
          "clippy"
          "miri"
          "rustc"
          "rustfmt"
        ];

        naersk' = pkgs.callPackage naersk {
          cargo = toolchain;
          rustc = toolchain;
        };

      in
      {
        packages.default = naersk'.buildPackage {
          src = ./.;
          DATABASE_URL = "sqlite:/home/cabero/Code/organize/organize.db";
        };

        devShells.default = pkgs.mkShell {
          packages = with pkgs; [
            toolchain
            rust-analyzer-nightly
            nixd
            sqlx-cli
            pkg-config
            openssl
            nixfmt-rfc-style
            nixd
            git
            lazygit
            just
          ];
        };
      }
    );
}
