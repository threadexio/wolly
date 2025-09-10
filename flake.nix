{
  description = "wolly";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixpkgs-unstable";

    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs =
    { self
    , nixpkgs
    , rust-overlay
    , ...
    }:
    let
      systems = [
        "aarch64-linux"
        "aarch64-darwin"
        "x86_64-linux"
        "x86_64-darwin"
      ];

      inherit (nixpkgs) lib;

      perSystem =
        f:
        lib.genAttrs systems (
          system:
          let
            pkgs = import nixpkgs {
              inherit system;

              overlays = [
                (final: prev: {
                  rust-bin = rust-overlay.lib.mkRustBin { } prev.buildPackages;
                  rustToolchain = (final.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml).override {
                    targets = [
                      "x86_64-unknown-linux-gnu"
                      "x86_64-unknown-linux-musl"
                      "aarch64-unknown-linux-gnu"
                      "aarch64-unknown-linux-musl"
                    ];
                  };
                })
              ];
            };
          in
          f pkgs
        );
    in
    {
      formatter = perSystem (pkgs: pkgs.nixpkgs-fmt);

      devShells = perSystem (
        pkgs:
        let
          mkShell = pkgs: import ./nix/devshell.nix pkgs;
        in
        {
          default = mkShell pkgs;

          cross-x86_64-unknown-linux-gnu = mkShell pkgs.pkgsCross.gnu64;
          cross-x86_64-unknown-linux-musl = mkShell pkgs.pkgsCross.musl64;
          cross-aarch64-unknown-linux-gnu = mkShell pkgs.pkgsCross.aarch64-multiplatform;
          cross-aarch64-unknown-linux-musl = mkShell pkgs.pkgsCross.aarch64-multiplatform-musl;
        }
      );

      packages = perSystem (
        pkgs:
        let
          mkPkg =
            pkgs:
            pkgs.callPackage ./nix/package.nix {
              rustPlatform = pkgs.makeRustPlatform {
                rustc = pkgs.rustToolchain;
                cargo = pkgs.rustToolchain;
              };
            };
        in
        {
          default = mkPkg pkgs;

          cross-x86_64-unknown-linux-gnu = mkPkg pkgs.pkgsCross.gnu64;
          cross-x86_64-unknown-linux-musl = mkPkg pkgs.pkgsCross.musl64;
          cross-aarch64-unknown-linux-gnu = mkPkg pkgs.pkgsCross.aarch64-multiplatform;
          cross-aarch64-unknown-linux-musl = mkPkg pkgs.pkgsCross.aarch64-multiplatform-musl;
        }
      );

      apps = perSystem (
        pkgs:
        let
          mkApp = drv: {
            type = "app";
            program = lib.getExe drv;
          };
        in
        lib.mapAttrs (_: mkApp) self.packages.${pkgs.system}
      );

      overlays.default = final: _: {
        wolly = self.packages.${final.system}.default;
      };

      nixosModules.default = import ./nix/module.nix { inherit self; };
    };
}
