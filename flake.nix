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

      crossTargets = {
        "aarch64-unknown-linux-gnu" = p: p.pkgsCross.aarch64-multiplatform;
        "aarch64-unknown-linux-musl" = p: p.pkgsCross.aarch64-multiplatform-musl;
        "x86_64-unknown-linux-gnu" = p: p.pkgsCross.gnu64;
        "x86_64-unknown-linux-musl" = p: p.pkgsCross.musl64;
      };

      inherit (nixpkgs) lib;

      extend = f: overlays: lib.fix (lib.foldl (acc: x: lib.extends x acc) f overlays);

      pkgsFor = system: import nixpkgs {
        inherit system;

        overlays = [
          (pkgs: _:
            let
              args = final: {
                inherit self;

                rust-bin = rust-overlay.lib.mkRustBin { } pkgs.buildPackages;
                rustToolchain = final.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml;
                rustPlatform = pkgs.makeRustPlatform {
                  rustc = final.rustToolchain;
                  cargo = final.rustToolchain;
                };
              };

              callPackage = pkg: extraArgs: pkgs.callPackage pkg (extend args [ extraArgs ]);
            in
            {
              devshell = callPackage ./nix/devshell.nix (_: prev: {
                rustToolchain = prev.rustToolchain.override {
                  extensions = [ "rustfmt" "clippy" "rust-src" "rust-analyzer" ];
                };
              });

              wolly = callPackage ./nix/package.nix (_: _: { });
            })
        ];
      };

      crossPkgsFor = pkgs: target: crossTargets.${target} pkgs;

      perSystem = f: lib.genAttrs systems (system: f (pkgsFor system));

      perCrossTarget = pkgs: f: lib.genAttrs (lib.attrNames crossTargets) (target: f (crossPkgsFor pkgs target));

      perTarget = f:
        perSystem (pkgs: (f pkgs) // (perCrossTarget pkgs f));

      mkApp = drv: {
        type = "app";
        program = lib.getExe drv;
      };
    in
    {
      formatter = perSystem (pkgs: pkgs.nixpkgs-fmt);

      devShells = perTarget (pkgs: {
        default = pkgs.devshell;
      });

      packages = perTarget (pkgs: rec {
        default = wolly;
        inherit (pkgs) wolly;
      });

      apps = perTarget (pkgs: {
        default = mkApp pkgs.wolly;
      });

      overlays.default = final: _:
        let
          inherit (final.stdenv) buildPlatform hostPlatform;
        in
        {
          wolly = self.packages.${buildPlatform.system}.${hostPlatform.config}.default;
        };

      nixosModules.default = import ./nix/module.nix { inherit self; };
    };
}
