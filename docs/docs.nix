{ ... }: {
  perSystem = { pkgs, unstablePkgs, lib, ensureAtRepositoryRoot, mkCi, ... }:
    let
      pkgsDeps = with pkgs; [ pkg-config ];
      nodeDeps = with unstablePkgs; [ vips nodePackages_latest.nodejs ];
      combinedDeps = pkgsDeps ++ nodeDeps;
    in
    {
      packages = {
        docs = mkCi false (unstablePkgs.buildNpmPackage {
          npmDepsHash = "sha256-53vSl7xI61Hvtjnk0HZE9m5xoz4Otv23E0iA3+KoNZc=";
          src = ./.;
          srcs = [ ./. ./../evm/. ./../networks/genesis/. ./../versions/. ];
          sourceRoot = "docs";
          pname = "docs";
          version = "0.0.1";
          nativeBuildInputs = combinedDeps;
          buildInputs = combinedDeps;
          installPhase = ''
            mkdir -p $out
            cp -r ./dist/* $out
          '';
          doDist = false;
          PUPPETEER_SKIP_DOWNLOAD = 1;
          ASTRO_TELEMETRY_DISABLED = 1;
          NODE_OPTIONS = "--no-warnings";
        });
      };

      apps = {
        docs-dev-server = {
          type = "app";
          program = pkgs.writeShellApplication {
            name = "docs-dev-server";
            runtimeInputs = combinedDeps;
            text = ''
              ${ensureAtRepositoryRoot}
              cd docs/

              export PUPPETEER_SKIP_DOWNLOAD=1 
              npm install
              npm run dev
            '';
          };
        };
        docs-check = {
          type = "app";
          program = pkgs.writeShellApplication {
            name = "docs-check";
            runtimeInputs = combinedDeps;
            text = ''
              ${ensureAtRepositoryRoot}
              cd docs/
              npm_config_yes=true npx astro check
            '';
          };
        };
      };
    };
}