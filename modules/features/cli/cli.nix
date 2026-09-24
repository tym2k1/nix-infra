{ self, inputs, ... }: {
  flake.nixosModules.cli-tools = { pkgs, lib, ... }: {
    environment.systemPackages =
      [ self.packages.${pkgs.stdenv.hostPlatform.system}.myCli ];
    imports = with self.nixosModules; [
        nix-index
      ];
  };
  perSystem = { pkgs, self', ... }:
    let
      myPkgs = pkgs.extend (final: prev: {
        sage = prev.sage.override {
          requireSageTests = false;
        };
      });
in {
    packages.myCli = myPkgs.writeShellApplication {
      name = "my-cli";

      runtimeInputs = with myPkgs; [
        self'.packages.myLf
        self'.packages.myFish
        self'.packages.myBash
        self'.packages.myStarship
        self'.packages.myGit
        self'.packages.myHelix
        self'.packages.myZellij
        self'.packages.comma
        dust
        btop
        dragon-drop
        tldr
        fastfetch
        trash-cli
        unzip
        file
        ripgrep
        ripgrep-all
        fd
        fzf
        sage
      ];

      text = ''exec ${self'.packages.myFish}/bin/fish -i'';
    };
  };
}
