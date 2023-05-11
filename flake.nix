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

      in {
        # For `nix build` & `nix run`:
        defaultPackage = naersk'.buildPackage {
          src = pkgs.fetchFromGitHub {
            owner = "cbr9";
            repo = "organizer";
            rev = "9ebf448";
            sha256 = "sha256-mAuTpq/lF/l3gPX7vk7mGge35WJsGNGrGJyYI9Bj4Fw=";
          };
        };

        # For `nix develop` (optional, can be skipped):
        devShell =
          pkgs.mkShell { nativeBuildInputs = with pkgs; [ rustc cargo ]; };
      });
}
