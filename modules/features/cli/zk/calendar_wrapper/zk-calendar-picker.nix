{ self, ... }:
{
  flake.nixosModules.zk-calendar-picker = { pkgs, ... }: {
    environment.systemPackages = [
      self.packages.${pkgs.stdenv.hostPlatform.system}.zkCalendarPicker
    ];
  };

  perSystem = { pkgs, ... }:
    let
      zkCalendarPicker = pkgs.rustPlatform.buildRustPackage {
        pname = "zk-calendar-picker";
        version = "0.1.0";

        src = ./src;

        cargoLock = {
          lockFile = ./src/Cargo.lock;
        };

        meta = {
          description = "Interactive calendar picker for zk daily notes";
          mainProgram = "zk-calendar-picker";
        };
      };
    in {
      packages = {
        inherit zkCalendarPicker;
        default = zkCalendarPicker;
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
