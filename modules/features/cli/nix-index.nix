{ inputs, ... }:

{
  perSystem = { pkgs, ... }:
    let
      nixIndexPkgs =
        pkgs.extend inputs.nix-index-database.overlays.nix-index;

      comma = nixIndexPkgs.callPackage
        "${inputs.nix-index-database}/comma-wrapper.nix"
        { };
    in {
      packages = {
        myComma = pkgs.symlinkJoin {
          name = "my-comma";

          paths = [ comma ];

          nativeBuildInputs = [ pkgs.makeWrapper ];

          postBuild = ''
            wrapProgram "$out/bin/," \
              --set COMMA_CACHING 0
          '';
        };

        myNixIndex = nixIndexPkgs.callPackage
          "${inputs.nix-index-database}/nix-index-wrapper.nix"
          { };
      };
    };
}
