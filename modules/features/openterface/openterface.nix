{ self, inputs, ... }: {
  flake.nixosModules.openterface = { pkgs, lib, ... }: {
    # The CH340/CH341 driver is in the Linux kernel.
    boot.kernelModules = [ "ch341" ];

    # Give the logged-in desktop user access to the KVM's
    # USB/HID/serial/video interfaces.
    services.udev.extraRules = ''
      # Openterface / MacroSilicon HDMI capture
      SUBSYSTEM=="usb", ATTRS{idVendor}=="534d", ATTRS{idProduct}=="2109", TAG+="uaccess"'
      SUBSYSTEM=="hidraw", ATTRS{idVendor}=="534d", ATTRS{idProduct}=="2109", TAG+="uaccess"'
      SUBSYSTEM=="usb", ATTRS{idVendor}=="345f", ATTRS{idProduct}=="2109", TAG+="uaccess"'
      SUBSYSTEM=="hidraw", ATTRS{idVendor}=="345f", ATTRS{idProduct}=="2109", TAG+="uaccess"'
      SUBSYSTEM=="usb", ATTRS{idVendor}=="345f", ATTRS{idProduct}=="2132", TAG+="uaccess"'
      SUBSYSTEM=="hidraw", ATTRS{idVendor}=="345f", ATTRS{idProduct}=="2132", TAG+="uaccess"'
      SUBSYSTEM=="ttyUSB", ATTRS{idVendor}=="1a86", ATTRS{idProduct}=="7523", TAG+="uaccess"'
      SUBSYSTEM=="usb", ATTRS{idVendor}=="1a86", ATTRS{idProduct}=="7523", TAG+="uaccess"'
      SUBSYSTEM=="ttyACM", ATTRS{idVendor}=="1a86", ATTRS{idProduct}=="fe0c", TAG+="uaccess"'
      SUBSYSTEM=="usb", ATTRS{idVendor}=="1a86", ATTRS{idProduct}=="fe0c", TAG+="uaccess"'
    '';
  };
}
