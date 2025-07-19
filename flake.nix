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

  outputs =
    { self
    , nixpkgs
    , flake-utils
    , rust-overlay
    , ...
    }:
    let
      mkPkgs =
        system:
        import nixpkgs {
          inherit system;
          overlays = [
            (
              _: prev:
                let
                  rust-bin = rust-overlay.lib.mkRustBin { } prev.buildPackages;
                  rustToolchain = (rust-bin.fromRustupToolchainFile ./rust-toolchain.toml).override {
                    targets = [
                      "x86_64-unknown-linux-gnu"
                      "x86_64-unknown-linux-musl"
                      "aarch64-unknown-linux-gnu"
                      "aarch64-unknown-linux-musl"
                    ];
                  };
                in
                {
                  inherit rust-bin rustToolchain;
                }
            )
            (
              _: prev:
                let
                  rustPlatform = prev.makeRustPlatform {
                    rustc = prev.rustToolchain;
                    cargo = prev.rustToolchain;
                  };
                in
                {
                  wolly = prev.callPackage ./nix/wolly.nix { inherit rustPlatform; };
                }
            )
          ];
        };
    in
    (flake-utils.lib.eachDefaultSystem (
      system:
      let
        pkgs = mkPkgs system;
      in
      {
        formatter = pkgs.nixpkgs-fmt;

        packages = {
          default = pkgs.wolly;

          cross-x86_64-unknown-linux-gnu = pkgs.pkgsCross.gnu64.wolly;
          cross-x86_64-unknown-linux-musl = pkgs.pkgsCross.musl64.wolly;
          cross-aarch64-unknown-linux-gnu = pkgs.pkgsCross.aarch64-multiplatform.wolly;
          cross-aarch64-unknown-linux-musl = pkgs.pkgsCross.aarch64-multiplatform-musl.wolly;
        };

        devShells =
          let
            mkShell =
              pkgs:
              let
                inherit (pkgs) lib;
                inherit (pkgs.stdenv.cc) targetPrefix;

                escapeTarget = prefix: builtins.replaceStrings [ "-" ] [ "_" ] prefix;

                cargoEnvTarget = lib.toUpper (escapeTarget (lib.removeSuffix "-" targetPrefix));
              in
              pkgs.mkShell {
                packages =
                  (with pkgs; [
                    buildPackages.rustToolchain
                    stdenv.cc
                  ]);

                CARGO_BUILD_TARGET = lib.removeSuffix "-" targetPrefix;
                "CARGO_TARGET_${cargoEnvTarget}_LINKER" = "${targetPrefix}cc";
                RUSTFLAGS = "-C target-feature=+crt-static";
              }
            ;
          in
          {
            default = mkShell pkgs;

            cross-x86_64-unknown-linux-gnu = mkShell pkgs.pkgsCross.gnu64;
            cross-x86_64-unknown-linux-musl = mkShell pkgs.pkgsCross.musl64;
            cross-aarch64-unknown-linux-gnu = mkShell pkgs.pkgsCross.aarch64-multiplatform;
            cross-aarch64-unknown-linux-musl = mkShell pkgs.pkgsCross.aarch64-multiplatform-musl;
          };

        apps.default = flake-utils.lib.mkApp { drv = self.packages.${system}.default; };
      }
    ))
    // {
      overlays.default = (
        final: _: {
          wolly = self.packages.${final.system}.default;
        }
      );
    };
}
