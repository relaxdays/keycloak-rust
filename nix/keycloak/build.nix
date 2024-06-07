{
  lib,
  stdenv,
  maven,
  xmlstarlet,
  nodejs,
  nodePackages,
  pnpm,
  makeSetupHook,
}: {
  src,
  version,
  pnpmHash,
  mvnHash,
}: let
  pname = "keycloak-build";
  pnpmDeps = pnpm.fetchDeps {
    inherit src pname version;
    hash = pnpmHash;
  };
  mavenNodejsHook =
    makeSetupHook {
      name = "maven-nodejs-hook";
      substitutions = {
        node = "${nodejs}";
        pnpm = "${pnpm}";
      };
    }
    ./maven-nodejs-hook.sh;
  configurePhase = ''
    runHook preConfigure

    echo "fixing up versions in pom.xml"
    # make sure the pom.xml requests exactly the versions of node/pnpm available
    xmlstarlet edit \
      --update '/*[local-name()="project"]/*[local-name()="properties"]/*[local-name()="node.version"]' --value "v${nodejs.version}" \
      --update '/*[local-name()="project"]/*[local-name()="properties"]/*[local-name()="pnpm.version"]' --value "${pnpm.version}" \
      pom.xml > pom.xml.patched
    mv pom.xml.patched pom.xml

    runHook postConfigure
  '';
in
  maven.buildMavenPackage {
    inherit src pname version;

    outputs = ["out" "api"];

    nativeBuildInputs = [
      xmlstarlet
      nodejs
      pnpm
      mavenNodejsHook
      pnpm.configHook
    ];
    inherit pnpmDeps mvnHash configurePhase;
    mvnFetchExtraArgs = {
      inherit pnpmDeps configurePhase;
    };
    mvnParameters = "-am -pl quarkus/deployment,quarkus/dist,services -P jboss-release";
    # keycloak tests are... flaky at best
    doCheck = false;

    installPhase = ''
      runHook preInstall

      mkdir -p $out $api
      cp services/target/apidocs-rest/swagger/apidocs/openapi.* $api/
      cp quarkus/dist/target/keycloak-*.tar.gz $out/keycloak.tar.gz

      runHook postInstall
    '';
  }
