{
  inputs = {
    flake-utils.url = "github:numtide/flake-utils";
    naersk.url = "github:nix-community/naersk";
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
  };

  outputs = { self, flake-utils, naersk, nixpkgs }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = (import nixpkgs) { inherit system; };
        naersk' = pkgs.callPackage naersk { };
        lib = nixpkgs.lib;
      in {
        # For `nix build` & `nix run`:
        defaultPackage = naersk'.buildPackage {
          copyBins = true;
          src = pkgs.fetchFromGitHub {
            owner = "cbr9";
            repo = "organizer";
            rev = "0b57e38";
            sha256 = "sha256-ULrgrTEUxSMcy8bzWk+Cf+P1St4WWy3KYPKzk6GXfWo=";
          };
        };

        # For `nix develop` (optional, can be skipped):
        devShell = pkgs.mkShell {
          nativeBuildInputs = with pkgs; [
            rustc
            cargo
            clippy
            rust-analyzer
            rustfmt
          ];
        };
      });
}
