{
  rustPlatform,
  stdenvNoCC,
  pkg-config,
  openssl,
  keycloak-openapi,
}:
rustPlatform.buildRustPackage {
  pname = "keycloak-api";
  version = "0.1.0";

  src = stdenvNoCC.mkDerivation {
    name = "source";
    builder = builtins.toFile "builder.sh" ''
      source $stdenv/setup
      mkdir $out
      cp -rT --no-preserve=mode,ownership $src $out/rust/
      cp --no-preserve=mode,ownership $cargoToml $out/Cargo.toml
      cp --no-preserve=mode,ownership $cargoLock $out/Cargo.lock
    '';
    src = ../../rust;
    cargoToml = ../../Cargo.toml;
    cargoLock = ../../Cargo.lock;
  };

  buildAndTestSubdir = "rust/api-example";
  cargoTestFlags = ["--workspace"];

  cargoLock.lockFile = ../../Cargo.lock;

  nativeBuildInputs = [pkg-config];
  buildInputs = [openssl];

  env.OPENAPI_SPEC_PATH = "${keycloak-openapi}/openapi.json";
}
