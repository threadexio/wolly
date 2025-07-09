{
  description = "wolly";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixpkgs-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, flake-utils, ... }:
    let
      mkPkgs = system: import nixpkgs {
        inherit system;
        overlays = [
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
        apps.default= flake-utils.lib.mkApp { drv = self.packages.${system}.default; };
      }
    );
}
