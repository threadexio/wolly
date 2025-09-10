{ mkShell
, stdenv
, buildPackages
, lib
, ...
}:

let
  inherit (stdenv.cc) targetPrefix;
  escapeTarget = prefix: lib.replaceStrings [ "-" ] [ "_" ] prefix;
  cargoEnvTarget = lib.toUpper (escapeTarget (lib.removeSuffix "-" targetPrefix));
in

mkShell {
  packages = [
    buildPackages.rustToolchain
    stdenv.cc
  ];

  CARGO_BUILD_TARGET = lib.removeSuffix "-" targetPrefix;
  "CARGO_TARGET_${cargoEnvTarget}_LINKER" = "${targetPrefix}cc";
  RUSTFLAGS = "-C target-feature=+crt-static";
}
