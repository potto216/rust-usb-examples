use bpaf::*;
use std::path::PathBuf;
use std::time::Duration;

use rusb::{
    Context, Device, DeviceDescriptor, DeviceHandle, Direction, Result, RequestType, UsbContext, Recipient
};


#[allow(dead_code)]
#[derive(Debug, Clone)]
struct Out {
    device: String,
    verbose: usize,
}

fn opts() -> OptionParser<Out> {
    // A flag, true if used in the command line. Can be required, this one is optional

    let device = short('d') // start with a short name
        .long("device") // also add a long name
        .help("[vendor]:[product] Show only devices with the specified vendor and product ID.  Both IDs are given in hexadecimal.")
        .argument::<String>("Device"); // and a help message to use

    // number of occurrences of the v/verbose flag capped at 3 with an error here but you can also
    // use `max` inside `map`
    let verbose = short('v')
        .long("verbose")
        .help("Increase the verbosity\nYou can specify it up to 3 times\neither as -v -v -v or as -vvv")
        .req_flag(())
        .many()
        .map(|xs| xs.len())
        .guard(|&x| x <= 3, "It doesn't get any more verbose than this");


    // packing things in a struct assumes parser for each field is in scope.
    construct!(Out {
        device,
        verbose,
    })
    .to_options()
    .descr("This is a description")
}

fn main() {
    let opts = opts().run();
    println!("{:#?}", opts);
    println!("Hello, bluetooth_le_controller v0.0.1");

    let vid = 0x10d7;
    let pid = 0xb012; 

    match Context::new() {
        Ok(mut context) => match open_device(&mut context, vid, pid) {
            Some((mut device, device_desc, mut handle)) => {
                read_device(&mut device, &device_desc, &mut handle).unwrap()
            }
            None => println!("could not find device {:04x}:{:04x}", vid, pid),
        },
        Err(e) => panic!("could not initialize libusb: {}", e),
    }



}

fn open_device<T: UsbContext>(
    context: &mut T,
    vid: u16,
    pid: u16,
) -> Option<(Device<T>, DeviceDescriptor, DeviceHandle<T>)> {
    let devices = match context.devices() {
        Ok(d) => d,
        Err(_) => return None,
    };

    for device in devices.iter() {
        let device_desc = match device.device_descriptor() {
            Ok(d) => d,
            Err(_) => continue,
        };

        if device_desc.vendor_id() == vid && device_desc.product_id() == pid {
            match device.open() {
                Ok(handle) => return Some((device, device_desc, handle)),
                Err(e) => panic!("Device found but failed to open: {}", e),
            }
        }
    }



    None
}

fn read_device<T: UsbContext>(
    device: &mut Device<T>,
    device_desc: &DeviceDescriptor,
    handle: &mut DeviceHandle<T>,
) -> Result<()> {
    handle.reset()?;

    let timeout = Duration::from_secs(1);
    let languages = handle.read_languages(timeout)?;

    println!("Active configuration: {}", handle.active_configuration()?);
    println!("Languages: {:?}", languages);

    if !languages.is_empty() {
        let language = languages[0];

        println!(
            "Manufacturer: {:?}",
            handle
                .read_manufacturer_string(language, device_desc, timeout)
                .ok()
        );
        println!(
            "Product: {:?}",
            handle
                .read_product_string(language, device_desc, timeout)
                .ok()
        );
        println!(
            "Serial Number: {:?}",
            handle
                .read_serial_number_string(language, device_desc, timeout)
                .ok()
        );
    }

    handle.set_active_configuration(1)?;
    // OGF/OCF pg 1810
    //For the HCI Control and Baseband commands, the OGF is defined as 0x03.
    //HCI_Reset OCF = 0x0003 Status pg 1972
    // Opcode MSB LSB Param Length = 0
    // OGF | OGC
    // 0000 11 | 00 0000 0011 = 0x0C03
    let mut hci_cmd_buf = [0x0C,0x03, 0x00];
    let timeout_cmd = Duration::from_secs(1);
    println!("Writing to control");

    match handle.write_control( 0x20,
        rusb::request_type(Direction::Out, RequestType::Standard, Recipient::Endpoint),
         0x00,
         0x00,
         &mut hci_cmd_buf,
         timeout_cmd) {
        Ok(len) => {
            println!(" - sent: {:?} bytes", len);
        }
        Err(err) => println!("could not read from endpoint: {}", err),
    }

    handle.claim_interface(0);
    // bEndpointAddress     0x81  EP 1 IN ; Transfer Type            Interrupt

    let mut buf = [0; 255];
    let timeout = Duration::from_secs(1);
    let endpoint_address = 0x81;

    println!("Reading event from interrupt");
    match handle.read_interrupt(endpoint_address, &mut buf, timeout) {
        Ok(len) => {
            println!(" - read: {:?}", &buf[..len]);
        }
        Err(err) => println!("could not read from endpoint: {}", err),
    }
        

    handle.release_interface(0);


    Ok(())
}
