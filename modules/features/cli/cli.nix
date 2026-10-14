{ self, inputs, ... }: {
  flake.nixosModules.cli-tools = { pkgs, lib, ... }: {
    environment.systemPackages = [
      self.packages.${pkgs.stdenv.hostPlatform.system}.myCli
    ];
  };

  perSystem = { pkgs, self', ... }:
    let

      coreRuntimeInputs = with pkgs; [
        self'.packages.myFish
        self'.packages.myBash
        self'.packages.myGit
        self'.packages.myHelix
        self'.packages.myStarship
        self'.packages.myZellij
        self'.packages.myLf
        self'.packages.myComma
        self'.packages.myNixIndex
        file
        tldr
        ripgrep
        ripgrep-all
        trash-cli
        dust
        btop
        fastfetch
        unzip
        fd
        dragon-drop
      ];

      additionalRuntimeInputs = with pkgs; [
        self'.packages.myZk
      ];

      mkMyCli = runtimeInputs:
        pkgs.writeShellApplication {
          name = "my-cli";
          inherit runtimeInputs;

          text = ''
            exec ${self'.packages.myFish}/bin/fish -i
          '';
        };
    in {
      packages = {
        myCli = mkMyCli (
          coreRuntimeInputs ++ additionalRuntimeInputs
        );

        "myCli-minimal" = mkMyCli coreRuntimeInputs;
      };
    };
}
