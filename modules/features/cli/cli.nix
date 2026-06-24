{ self, inputs, ... }: {
  flake.nixosModules.cli-tools = { pkgs, lib, ... }: {
    # imports = with self.nixosModules; [
    #   nix-index
    #   lf
    # ];

    environment.systemPackages =
      [ self.packages.${pkgs.stdenv.hostPlatform.system}.myCli ];
  };

  perSystem = { pkgs, self', ... }: {
    packages.myCli = pkgs.writeShellApplication {
      name = "my-cli";

      runtimeInputs = with pkgs; [
        self'.packages.myLf
        self'.packages.myFish
        self'.packages.myGit
        nix-index
        zellij
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
        helix
      ];

      text = ''exec ${self'.packages.myFish}/bin/fish -i'';
    };
  };
}
