{
  description = "comp3931";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixpkgs-unstable";

    crane.url = "github:ipetkov/crane";

    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    advisory-db = {
      url = "github:rustsec/advisory-db";
      flake = false;
    };
  };

  outputs = {
    self,
    nixpkgs,
    ...
  } @ inputs: let
    inherit (nixpkgs) lib;

    forAllSystems = function:
      lib.genAttrs lib.systems.flakeExposed (
        system: let
          pkgs = nixpkgs.legacyPackages.${system};
          toolchain = (with inputs.fenix.packages.${system};
            combine [
              latest.cargo
              latest.clippy
              latest.rust-analyzer
              latest.rust-docs
              latest.rust-src
              latest.rust-std
              latest.rustc
              latest.rustfmt
            ]).overrideAttrs (final: prev: {
            # WATCH: https://github.com/nix-community/fenix/issues/155
            buildCommand =
              prev.buildCommand
              + ''
                ls $out/etc/bash_completion.d
                if [ -d $out/etc/bash_completion.d ]; then
                  mkdir -p $out/share/bash-completion/completions
                  cp $out/etc/bash_completion.d/* $out/share/bash-completion/completions
                fi
              '';
          });

          craneLib = (inputs.crane.mkLib pkgs).overrideToolchain toolchain;

          commonArgs = {
            src = let
              root = ./.;
            in
              lib.fileset.toSource {
                inherit root;

                fileset = lib.fileset.unions [
                  (craneLib.fileset.commonCargoSources root)
                  (lib.fileset.fileFilter (file: file.hasExt "wgsl") root)
                ];
              };
            strictDeps = true;
          };

          cargoArtifacts = craneLib.buildDepsOnly commonArgs;
        in
          function {inherit system pkgs craneLib commonArgs cargoArtifacts;}
      );
  in {
    packages = forAllSystems ({
      system,
      craneLib,
      commonArgs,
      cargoArtifacts,
      ...
    }: let
      package = craneLib.buildPackage (commonArgs
        // {
          inherit cargoArtifacts;
        });
    in {
      default = package // {meta.mainProgram = package.pname;};
    });

    apps = forAllSystems ({system, ...}: {
      default = {
        program = "${lib.getExe self.packages.${system}.default}";
        type = "app";
      };
    });

    devShells = forAllSystems ({
      system,
      craneLib,
      ...
    }: {
      default = craneLib.devShell {
        checks = self.checks.${system};
      };
    });

    checks = forAllSystems ({
      system,
      craneLib,
      commonArgs,
      cargoArtifacts,
      ...
    }: {
      inherit (self.packages.${system}) default;

      cargo-audit = craneLib.cargoAudit {
        inherit (inputs) advisory-db;
        inherit (commonArgs) src;
      };

      cargo-clippy = craneLib.cargoClippy (commonArgs
        // {
          inherit cargoArtifacts;
          cargoClippyExtraArgs = "--all-targets -- --deny warnings";
        });

      cargo-deny = craneLib.cargoDeny {inherit (commonArgs) src;};

      cargo-doc = craneLib.cargoDoc (commonArgs // {inherit cargoArtifacts;});

      cargo-fmt = craneLib.cargoFmt {inherit (commonArgs) src;};

      cargo-test = craneLib.cargoNextest (
        commonArgs
        // {
          inherit cargoArtifacts;
          partitions = 1;
          partitionType = "count";
          cargoNextestPartitionsExtraArgs = "--no-tests=pass";
        }
      );
    });
  };
}
