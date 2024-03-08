{
  lib,
  stdenv,
  maven,
  nodejs,
  nodePackages,
}: {
  mvnArgs,
  src,
  version,
  deps,
}:
stdenv.mkDerivation {
  pname = "keycloak-build";
  inherit src version;

  outputs = ["out" "api"];

  nativeBuildInputs = [maven nodejs nodePackages.pnpm];

  preBuild = ''
    export HOME="$TMPDIR"
    pnpm config set store-dir ${deps}/pnpm --global
  '';

  buildPhase = ''
    runHook preBuild

    mvnDeps="$(cp -dpR ${deps}/.m2 ./ && chmod -R +w .m2 && pwd)"
    ${lib.concatMapStringsSep "\n" (a: "mvn package --offline --no-snapshot-updates -Dmaven.repo.local=\"$mvnDeps/.m2\" ${a}") mvnArgs}

    runHook postBuild
  '';

  installPhase = ''
    runHook preInstall

    mkdir -p $out $api
    cp services/target/apidocs-rest/swagger/apidocs/openapi.* $api/
    cp quarkus/dist/target/keycloak-*.tar.gz $out/keycloak.tar.gz

    runHook postInstall
  '';

  passthru = {inherit deps;};
}
