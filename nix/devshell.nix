{ mkShell
, stdenv
, rustToolchain
, lib
, ...
}:

let
  inherit (stdenv) buildPlatform hostPlatform;
in

mkShell
{
  nativeBuildInputs = [ rustToolchain ];

  env = lib.optionalAttrs (buildPlatform != hostPlatform) {
    CARGO_BUILD_TARGET = hostPlatform.rust.rustcTarget;
    "CARGO_TARGET_${hostPlatform.rust.cargoEnvVarTarget}_LINKER" = "${hostPlatform.config}-cc";
  };
}
