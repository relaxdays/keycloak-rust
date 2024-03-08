{
  description = "env";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    microvm.url = "github:999eagle/microvm.nix/fix/crash-on-sgx";
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
      overlays.default = self: super: let
        keycloak-builder = self.callPackage ./nix/keycloak {
          keycloak = super.keycloak;
        };
      in rec {
        inherit keycloak-builder;
        keycloak = keycloak-builder {
          version = "unstable-2024-03-03";
          rev = "14a12d106aeefa3c2e27c9b37bf9f294caacd2be";
          hash = "sha256-hGOazvkVTVWVo6/YCq2iu3RwgSTf9ihTIDE4QQy87hE=";
          patches = [
            # https://github.com/keycloak/keycloak/pull/26867
            (self.fetchpatch {
              url = "https://github.com/keycloak/keycloak/commit/76a4f00d2964d64ae7a6bcbd5622b8250fa87ecd.patch";
              hash = "sha256-z+TImkTa9GjFZ7anRtoEGjKeRoosJGEqsC5VLWyjUFo=";
            })
          ];
          depsHash = "sha256-AdH1GR+Ifc4+U2AoG9IoRd7BE8GJOm/SzH3imjN6ZkA=";
        };
        keycloak-openapi = keycloak.api;
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
        openapi = pkgs.runCommand "keycloak-openapi" {} ''
          mkdir -p $out
          cp -r ${pkgs.keycloak-openapi}/* $out/
        '';
        vm-test = self.nixosConfigurations.${system}.keycloak-vm.config.microvm.declaredRunner;
        default = keycloak;
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
            pkgs,
            ...
          }: {
            services.keycloak = {
              enable = true;
              package = pkgs.keycloak;
              initialAdminPassword = "Start123!";
              settings = {
                #hostname = config.networking.hostName;
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
          })
        ];
      };
    });
}
