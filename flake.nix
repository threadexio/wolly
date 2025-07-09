{
  description = "wolly";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixpkgs-unstable";
    flake-utils.url = "github:numtide/flake-utils";

    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { self, nixpkgs, flake-utils, rust-overlay, ... }:
    let
      mkPkgs = system: import nixpkgs {
        inherit system;
        overlays = [
          (import rust-overlay)
          (final: _: {
            rustToolchain = final.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml;
          })
          (final: _: {
            wolly = final.callPackage ./nix/wolly.nix {};
          })
        ];
      };
    in
    flake-utils.lib.eachDefaultSystem (system:
      let pkgs = mkPkgs system; in
      {
        packages.default = pkgs.wolly;
        apps.default = flake-utils.lib.mkApp { drv = self.packages.${system}.default; };

        devShells = let
          mkShell = pkgs: extraArgs: pkgs.mkShell (extraArgs // {
            packages = with pkgs; [
              buildPackages.rustToolchain
              stdenv.cc
            ];
          });
        in {
          default = mkShell pkgs;

          cross-aarch64-gnu = mkShell pkgs.pkgsCross.aarch64-multiplatform {
            CARGO_BUILD_TARGET = "aarch64-unknown-linux-gnu";
            CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER = "aarch64-unknown-linux-gnu-gcc";
          };

          cross-aarch64-musl = mkShell pkgs.pkgsCross.aarch64-multiplatform-musl {
            CARGO_BUILD_TARGET = "aarch64-unknown-linux-musl";
            CARGO_TARGET_AARCH64_UNKNOWN_LINUX_MUSL_LINKER = "aarch64-unknown-linux-musl-gcc";
          };
        };
      }
    );
}
