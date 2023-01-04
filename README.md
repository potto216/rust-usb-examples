# Examples using Rust to access USB devices
## Overview
This code demonstrates how to communicate with USB devices using Rust on Linux. The files are intendeded to be a starting point for others wanting to experiment with USB. USB access uses [rusb]. Command line argument parsing is done with [bpaf].

## Setup
The code is tested on VM running: 
Linux 5.15.0-53-generic #59-Ubuntu SMP Mon Oct 17 18:53:30 UTC 2022 x86_64 x86_64 x86_64 GNU/Linux

### General Setup
The setup requires libusb development packages which can be installed with
`sudo apt install libusb-1.0-0-dev libusb-1.0-0`

### Accessing Bluetooth Controllers
Linux commmunicates with Bluetooth controllers using the BlueZ stack. This stack prevents the direct USB access of the controllers. Therefore
to communicate with Bluetooth controllers directly using [rusb] we must disable BlueZ.

*If you wish to communicate with the Bluetooth controller while keeping the BlueZ stack enabled you can use the BlueZ `HCI_CHANNEL_USER` option as demonstarted in the [PayPal GATT repo](github.com/paypal/gatt), however you will not have direct USB access.

First disable the BlueZ drivers
```
sudo sh -c 'echo "install btusb /bin/true" >> /etc/modprobe.d/blacklist.conf'
sudo sh -c 'echo "install bluetooth /bin/true" >> /etc/modprobe.d/blacklist.conf'
sudo sh -c 'echo "install btrtl /bin/true" >> /etc/modprobe.d/blacklist.conf'
sudo sh -c 'echo "install btintel /bin/true" >> /etc/modprobe.d/blacklist.conf'
sudo sh -c 'echo "install btbcm /bin/true" >> /etc/modprobe.d/blacklist.conf'
sudo update-initramfs -u
sudo systemctl disable bluetooth.service
sudo reboot now

# Verify modules are not being loaded
lsmod | grep -i bluetooth
```

Setup the rules to make the USB device read write
```
sudo sh -c 'echo "# Broadcom Corp. BCM20702A0 Bluetooth 4.0" >> /etc/udev/rules.d/bluetooth-controller.rules'
sudo sh -c 'echo "SUBSYSTEMS==\"usb\", ATTRS{idVendor}==\"0a5c\", ATTRS{idProduct}==\"21e8\", MODE:=\"0666\"" >> /etc/udev/rules.d/bluetooth-controller.rules'

cat /etc/udev/rules.d/bluetooth-controller.rules
# Broadcom Corp. BCM20702A0 Bluetooth 4.0
SUBSYSTEMS=="usb", ATTRS{idVendor}=="0a5c", ATTRS{idProduct}=="21e8", MODE:="0666"

sudo udevadm control --reload-rules && sudo udevadm trigger
```

### Accessing a Human Interface Device (HID)
In Linux HIDs are owned by the usbhid driver. Therefore to access it the driver must be disabled. WARNING! Disabling the usbhid driver will disable the USB keyboard and mouse connected to your Linux system. Therefore you must remote into the Linux system using SSH or something similar after disabling usbhid.

### If using VMware 
If using VMWare the HID devices are not available as individual devices and instead are wrapped in a virtual driver. To show the individual devices
1. Shutdown the VM
2. Go to VM->Settings->"USB Controller" and check "Show all USB input devices"
3. Start VM and log in.
You can now connect the individual keyboards and mice and they will show up when running `lsusb`

### Disable the HID driver
```
# Add the line `blacklist usbhid` to `/etc/modprobe.d/usbhid.conf`
sudo sh -c 'echo "blacklist usbhid" >> /etc/modprobe.d/usbhid.conf'

cat /etc/modprobe.d/usbhid.conf
blacklist usbhid

sudo update-initramfs -u
sudo reboot now

sudo sh -c 'echo "# MosArt Semiconductor Corp. Wireless Mouse'
sudo sh -c 'echo "SUBSYSTEMS==\"usb\", ATTRS{idVendor}==\"062a\", ATTRS{idProduct}==\"4102\", MODE:=\"0666\"" >> /etc/udev/rules.d/bluetooth-controller.rules'

cat /etc/udev/rules.d/mouse-hid.rules
# MosArt Semiconductor Corp. Wireless Mouse
SUBSYSTEMS=="usb", ATTRS{idVendor}=="062a", ATTRS{idProduct}=="4102", MODE:="0666"

sudo udevadm control --reload-rules && sudo udevadm trigger
```

## Building and Running the Examples

Clone and build the examples
```
git clone https://github.com/potto216/rust-usb-examples.git
cd rust-usb-examples
cargo build  --bin bluetooth_le_controller
cargo build  --bin mouse_hid
```
Now find the USB devices and test them 

```
lsusb
Bus 002 Device 004: ID 0a5c:21e8 Broadcom Corp. BCM20702A0 Bluetooth 4.0
Bus 002 Device 006: ID 062a:4102 MosArt Semiconductor Corp. Wireless Mouse

cargo run  --bin bluetooth_le_controller -- -d 0a5c:21e8 -c hci_read_bd_addr -v
cargo run  --bin mouse_hid -- -d 062a:4102 -v
```
## Debugging  Tips
To see the maximum amount of libusb information set `export LIBUSB_DEBUG=4`. This information is printed over standard error (stderr).
To remove libusb debug information remove the environment variable with `unset LIBUSB_DEBUG`

To dump USB traffic on Linux you first need the usbmon kernel module. 
```
# Check if it is running
lsmod | grep usb

#if not running, check if you have it in the kernel
modinfo usbmon

# If you have it and not loaded, then load it`
sudo modprobe usbmon
```

Either install Wireshark, tshark (command line Wireshark) or use tcpdump. if using tcpdump to capture USB in PCAP format you may need to run `sudo apt install libdbus-1-dev libpcap-dev libpcap0.8-dev`

```
# list the USB bus capture options
tcpdump -D
# Then choose the USB bus that your devices is on which you can find out from lsusb and capture
tcpdump -i usbmon2 -w usbmouse.pcap 
```

[rusb]: https://github.com/a1ien/rusb
[bpaf]: https://github.com/pacak/bpaf
