// run with cargo run --bin bluetooth_le_controller

// setup with sudo apt install libusb-1.0-0-dev libusb-1.0-0
//  sudo udevadm control --reload-rules && sudo udevadm trigger
// Ref
//https://github.com/a1ien/rusb/blob/master/examples/read_device.rs
//https://github.com/pacak/bpaf/blob/master/examples/basic.rs
//  cargo run  --bin bluetooth_le_controller -- -d abc
//  cargo build  --bin bluetooth_le_controller


# First remove the BlueZ drivers
sudo sh -c 'echo "install btusb /bin/true" >> /etc/modprobe.d/blacklist.conf'
sudo sh -c 'echo "install bluetooth /bin/true" >> /etc/modprobe.d/blacklist.conf'
sudo sh -c 'echo "install btrtl /bin/true" >> /etc/modprobe.d/blacklist.conf'
sudo sh -c 'echo "install btintel /bin/true" >> /etc/modprobe.d/blacklist.conf'
sudo sh -c 'echo "install btbcm /bin/true" >> /etc/modprobe.d/blacklist.conf'
sudo update-initramfs -u

sudo systemctl disable bluetooth.service

# setup the 
/etc/udev/rules.d
sudo vi /etc/udev/rules.d/bluetooth-controller.rules

cat /etc/udev/rules.d/bluetooth-controller.rules
# Actions general adapter
SUBSYSTEMS=="usb", ATTRS{idVendor}=="10d7", ATTRS{idProduct}=="b012", MODE:="0666"

sudo udevadm control --reload-rules && sudo udevadm trigger

sudo reboot now


# check with
lsmod | grep -i bluetooth
export LIBUSB_DEBUG=4
cd rust-usb-examples
cargo build --bin bluetooth_le_controller
cargo run  --bin bluetooth_le_controller -- -d abc