{ mkShell
, stdenv
, rustToolchain
, ...
}:

let
  inherit (stdenv) hostPlatform;
in

mkShell {
  nativeBuildInputs = [ rustToolchain ];

  CARGO_BUILD_TARGET = hostPlatform.config;
}
