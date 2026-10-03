{ self, inputs, ... }: {
  flake.nixosModules.niri = { pkgs, lib, ... }: {
    programs.niri = {
      enable = true;
      package = self.packages.${pkgs.stdenv.hostPlatform.system}.myNiri;
    };

    services.upower.enable = true;
    services.greetd = {
       enable = true;
       useTextGreeter = true;
       settings = {
         default_session = {
           command = "${pkgs.tuigreet}/bin/tuigreet --time --cmd niri";
           user = "greeter";
         };
       };
    };

    environment.systemPackages = with pkgs; [
      flameshot
      grim
    ];

    console = {
      earlySetup = true;
      font = "${pkgs.terminus_font}/share/consolefonts/ter-116n.psf.gz";
      packages = with pkgs; [ terminus_font ];
    };
    xdg.portal = {
      enable = true;
      xdgOpenUsePortal = true;
      extraPortals = [
        pkgs.xdg-desktop-portal-gtk
        pkgs.xdg-desktop-portal-gnome
      ];
      config = {
        common.default = [ "gnome" ];
      };
    };
  };

  perSystem = { pkgs, lib, self', ... }:
    let
    config = pkgs.writeText "niri-config.kdl" ''
      xwayland-satellite {
        path "${lib.getExe pkgs.xwayland-satellite}"
      }

      binds {
        Mod+Return {
          spawn "${lib.getExe self'.packages.myWezterm}"
        }

        Mod+M repeat=false {
          spawn-sh "${pkgs.wl-mirror}/bin/wl-mirror $(niri msg --json focused-output | ${pkgs.jq}/bin/jq -r .name)"
        }

        ${builtins.readFile ./binds.kdl}
      }

      ${builtins.readFile ./niri-config.kdl}
    '';
  in {
    packages.myNiri = pkgs.symlinkJoin {
      name = "niri";

      paths = [ pkgs.niri ];

      nativeBuildInputs = [ pkgs.makeWrapper ];

      postBuild = ''
        wrapProgram $out/bin/niri \
          --set NIRI_CONFIG "${config}"
         '';

      passthru.providedSessions = [ "niri" ];
    };
  };
}
