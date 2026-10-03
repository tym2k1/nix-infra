{ self, ... }:
{
  flake.nixosModules.wallpaper-gen = { pkgs, ... }: {
    environment.systemPackages = [
      self.packages.${pkgs.stdenv.hostPlatform.system}.wallpaperGen
    ];
  };

  perSystem = { pkgs, ... }:
    let
      wallpaperGen = pkgs.rustPlatform.buildRustPackage {
        pname = "wallpaper-gen";
        version = "0.1.0";

        src = ./src;

        cargoLock = {
          lockFile = ./src/Cargo.lock;
        };

        meta = {
          description = "Fancy util to generate wallpapers based on nix hash";
          mainProgram = "wallpaper-gen";
        };
      };
    in {
      packages = {
        inherit wallpaperGen;
        default = wallpaperGen;
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
