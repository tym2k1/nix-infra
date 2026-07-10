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

  perSystem = { pkgs, lib, self', ... }: {
    packages.myNiri = inputs.wrapper-modules.wrappers.niri.wrap {
      inherit pkgs;
      "config.kdl".content =
      ''
        spawn-at-startup "${(lib.getExe self'.packages.myNoctalia)}"
        xwayland-satellite { path "${lib.getExe pkgs.xwayland-satellite}"; }

        binds {
        Mod+Return { spawn "${lib.getExe pkgs.wezterm}"; }
        Mod+S { spawn-sh "${lib.getExe self'.packages.myNoctalia} ipc call launcher toggle"; }
        Mod+L { spawn-sh "${lib.getExe self'.packages.myNoctalia} ipc call lockScreen lock"; }
        Mod+V { spawn-sh "${lib.getExe self'.packages.myNoctalia} ipc call launcher clipboard"; }
        Mod+M repeat=false { spawn-sh "${pkgs.wl-mirror}/bin/wl-mirror $(niri msg --json focused-output | ${pkgs.jq}/bin/jq -r .name)"; }
        ${builtins.readFile ./binds.kdl}
        }
        ${builtins.readFile ./niri-config.kdl}
      '';
    };
  };
}
