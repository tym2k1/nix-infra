{ self, inputs, pkgs, lib, ... }: {
  perSystem = { pkgs, lib, ... }: let
    # First define wallpapers
    myWallpapers = pkgs.stdenv.mkDerivation {
      pname = "myWallpapers";
      version = "1.0";
      src = ./images;
      installPhase = ''
        mkdir -p $out
        cp -r $src/* $out/
      '';
    };

    # Now you can reference it
    wallpaperDir = "${myWallpapers}";

    # Read base settings
    baseSettings = (builtins.fromJSON (builtins.readFile ./noctalia.json)).settings;

    # Merge override
    settingsWithWallpaper = lib.recursiveUpdate baseSettings {
      wallpaper = { "directory" = wallpaperDir; };
    };
  in {
    packages.myWallpapers = myWallpapers;

    packages.myNoctalia = inputs.wrapper-modules.wrappers.noctalia-shell.wrap {
      inherit pkgs;
      colors = {
        mError = "#dddddd";
        mOnError = "#111111";
        mOnPrimary = "#111111";
        mOnSecondary = "#111111";
        mOnSurface = "#828282";
        mOnSurfaceVariant = "#5d5d5d";
        mOnTertiary = "#111111";
        mOutline = "#3c3c3c";
        mPrimary = "#aaaaaa";
        mSecondary = "#a7a7a7";
        mShadow = "#000000";
        mSurface = "#111111";
        mSurfaceVariant = "#191919";
        mTertiary = "#cccccc";
      };
      settings = settingsWithWallpaper;
    };
  };
}
