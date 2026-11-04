{ self, inputs, ... }: {
  flake.nixosModules.NUC_D54250Hardware = { config, lib, pkgs, modulesPath, ... }:
    {
      imports =
        [ (modulesPath + "/installer/scan/not-detected.nix")
        ];

      boot.initrd.availableKernelModules = [ "xhci_pci" "ehci_pci" "usb_storage" "usbhid" "sd_mod" ];
      boot.initrd.kernelModules = [ ];
      boot.kernelModules = [ "kvm-intel" ];
      boot.extraModulePackages = [ ];

      fileSystems."/" =
        { device = "/dev/disk/by-uuid/b66a83ef-cb6e-4a75-9e75-a8bc6eebb455";

          fsType = "ext4";
        };

      boot.initrd.luks.devices."luks-65d83d20-a240-415c-a973-926ed7d76e96".device = "/dev/disk/by-uuid/65d83d20-a240-415c-a973-926ed7d76e96";

      fileSystems."/boot" =
        { device = "/dev/disk/by-uuid/538C-4692";
          fsType = "vfat";
          options = [ "fmask=0077" "dmask=0077" ];
        };

        swapDevices =
          [ { device = "/dev/disk/by-uuid/274e37dc-032c-46e1-9913-d3ca5a5863f4"; }
        ];

      nixpkgs.hostPlatform = lib.mkDefault "x86_64-linux";
      hardware.cpu.intel.updateMicrocode = lib.mkDefault config.hardware.enableRedistributableFirmware;
    };
}
