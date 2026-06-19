{
  description = "dolly";

  inputs.nixpkgs.url = "https://channels.nixos.org/nixpkgs-unstable/nixexprs.tar.xz";

  outputs =
    { self, nixpkgs }:
    let
      systems = [
        "x86_64-linux"
        "aarch64-linux"
        "aarch64-darwin"
      ];

      forAllSystems =
        function: nixpkgs.lib.genAttrs systems (system: function nixpkgs.legacyPackages.${system});
    in
    {
      packages = forAllSystems (pkgs: rec {
        default = dolly;
        dolly =
          let
            package = (pkgs.lib.importTOML ./Cargo.toml).package;
          in
          pkgs.rustPlatform.buildRustPackage {
            pname = package.name;
            inherit (package) version;

            src = pkgs.lib.fileset.toSource {
              root = ./.;
              fileset =
                pkgs.lib.fileset.intersection (pkgs.lib.fileset.fromSource (pkgs.lib.sources.cleanSource ./.))
                  (
                    pkgs.lib.fileset.unions [
                      ./Cargo.toml
                      ./Cargo.lock
                      ./src
                    ]
                  );
            };

            cargoLock.lockFile = ./Cargo.lock;

            buildInputs = with pkgs; [
              openssl
              sqlite
            ];
            nativeBuildInputs = with pkgs; [ pkg-config ];

            meta.mainProgram = "dolly";
          };
      });

      devShells = forAllSystems (pkgs: {
        default = self.packages.${pkgs.stdenv.hostPlatform.system}.dolly.overrideAttrs (oa: {
          nativeBuildInputs = [
            pkgs.rustfmt
            pkgs.rust-analyzer
            pkgs.cargo
            pkgs.rustc
          ]
          ++ (oa.nativeBuildInputs or [ ]);
        });
      });

      nixosModules.default =
        {
          lib,
          config,
          pkgs,
          ...
        }:
        let
          inherit (lib)
            mkEnableOption
            mkOption
            types
            optionalAttrs
            ;
          cfg = config.services.dolly;
        in
        {
          options.services.dolly = {
            enable = mkEnableOption "dolly";

            package = mkOption {
              type = types.package;
              default = self.packages.${pkgs.stdenv.hostPlatform.system}.dolly;
              defaultText = lib.literalExpression "self.packages.\${pkgs.stdenv.hostPlatform.system}.dolly";
              description = "The dolly package to run.";
            };

            environmentFile = mkOption {
              type = types.nullOr types.path;
              default = null;
              description = "Environment file for dolly.";
            };

            port = mkOption {
              type = types.port;
              default = 3000;
              description = "Port for dolly to listen on.";
            };

            dataDir = mkOption {
              type = types.path;
              default = "/var/lib/dolly";
              description = "Directory for dolly data files.";
            };

            user = mkOption {
              type = types.str;
              default = "dolly";
              description = "User account under which dolly runs.";
            };

            group = mkOption {
              type = types.str;
              default = "dolly";
              description = "Group account under which dolly runs.";
            };
          };

          config = lib.mkIf cfg.enable {
            systemd.tmpfiles.rules = [
              "d ${cfg.dataDir} 0750 ${cfg.user} ${cfg.group} - -"
            ];

            systemd.services.dolly = {
              description = "dolly";
              after = [ "network-online.target" ];
              wants = [ "network-online.target" ];
              wantedBy = [ "multi-user.target" ];

              serviceConfig = {
                Type = "simple";
                User = cfg.user;
                Group = cfg.group;
                WorkingDirectory = cfg.dataDir;

                Environment = [
                  "RUST_LOG=dolly=info"
                  "PORT=${toString cfg.port}"
                  "DOLLY_DATA_DIR=${cfg.dataDir}"
                ];
                EnvironmentFile = lib.mkIf (cfg.environmentFile != null) cfg.environmentFile;

                ExecStart = lib.getExe cfg.package;
                Restart = "always";
                RestartSec = 1;

                AmbientCapabilities = "";
                CapabilityBoundingSet = "";
                LockPersonality = true;
                MemoryDenyWriteExecute = true;
                NoNewPrivileges = true;
                PrivateDevices = true;
                PrivateIPC = true;
                PrivateNetwork = false;
                PrivateTmp = true;
                PrivateUsers = true;
                ProcSubset = "pid";
                ProtectClock = true;
                ProtectControlGroups = true;
                ProtectHome = true;
                ProtectHostname = true;
                ProtectKernelLogs = true;
                ProtectKernelModules = true;
                ProtectKernelTunables = true;
                ProtectProc = "invisible";
                ProtectSystem = "strict";
                ReadWritePaths = [ cfg.dataDir ];
                RemoveIPC = true;
                RestrictAddressFamilies = [
                  "AF_UNIX"
                  "AF_INET"
                  "AF_INET6"
                ];
                RestrictNamespaces = true;
                RestrictRealtime = true;
                RestrictSUIDSGID = true;
                SystemCallArchitectures = "native";
                SystemCallFilter = [
                  "~@clock"
                  "~@cpu-emulation"
                  "~@debug"
                  "~@module"
                  "~@mount"
                  "~@obsolete"
                  "~@privileged"
                  "~@raw-io"
                  "~@reboot"
                  "~@resources"
                  "~@swap"
                ];
                UMask = "0077";
              };
            };

            users.users = optionalAttrs (cfg.user == "dolly") {
              dolly = {
                isSystemUser = true;
                group = cfg.group;
                description = "dolly service user";
                home = cfg.dataDir;
              };
            };

            users.groups = optionalAttrs (cfg.group == "dolly") {
              dolly = { };
            };
          };
        };
    };
}
