These are the instructions to setup the 

// run with cargo run --bin bluetooth_le_controller

// setup with sudo apt install libusb-1.0-0-dev libusb-1.0-0
//  sudo udevadm control --reload-rules && sudo udevadm trigger
// Ref
//https://github.com/a1ien/rusb/blob/master/examples/read_device.rs
//https://github.com/pacak/bpaf/blob/master/examples/basic.rs

cargo build  --bin bluetooth_le_controller
cargo run  --bin bluetooth_le_controller -- -d abc
export LIBUSB_DEBUG=4
unset LIBUSB_DEBUG=4


# First remove the BlueZ drivers
sudo sh -c 'echo "install btusb /bin/true" >> /etc/modprobe.d/blacklist.conf'
sudo sh -c 'echo "install bluetooth /bin/true" >> /etc/modprobe.d/blacklist.conf'
sudo sh -c 'echo "install btrtl /bin/true" >> /etc/modprobe.d/blacklist.conf'
sudo sh -c 'echo "install btintel /bin/true" >> /etc/modprobe.d/blacklist.conf'
sudo sh -c 'echo "install btbcm /bin/true" >> /etc/modprobe.d/blacklist.conf'
sudo update-initramfs -u

sudo systemctl disable bluetooth.service

# Setup the rules to make the USB device read write
/etc/udev/rules.d
sudo vi /etc/udev/rules.d/bluetooth-controller.rules

cat /etc/udev/rules.d/bluetooth-controller.rules
# Broadcom Corp. BCM20702A0 Bluetooth 4.0
SUBSYSTEMS=="usb", ATTRS{idVendor}=="0a5c", ATTRS{idProduct}=="21e8", MODE:="0666"

sudo sh -c 'echo "# Broadcom Corp. BCM20702A0 Bluetooth 4.0" >> /etc/udev/rules.d/bluetooth-controller.rules'
sudo sh -c 'echo "SUBSYSTEMS==\"usb\", ATTRS{idVendor}==\"0a5c\", ATTRS{idProduct}==\"21e8\", MODE:=\"0666\"" >> /etc/udev/rules.d/bluetooth-controller.rules'


sudo udevadm control --reload-rules && sudo udevadm trigger

sudo reboot now


# check with
lsmod | grep -i bluetooth
export LIBUSB_DEBUG=4
cd rust-usb-examples
cargo build --bin bluetooth_le_controller
cargo run  --bin bluetooth_le_controller -- -d abc


# For the mouse
# If using VMware 
Shutdown the VM
Go to VM->Settings->"USB Controller" and check "Show all USB input devices"
Start VM and log in

# disable hid
sudo vi /etc/modprobe.d/usbhid.conf
blacklist usbhid
sudo update-initramfs -u
# Update /etc/udev/rules.d/bluetooth-controller.rules

sudo vi /etc/udev/rules.d/mouse-hid.rules
# MosArt Semiconductor Corp. Wireless Mouse
SUBSYSTEMS=="usb", ATTRS{idVendor}=="062a", ATTRS{idProduct}=="4102", MODE:="0666"

sudo reboot now