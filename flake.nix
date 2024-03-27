{
  description = "env";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    microvm.url = "github:astro/microvm.nix";
    microvm.inputs.nixpkgs.follows = "nixpkgs";
    microvm.inputs.flake-utils.follows = "flake-utils";
  };

  outputs = {
    self,
    nixpkgs,
    flake-utils,
    microvm,
  }:
    {
      overlays.default = self: super: {
        keycloak-builder = self.callPackage ./nix/keycloak {
          keycloak = super.keycloak;
        };
        keycloak = self.keycloak-builder {
          version = "unstable-2024-03-26";
          rev = "be32f8b1bfb4fa709aa48552a1bbfdcf9fab6299";
          hash = "sha256-HRqPk4pw4ol3GcP2X9O59TBgR+PrZKSaz7pq6wtx5QM=";
          patches = [
            # https://github.com/keycloak/keycloak/pull/26867
            (self.fetchpatch {
              url = "https://github.com/keycloak/keycloak/pull/26867/commits/08339da50f39734b051e2a64e2c310e8f5eef4a7.patch";
              hash = "sha256-KewvoZjqxSd30gs19aSU5PhWvrF0F1SXisKKhHTL9mM=";
            })
          ];
          depsHash = "sha256-dOnnQUbtlUkNQ/v8jBUj1QtdtYQqi0fFOTW80jDUEvI=";
        };
        keycloak-openapi = self.keycloak.api;
        keycloak-api-rust = self.callPackage ./nix/crate {};
      };
    }
    // flake-utils.lib.eachDefaultSystem (system: let
      pkgs = import nixpkgs {
        inherit system;
        overlays = [self.overlays.default];
      };
    in {
      packages = rec {
        keycloak = pkgs.keycloak;
        api-rust = pkgs.keycloak-api-rust;
        openapi = pkgs.runCommand "keycloak-openapi" {} ''
          mkdir -p $out
          cp -r ${pkgs.keycloak-openapi}/* $out/
        '';
        vm-test = self.nixosConfigurations.${system}.keycloak-vm.config.microvm.declaredRunner;
        default = keycloak;
      };
      devShells.default = pkgs.mkShell {
        inputsFrom = [pkgs.keycloak-api-rust];
        packages = [pkgs.clippy];
        OPENAPI_SPEC_PATH = "${pkgs.keycloak-openapi}/openapi.json";
        RUST_SRC_PATH = "${pkgs.rustPlatform.rustLibSrc}";
      };
      nixosConfigurations.keycloak-vm = nixpkgs.lib.nixosSystem {
        inherit system;
        modules = [
          microvm.nixosModules.microvm
          {
            nixpkgs.pkgs = pkgs;
            networking.hostName = "keycloak-vm";
            users.users.root.password = "root";
            services.getty.autologinUser = "root";
            microvm = {
              hypervisor = "qemu";
              socket = "control.socket";
              shares = [
                {
                  proto = "9p";
                  tag = "ro-store";
                  source = "/nix/store";
                  mountPoint = "/nix/.ro-store";
                }
              ];
              volumes = [
                {
                  mountPoint = "/var";
                  image = "var.img";
                  size = 256;
                }
              ];
              interfaces = [
                {
                  type = "user";
                  id = "veth-kc";
                  mac = "02:00:00:00:00:01";
                }
              ];
              forwardPorts = [
                {
                  from = "host";
                  host.port = 8080;
                  guest.port = 80;
                }
              ];
              vcpu = 2;
              mem = 1024;
            };
          }
          ({
            config,
            lib,
            pkgs,
            ...
          }: let
            kcEnv = {
              KEYCLOAK_BASE_URL = "http://localhost";
              KEYCLOAK_REALM = "master";
              KEYCLOAK_USERNAME = "admin";
              KEYCLOAK_PASSWORD = "Start123!";
            };
            wrappedApiExamplePkg = let
              wrapperArgs = lib.flatten (lib.mapAttrsToList (env: val: ["--set-default" env val]) kcEnv);
            in
              pkgs.runCommand "kc-rust-wrapped" {nativeBuildInputs = with pkgs; [makeWrapper];} ''
                mkdir -p $out/bin
                makeWrapper "${pkgs.keycloak-api-rust}/bin/keycloak-api-example" "$out/bin/keycloak-api-example" \
                  ${lib.escapeShellArgs wrapperArgs}
              '';
          in {
            services.keycloak = {
              enable = true;
              package = pkgs.keycloak;
              initialAdminPassword = kcEnv.KEYCLOAK_PASSWORD;
              settings = {
                #hostname = config.networking.hostName;
                health-enabled = true;
                # these are dev-mode settings
                hostname = "";
                http-enabled = true;
                hostname-strict = false;
                hostname-debug = true;
              };
              database = {
                createLocally = true;
                passwordFile = "${pkgs.writeText "pwd" "somelongrandomstringidontcare"}";
              };
            };
            networking.firewall.allowedTCPPorts = [80];
            environment.systemPackages = [
              wrappedApiExamplePkg
            ];
            systemd.services.keycloak-rust-test = {
              after = ["keycloak-ready.service"];
              wants = ["keycloak-ready.service"];
              wantedBy = ["multi-user.target"];
              serviceConfig = {
                Type = "oneshot";
              };
              environment = kcEnv;
              script = ''
                keycloak-api-example
              '';
              path = with pkgs; [keycloak-api-rust];
            };

            # simple health-check/wait service
            # probably we should find some way to make keycloak use sd_notify
            systemd.services.keycloak-ready = {
              after = ["keycloak.service"];
              bindsTo = ["keycloak.service"];
              wantedBy = ["keycloak.service"];
              serviceConfig = {
                Type = "oneshot";
                RemainAfterExit = true;
              };
              script = ''
                set +e
                while true; do
                  status="$(set -o pipefail; curl -v http://localhost/health | jq -r '.status')"
                  exit="$?"
                  if [[ "$exit" -eq 0 ]]; then
                    if [[ "$status" == "UP" ]]; then
                      exit 0
                    else
                      exit 1
                    fi
                  fi
                  echo "waiting..."
                  sleep 1
                done
              '';
              path = with pkgs; [curl jq];
            };
          })
        ];
      };
    });
}
