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
      nixosModules.keycloak-vm = {pkgs, ...}: {
        services.keycloak = {
          enable = true;
          package = pkgs.keycloak;
          initialAdminPassword = "Start123!";
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
        systemd.services.keycloak = {
          path = with pkgs; [curl jq];
          postStart = ''
            set +e
            while true; do
              if ! kill -0 "$MAINPID"; then exit 1; fi
              status="$(set -o pipefail; curl -Ss http://localhost/health | jq -r '.status')"
              exit="$?"
              if [[ "$exit" -eq 0 && "$status" == "UP" ]]; then break; fi
              sleep 1
            done
            set -e
          '';
        };
        networking.firewall.allowedTCPPorts = [80];
      };
      lib.build-keycloak-vm = {
        system,
        extraModules,
        ...
      }:
        nixpkgs.lib.nixosSystem {
          inherit system;
          modules =
            [
              microvm.nixosModules.microvm
              self.nixosModules.keycloak-vm
              {
                networking.hostName = "keycloak-vm";
                users.users.root.password = "root";
                services.getty.autologinUser = "root";
                systemd.network.networks."90-fallback" = {
                  matchConfig.Name = ["en*" "eth*"];
                  domains = ["~."];
                  DHCP = "yes";
                };
                microvm = {
                  hypervisor = "qemu";
                  socket = "control.socket";
                  preStart = ''
                    mkdir -p shared
                  '';
                  shares = [
                    {
                      proto = "9p";
                      tag = "ro-store";
                      source = "/nix/store";
                      mountPoint = "/nix/.ro-store";
                    }
                    {
                      proto = "9p";
                      tag = "meow";
                      source = "./shared/";
                      mountPoint = "/opt/shared";
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
            ]
            ++ extraModules;
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
      nixosConfigurations.keycloak-vm = self.lib.build-keycloak-vm {
        inherit system;
        extraModules = [
          {nixpkgs.pkgs = pkgs;}
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
            environment.systemPackages = [
              wrappedApiExamplePkg
            ];
            systemd.services.keycloak-rust-test = {
              after = ["keycloak.service"];
              wants = ["keycloak.service"];
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
          })
        ];
      };
    });
}
