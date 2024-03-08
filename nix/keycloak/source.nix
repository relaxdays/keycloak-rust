{
  applyPatches,
  xmlstarlet,
  nodejs,
  nodePackages,
  gnused,
}: {
  src,
  patches,
  postPatch ? "",
}:
applyPatches {
  inherit src patches;
  nativeBuildInputs = [
    xmlstarlet
    nodejs
    nodePackages.pnpm
  ];

  postPatch =
    ''
      # make sure the pom.xml requests exactly the versions of node/pnpm available
      xmlstarlet edit \
        --update '/*[local-name()="project"]/*[local-name()="properties"]/*[local-name()="node.version"]' --value "$(node --version)" \
        --update '/*[local-name()="project"]/*[local-name()="properties"]/*[local-name()="pnpm.version"]' --value "$(pnpm --version)" \
        pom.xml > pom.xml.patched
      mv pom.xml.patched pom.xml

      # "install" node+packages so the maven plugin finds and uses these
      mkdir -p js/node node/node_modules
      ln -sf ${nodejs}/bin/node js/node/node
      ln -sf ${nodejs}/bin/node node/node
      ln -sf ${nodejs}/bin/npm node/npm
      ln -sf ${nodejs}/bin/npx node/npx
      ln -sf ${nodejs}/lib/node_modules/npm node/node_modules/npm
      ln -sf ${nodePackages.pnpm}/bin/pnpm node/pnpm
      ln -sf ${nodePackages.pnpm}/lib/node_modules/pnpm node/node_modules/pnpm
    ''
    + postPatch;
}
