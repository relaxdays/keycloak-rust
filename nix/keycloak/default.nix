{
  callPackage,
  applyPatches,
  fetchFromGitHub,
  fetchpatch,
  keycloak,
  pnpm_9,
}: {
  version,
  rev ? version,
  srcHash ? "",
  patches ? [],
  postPatch ? "",
  pnpmHash ? "",
  mvnHash ? "",
}: let
  src = applyPatches {
    src = fetchFromGitHub {
      owner = "keycloak";
      repo = "keycloak";
      inherit rev;
      hash = srcHash;
    };
    inherit patches postPatch;
  };
  build = callPackage ./build.nix {pnpm = pnpm_9;} {
    inherit version src mvnHash pnpmHash;
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
