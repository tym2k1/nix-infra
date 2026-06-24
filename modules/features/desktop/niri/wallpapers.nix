{ self, inputs, ... }: {
  perSystem = { pkgs, lib, ... }: {
    packages.myWallpapers = pkgs.stdenv.mkDerivation {
      pname = "myWallpapers";
      version = "1.0";

      src = ./images;

      installPhase = ''
        mkdir -p $out
        cp -r $src/* $out/
      '';
  };
};
}
