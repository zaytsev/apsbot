{
  description = "Basic Rust dev environment";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    flake-compat = {
      url = github:edolstra/flake-compat;
      flake = false;
    };
    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { self, nixpkgs, flake-utils, flake-compat, fenix }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        overlays = [ fenix.overlay ];
        pkgs = import nixpkgs { inherit system overlays; };
      in
      {
        devShell = pkgs.mkShell {
          buildInputs = with pkgs; [
            (
              with fenix.packages.${system};
              combine [
                stable.rustc
                stable.cargo
                stable.rust-src
                stable.rustfmt
                rust-analyzer
              ]
            )
            cargo-edit
            cargo-update
            cargo-geiger
            cargo-outdated
            cargo-audit
            
            cocogitto
          ];
        };
      }
    );
}
