{
  description = "`collage` is a template engine for Rust, designed for writing HTML and similar markup languages.";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    crane.url = "github:ipetkov/crane";
    flake-utils.url = "github:numtide/flake-utils";
    advisory-db.url = "github:rustsec/advisory-db";
    advisory-db.flake = false;
    git-hooks.url = "github:cachix/git-hooks.nix";
    git-hooks.inputs.nixpkgs.follows = "nixpkgs";
  };

  outputs = {
    self,
    nixpkgs,
    crane,
    flake-utils,
    advisory-db,
    git-hooks,
    ...
  }:
    flake-utils.lib.eachDefaultSystem (system: let
      pkgs = nixpkgs.legacyPackages.${system};

      inherit (pkgs) lib;

      craneLib = crane.mkLib pkgs;
      unfilteredRoot = ./.;
      src = craneLib.cleanCargoSource ./.;

      # Common arguments can be set here to avoid repeating them later
      commonArgs = {
        inherit src;
        strictDeps = true;
        buildInputs = [] ++ lib.optionals pkgs.stdenv.isDarwin [pkgs.libiconv];
      };

      # Build *just* the cargo dependencies (of the entire workspace),
      # so we can reuse all of that work (e.g. via cachix) when running in CI
      # It is *highly* recommended to use something like cargo-hakari to avoid
      # cache misses when building individual top-level-crates
      cargoArtifacts = craneLib.buildDepsOnly commonArgs;

      individualCrateArgs =
        commonArgs
        // {
          inherit cargoArtifacts;
          inherit (craneLib.crateNameFromCargoToml {inherit src;}) version;
          # NB: we disable tests since we'll run them all via cargo-nextest
          doCheck = false;
        };

      fileSetForCrate = crate:
        lib.fileset.toSource {
          root = unfilteredRoot;
          fileset = lib.fileset.unions [
            ./Cargo.toml
            ./Cargo.lock
            (craneLib.fileset.commonCargoSources ./crates/collage-core)
            (craneLib.fileset.commonCargoSources ./crates/collage-macros)
            (craneLib.fileset.commonCargoSources crate)
            (lib.fileset.fileFilter (file: file.hasExt "stderr") unfilteredRoot)
          ];
        };

      # Build the top-level crates of the workspace as individual derivations.
      # This allows consumers to only depend on (and build) only what they need.
      # Though it is possible to build the entire workspace as a single derivation,
      # so this is left up to you on how to organize things
      #
      # Note that the cargo workspace must define `workspace.members` using wildcards,
      # otherwise, omitting a crate (like we do below) will result in errors since
      # cargo won't be able to find the sources for all members.
      collage = craneLib.buildPackage (individualCrateArgs
        // {
          pname = "collage";
          cargoExtraArgs = "--package collage";
          src = fileSetForCrate ./crates/collage;
        });
    in {
      checks = {
        # Build the crates as part of `nix flake check` for convenience
        inherit collage;

        # Run clippy (and deny all warnings) on the workspace source,
        # again, reusing the dependency artifacts from above.
        #
        # Note that this is done as a separate derivation so that
        # we can block the CI if there are issues here, but not
        # prevent downstream consumers from building our crate by itself.
        collage-clippy = craneLib.cargoClippy (commonArgs
          // {
            inherit cargoArtifacts;
            cargoClippyExtraArgs = "--all-targets -- --deny warnings";
          });

        collage-doc = craneLib.cargoDoc (commonArgs // {inherit cargoArtifacts;});

        # Check formatting
        collage-fmt = craneLib.cargoFmt {inherit src;};

        collage-toml-fmt = craneLib.taploFmt {
          src = pkgs.lib.sources.sourceFilesBySuffices src [".toml"];
          # taplo arguments can be further customized below as needed
          # taploExtraArgs = "--config ./taplo.toml";
        };

        # Audit dependencies
        collage-audit = craneLib.cargoAudit {inherit src advisory-db;};

        # Audit licenses
        collage-deny = craneLib.cargoDeny {inherit src;};

        # Run tests with cargo-nextest
        # Consider setting `doCheck = false` on other crate derivations
        # if you do not want the tests to run twice
        collage-nextest = craneLib.cargoNextest (commonArgs
          // {
            inherit cargoArtifacts;
            # TODO include only the *.stderr files here?
            src = fileSetForCrate ./crates/collage;
            partitions = 1;
            partitionType = "count";
            cargoNextestPartitionsExtraArgs = "--no-tests=pass";
          });

        collage-doctest = craneLib.cargoDocTest (commonArgs // {inherit cargoArtifacts;});

        git-hooks = git-hooks.lib.${system}.run {
          src = unfilteredRoot;
          settings = {
            rust.check.cargoDeps = pkgs.rustPlatform.importCargoLock {
              lockFile = ./Cargo.lock;
            };
          };
          hooks = {
            alejandra.enable = true;
            cargo-check.enable = true;
            clippy.enable = true;
            rustfmt.enable = true;
          };
        };
      };

      devShells.default = craneLib.devShell {
        # Inherit inputs from checks.
        checks = self.checks.${system};

        # Extra inputs can be added here; cargo and rustc are provided by default.
        packages = with pkgs; [
          rust-analyzer # rust lsp
          nil # nix lsp
          alejandra # nix formatter
          watchexec # code watcher
        ];
      };
    });
}
