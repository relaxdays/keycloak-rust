{
  lib,
  stdenvNoCC,
  maven,
  nodejs,
  nodePackages,
  cacert,
  jq,
  moreutils,
}: {
  mvnArgs,
  src,
  hash ? "",
}:
stdenvNoCC.mkDerivation {
  name = "keycloak-deps";
  inherit src;
  outputHashMode = "recursive";

  outputHashAlgo =
    if hash == ""
    then "sha256"
    else "";
  outputHash = hash;

  nativeBuildInputs = [
    maven
    nodejs
    nodePackages.pnpm
    jq
    moreutils
  ];

  # this would patch paths in the pnpm store, which we don't want to do
  dontFixup = true;

  # required so pnpm can download packages
  NODE_EXTRA_CA_CERTS = "${cacert}/etc/ssl/certs/ca-bundle.crt";

  preBuild = ''
    mkdir -p $out/{.m2,pnpm}

    export HOME="$TMPDIR"
    pnpm config set store-dir $out/pnpm --global

    echo "Preinstalling and patching npm packages"
    pnpm i --frozen-lockfile --ignore-scripts
    patchShebangs --host node_modules/.bin
  '';

  buildPhase = ''
    runHook preBuild

    ${lib.concatMapStringsSep "\n" (a: "mvn package -Dmaven.repo.local=$out/.m2 ${a}") mvnArgs}

    runHook postBuild
  '';

  installPhase = ''
    runHook preInstall

    # yoinked from buildMavenPackage
    find $out/.m2 -type f \( \
      -name \*.lastUpdated \
      -o -name resolver-status.properties \
      -o -name _remote.repositories \) \
      -delete

    # pnpm puts the current timestamp into those index files, just zero those out
    find $out/pnpm/v3 -type f \
      -name \*-index.json \
      -execdir sh -c "jq -c 'walk(if type == \"object\" and has(\"checkedAt\") then .checkedAt |= 0 else . end)' {} | sponge {}" \;

    runHook postInstall
  '';
}
