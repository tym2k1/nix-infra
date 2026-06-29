{ self, inputs, ... }: {

  # Define module as a nixos config
  flake.nixosConfigurations.HP-Victus15 = inputs.nixpkgs.lib.nixosSystem {
    modules = [
      self.nixosModules.HP-Victus15Configuration
    ];
  };

  # Module definition
  flake.nixosModules.HP-Victus15Configuration = { pkgs, lib, ... }: {
    # import any other modules from here
    imports = with self.nixosModules; [
      HP-Victus15Hardware
      niri
      cli-tools
      steam
      lix
      audio
      desktop-utils
    ];

    system.stateVersion = "25.11";

    nix.settings.experimental-features = [ "nix-command" "flakes" ];

    environment.systemPackages = with pkgs; [
        openvpn # TODO Maybe to be moved to a feature
        vim
        git
    ];

    # Bootloader.
    boot.loader.systemd-boot.enable = true;
    boot.loader.efi.canTouchEfiVariables = true;

    # Boot eye-candy
    boot.initrd.systemd.enable = true;
    boot.loader.systemd-boot.consoleMode = "max";
    boot = {
        plymouth = {
            enable = true;
        };
        kernelModules = [ "quiet" "splash" ];
    };

    networking.hostName = "HP_Victus_15"; # Define your hostname.

    # Enable networking
    networking.networkmanager.enable = true;

    # Set your time zone.
    time.timeZone = "Europe/Warsaw";

    # Select internationalisation properties.
    i18n.defaultLocale = "en_US.UTF-8";

    i18n.extraLocaleSettings = {
        LC_ADDRESS = "pl_PL.UTF-8";
        LC_IDENTIFICATION = "pl_PL.UTF-8";
        LC_MEASUREMENT = "pl_PL.UTF-8";
        LC_MONETARY = "pl_PL.UTF-8";
        LC_NAME = "pl_PL.UTF-8";
        LC_NUMERIC = "pl_PL.UTF-8";
        LC_PAPER = "pl_PL.UTF-8";
        LC_TELEPHONE = "pl_PL.UTF-8";
        LC_TIME = "pl_PL.UTF-8";
    };

    # Configure keymap in X11
    services.xserver = {
        enable = true;
        xkb = {
        layout = "pl";
        variant = "";
        };
    };

    # Disable alt+arrow keys switching TTY
    console.keyMap = pkgs.runCommand "patched-keymap" {} ''
      ${pkgs.ckbcomp}/bin/ckbcomp -layout pl > keymap

      sed -e 's/Decr_Console/VoidSymbol/g' \
          -e 's/Incr_Console/VoidSymbol/g' \
          keymap > $out
    '';

    # Enable CUPS to print documents.
    services.printing.enable = true;

    # Define a user account. Don't forget to set a password with ‘passwd’.
    users = {
        defaultUserShell = pkgs.zsh;
        users = {
                tym2k1 = {
                    isNormalUser = true;
                    description = "Tymoteusz Burak";
                    extraGroups = [ "networkmanager" "wheel" "docker" "dialout"];
                    initialPassword = "test";
                };
        };
    };

    # Docker
    virtualisation.docker.enable = true;

    programs.zsh.enable = true;

    # Allow unfree packages
    nixpkgs.config.allowUnfree = true;

    nix.settings.trusted-users = [ "tym2k1" "@wheel" ];

    hardware.graphics.enable = true;
    hardware.nvidia.prime = {
        offload = {
            enable = true;
            enableOffloadCmd = true;
        };
        intelBusId = "PCI:0:2:0";
        nvidiaBusId = "PCI:1:0:0";
    };

    specialisation = {
        nvidia-sync-mode.configuration = {
            hardware.nvidia = {
                prime.sync.enable = pkgs.lib.mkForce true;
                prime.offload = {
                    enable = pkgs.lib.mkForce false;
                    enableOffloadCmd = pkgs.lib.mkForce false;
                };
            };
        };
    };

    services.pcscd.enable = true;
    programs.gnupg.agent = {
        pinentryPackage = pkgs.pinentry-curses;
        enable = true;
        enableSSHSupport = true;
    };

  # Required for a shitty lab at uni
  services.udev.extraRules = ''
    SUBSYSTEM=="tty", ATTRS{idVendor}=="0403", ATTRS{idProduct}=="6001", ATTRS{serial}=="FTHJPYW1", SYMLINK+="VMC"
    SUBSYSTEM=="tty", ATTRS{idVendor}=="0403", ATTRS{idProduct}=="6001", ATTRS{serial}=="FTHJRKHP", SYMLINK+="xsens"
    SUBSYSTEM=="tty", ATTRS{idVendor}=="0403", ATTRS{idProduct}=="6001", ATTRS{serial}=="FTGSEMAV", SYMLINK+="VMC"

    SUBSYSTEM=="usb", ATTR{idVendor}=="0403", ATTR{idProduct}=="a8b0", ATTR{serial}=="662080015707", SYMLINK+="EPOS2R", GROUP="users", MODE="0666"
    SUBSYSTEM=="usb", ATTR{idVendor}=="0403", ATTR{idProduct}=="a8b0", ATTR{serial}=="662080015698", SYMLINK+="EPOS2L", GROUP="users", MODE="0666"

    SUBSYSTEM=="usb", ATTR{idVendor}=="24e7", ATTR{idProduct}=="3b01", SYMLINK+="EPOS4", GROUP="users", MODE="0666"

    # ftdi rule for EPOS4 70/15
    SUBSYSTEMS=="usb", ATTR{idVendor}=="0403", ATTR{idProduct}=="a8b0", GROUP="users", MODE="0666"
  '';

  };

}
