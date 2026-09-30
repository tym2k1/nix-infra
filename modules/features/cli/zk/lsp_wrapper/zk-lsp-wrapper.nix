{ self, ... }:
{
  flake.nixosModules.zk-lsp-wrapper = { pkgs, ... }: {
    environment.systemPackages = [
      self.packages.${pkgs.stdenv.hostPlatform.system}.zkLSPWrapper
    ];
  };

  perSystem = { pkgs, ... }:
    let
      zkLSPWrapper = pkgs.rustPlatform.buildRustPackage {
        pname = "zk-lsp-wrapper";
        version = "0.1.0";

        src = ./src;

        cargoLock = {
          lockFile = ./src/Cargo.lock;
        };

        nativeBuildInputs = [ pkgs.makeWrapper ];

        postInstall = ''
          wrapProgram $out/bin/zk-lsp-wrapper \
            --run 'export ZK_NOTEBOOK_DIR="$HOME/Notes"'
        '';

        meta = {
          description = "LSP wrapper for zk to not use relative paths in wikilinks";
          mainProgram = "zk-lsp-wrapper";
        };
      };
    in {
      packages = {
        inherit zkLSPWrapper;
        default = zkLSPWrapper;
      };

      devShells.default = pkgs.mkShell {
        packages = [
          pkgs.cargo
          pkgs.clippy
          pkgs.rustc
          pkgs.rustfmt
        ];
      };
    };
}
