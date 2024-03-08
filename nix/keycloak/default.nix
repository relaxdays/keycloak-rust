{
  callPackage,
  fetchFromGitHub,
  fetchpatch,
  keycloak,
}: {
  version,
  rev ? version,
  hash ? "",
  patches ? [],
  postPatch ? "",
  depsHash ? "",
}: let
  mvnArgs = [
    "-DskipTests -am -pl quarkus/deployment,quarkus/dist"
    # this probably takes half the build time for some reason even though it's much less?
    "-DskipTests -am -pl services -P jboss-release"
  ];
  src = callPackage ./source.nix {} {
    src = fetchFromGitHub {
      owner = "keycloak";
      repo = "keycloak";
      inherit rev hash;
    };
    inherit patches postPatch;
  };
  genDeps = callPackage ./deps.nix {};
  deps = genDeps {
    inherit src mvnArgs;
    hash = depsHash;
  };
  build = callPackage ./build.nix {} {
    inherit version src mvnArgs deps;
  };
in
  keycloak.overrideAttrs (old: {
    inherit version;
    src = build;

    unpackPhase = ''
      runHook preUnpack

      mkdir source
      cd source
      tar -xaf $src/keycloak.tar.gz --strip-components 1

      runHook postUnpack
    '';

    passthru =
      old.passthru
      // {
        dist = build;
        api = build.api;
      };
  })
