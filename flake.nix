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
          inherit (lib) mkEnableOption mkOption types;
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
               type = types.nullOr types.path;
               default = "/var/lib/dolly";
               description = "Directory for dolly data files (seen_events.json, matrix_store.sqlite, matrix_session.json).";
             };
          };

          config = lib.mkIf cfg.enable {
            systemd.services.dolly = {
              description = "dolly";
              after = [ "network-online.target" ];
              wants = [ "network-online.target" ];
              wantedBy = [ "multi-user.target" ];

               serviceConfig = {
                 Type = "simple";
                 DynamicUser = true;
                 Environment = [
                   "RUST_LOG=dolly=info"
                   "PORT=${toString cfg.port}"
                   "DOLLY_DATA_DIR=${cfg.dataDir}"
                 ];
                EnvironmentFile = lib.mkIf (cfg.environmentFile != null) cfg.environmentFile;
                ExecStart = lib.getExe cfg.package;
                Restart = "always";

                PrivateNetwork = false;

                ProtectHostname = false;
                ProtectKernelTunables = false;
                ProtectKernelModules = false;
                ProtectControlGroups = false;

                NoNewPrivileges = true;
                PrivateDevices = true;
                PrivateIPC = true;
                PrivateTmp = true;
                PrivateUsers = true;
                ProtectClock = true;
                ProtectHome = true;
                ProtectKernelLogs = true;
                RestrictNamespaces = [
                  "uts"
                  "ipc"
                  "pid"
                  "user"
                  "cgroup"
                ];
                RestrictRealtime = true;
                RestrictSUIDSGID = true;
                SystemCallArchitectures = "native";

                SystemCallFilter = [
                  "@system-service"
                  "@network"
                ];

                BindAddress = [ "0.0.0.0:${toString cfg.port}" ];

                UMask = "0077";
              };
            };
          };
        };
    };
}
