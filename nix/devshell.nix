{ mkShell
, stdenv
, buildPackages
, ...
}:

let
  inherit (stdenv) hostPlatform;
in

mkShell {
  packages = [
    buildPackages.rustToolchain
    stdenv.cc
  ];

  CARGO_BUILD_TARGET = hostPlatform.config;
}
