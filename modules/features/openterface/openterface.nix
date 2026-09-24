{ self, inputs, ... }: {
  flake.nixosModules.openterface = { pkgs, lib, ... }: {
    # The CH340/CH341 driver is in the Linux kernel.
    boot.kernelModules = [ "ch341" ];

    environment.systemPackages = with pkgs; [
        openterface-qt
        v4l-utils
    ];


    # Give the logged-in desktop user access to the KVM's
    # USB/HID/serial/video interfaces.
    services.udev.extraRules = ''
      # Openterface / MacroSilicon HDMI capture
      SUBSYSTEM=="usb", ATTRS{idVendor}=="534d", ATTRS{idProduct}=="2109", TAG+="uaccess"
      SUBSYSTEM=="usb", ATTRS{idVendor}=="345f", ATTRS{idProduct}=="2109", TAG+="uaccess"
      SUBSYSTEM=="usb", ATTRS{idVendor}=="345f", ATTRS{idProduct}=="2132", TAG+="uaccess"

      # HID interfaces
      SUBSYSTEM=="hidraw", ATTRS{idVendor}=="534d", ATTRS{idProduct}=="2109", MODE="0660", GROUP="users"
      SUBSYSTEM=="hidraw", ATTRS{idVendor}=="345f", ATTRS{idProduct}=="2109", MODE="0660", GROUP="users"
      SUBSYSTEM=="hidraw", ATTRS{idVendor}=="345f", ATTRS{idProduct}=="2132", MODE="0660", GROUP="users"

      # CH340/CH341 serial interfaces
      SUBSYSTEM=="ttyUSB", ATTRS{idVendor}=="1a86", ATTRS{idProduct}=="7523", MODE="0660", GROUP="dialout"
      SUBSYSTEM=="usb", ATTRS{idVendor}=="1a86", ATTRS{idProduct}=="7523", TAG+="uaccess"

      SUBSYSTEM=="ttyACM", ATTRS{idVendor}=="1a86", ATTRS{idProduct}=="fe0c", MODE="0660", GROUP="dialout"
      SUBSYSTEM=="usb", ATTRS{idVendor}=="1a86", ATTRS{idProduct}=="fe0c", TAG+="uaccess"

      # Openterface Mini-KVM HID interface
      SUBSYSTEM=="hidraw", ATTRS{idVendor}=="0c45", ATTRS{idProduct}=="7e0c", TAG+="uaccess", MODE="0660"
    '';

  };
}
