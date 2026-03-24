{
  description = "comp3931";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixpkgs-unstable";

    crane.url = "github:ipetkov/crane";

    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    treefmt-nix = {
      url = "github:numtide/treefmt-nix";
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
            ]).overrideAttrs (_: prev: {
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

          commonArgs = let
            root = ./.;
          in {
            strictDeps = true;

            src = lib.fileset.toSource {
              inherit root;
              fileset = lib.fileset.unions [
                (craneLib.fileset.commonCargoSources root)
                ./assets
              ];
            };
          };

          cargoArtifacts = craneLib.buildDepsOnly commonArgs;

          treefmt = inputs.treefmt-nix.lib.evalModule pkgs {
            projectRootFile = "flake.nix";

            settings.global.excludes = [
              "/assets/textures/*"
              "/flake.lock"
              "/LICENSE"
            ];

            programs = {
              # nix
              alejandra.enable = true;
              deadnix.enable = true;
              statix.enable = true;

              # project
              rustfmt = {
                enable = true;
                package = toolchain;
              };
              taplo.enable = true;
              wgslfmt.enable = true;
            };
          };
        in
          function {inherit system pkgs craneLib commonArgs cargoArtifacts treefmt;}
      );
  in {
    packages = forAllSystems ({
      pkgs,
      craneLib,
      commonArgs,
      cargoArtifacts,
      ...
    }: let
      libs = with pkgs;
        lib.makeLibraryPath [
          libxkbcommon
          vulkan-loader
          libGL
          wayland
          xorg.libX11
          xorg.libXi
          xorg.libXtst
          xorg.libXcursor
        ];

      package = craneLib.buildPackage (commonArgs
        // {
          inherit cargoArtifacts;

          nativeBuildInputs = lib.optionals pkgs.stdenv.isLinux [
            pkgs.makeWrapper
          ];

          postInstall = lib.optionalString pkgs.stdenv.isLinux ''
            wrapProgram $out/bin/comp3931 --set LD_LIBRARY_PATH ${libs}
          '';
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
      treefmt,
      craneLib,
      ...
    }: {
      default = craneLib.devShell {
        packages = builtins.attrValues treefmt.config.build.programs ++ [self.formatter.${system}];
        checks = self.checks.${system};
      };
    });

    formatter = forAllSystems ({treefmt, ...}: treefmt.config.build.wrapper);

    checks = forAllSystems ({
      system,
      treefmt,
      craneLib,
      commonArgs,
      cargoArtifacts,
      ...
    }: {
      inherit (self.packages.${system}) default;

      formatting = treefmt.config.build.check self;

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
