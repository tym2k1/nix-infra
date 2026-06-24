{ self, inputs, ... }: {
  flake.nixosModules.git = { pkgs, ... }: {
    environment.systemPackages = [
      self.packages.${pkgs.stdenv.hostPlatform.system}.myGit
    ];
  };

  perSystem = { pkgs, ... }: let

    gitIdentity = pkgs.writeShellApplication {
      name = "git-identity";

      runtimeInputs = with pkgs; [
        git
        fzf
        gnused
      ];

      text = builtins.readFile ./git-identity;
    };

    gitConfig = pkgs.writeText "gitconfig" ''

      # extremely important, otherwise git will attempt to guess a default user identity. see `man git-config` for more details
      [user]
        useConfigOnly = true

      [user "personal"]
        name = Tymoteusz Burak
        email = tymbur@gmail.com

      [user "work"]
        name = Tymoteusz Burak
        email =

      [alias]
        identity = "!git-identity"
        id = "!git-identity"

      ${builtins.readFile ./gitconfig}
    '';

  in {
    packages.myGit = pkgs.writeShellApplication {
      name = "git";

      runtimeInputs = [
        pkgs.git
        pkgs.gnupg
        gitIdentity
      ];

    text = ''
      export GIT_CONFIG_GLOBAL=${gitConfig}
      exec git "$@"
    '';
    };
  };
}
