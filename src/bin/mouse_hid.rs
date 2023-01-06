use bpaf::*;
use std::time::Duration;

use rusb::{
    Context, Device, DeviceDescriptor, DeviceHandle, Direction, Result, RequestType, UsbContext, Recipient
};


fn verbose() -> impl Parser<usize> {
    short('v')
        .long("verbose")
        .help("Increase output verbosity, can be used several times")
        .req_flag(())
        .many()
        .map(|v| v.len())
}

#[allow(dead_code)]
#[derive(Debug, Clone, Bpaf)]
#[bpaf(options)]
/// Example for bluetooth low energy controller demo
struct Options {
    #[bpaf(short, long, argument::<String>("DEVICE"), parse(parse_device))]
    /// [vendor]:[product] Open a device with the specified vendor and product ID.
    /// Both IDs are given in hexadecimal.
    device: (u16, u16),

    /// Increase verbosity
    #[bpaf(external)]
    verbose: usize,

    #[bpaf(short, long)]
    /// Reset the USB device
    reset: bool,
}

fn parse_device(input: String) -> std::result::Result<(u16, u16), &'static str> {
    let (vid, pid) = input
        .split_once(':')
        .ok_or("Device must be in form of XXXX:YYYY")?;
    let vid = u16::from_str_radix(vid, 16).map_err(|_| "Not a valid VID")?;
    let pid = u16::from_str_radix(pid, 16).map_err(|_| "Not a valid PID")?;
    Ok((vid, pid))
}

fn main() {
    let opts = options().run();
    let verbose = opts.verbose;
    if verbose > 0
    {
        println!("Command line options are: {:#?}", opts);
    }
    
    let (vid, pid) = opts.device;
    let reset_device = opts.reset;

    println!("mouse_hid v0.0.1 will open device {:04x}:{:04x}", vid, pid);

    match Context::new() {
        Ok(mut context) => match open_device(&mut context, vid, pid) {
            Some((mut device, device_desc, mut handle)) => {
                read_device(&mut device, &device_desc, &mut handle, reset_device).unwrap()
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
    _device: &mut Device<T>,
    device_desc: &DeviceDescriptor,
    handle: &mut DeviceHandle<T>,
    reset_device: bool, 
) -> Result<()> {

    if reset_device
    {
        handle.reset()?;
    }

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

    let mut cmd_buf = [0; 255];
    let timeout_cmd = Duration::from_secs(1);
    println!("Writing to control");

    match handle.read_control( rusb::request_type(Direction::In, RequestType::Standard, Recipient::Interface),
         0x06,
         0x2200,
         0x00,
         &mut cmd_buf,
         timeout_cmd) {
        Ok(len) => {
            // see the comments at the end of the code for decoding a similar report description 
            println!(" - received a report descriptor of length: {:?} bytes which equal {:?}", len, &cmd_buf[..len]);
        }
        Err(err) => println!("could not read from endpoint: {}", err),
    }

    handle.claim_interface(0)?;

    // bEndpointAddress     0x81  EP 1 IN ; Transfer Type            Interrupt
    let mut buf = [0; 256];
    let timeout = Duration::from_millis(10);    // This needs to be fast enough to keep up with how often the device is generating reports
    let endpoint_address = 0x81;

    let mut runs=0;
    
    // A value of 3000 ran for about 20 seconds on my VM
    while runs < 3000 
    {
        match handle.read_interrupt(endpoint_address, &mut buf, timeout) {
            Ok(len) => {
                println!(" - read: {:?}", &buf[..len]);
            }
            Err(err) => {println!("Endpoint message: {}", err)}, 
        }

        //  Report Structure
        //        00000000 00111111 11112222 22222233 33333333
        //        01234567 89012345 67890123 45678901 23456789 LSB
        // Button 012XXXXX 
        //                 00000000 0011
        // X dir moved LSB 01234567 8901 MSB
        //                              0000 00000011  
        // Y dir moved LSB              0123 45678901 MSB
        // Wheel                                      01234567

        let buttons:u8 = buf[0];
        if (buttons & 0x01) != 0
        {
            println!("button 1 pressed");     
        }
        if (buttons & 0x02) != 0
        {
            println!("button 2 pressed");     
        }    
        
        if (buttons & 0x04) != 0
        {
            println!("button 3 pressed");     
        }     

        println!("wheel value is {}", (buf[4] as i8));
        
        // This value represents the relative position of how much the mouse has moved
        // Need to assemble this 12 bit value which is split between 1 full byte and a nibble of the next byte into a 16 bit unsigned value
        // so that it can be cast to a 2s complement signed value.        
        let x_moved_unsigned:u16= (((buf[2] & 0x0F) as u16) << 12) | (buf[1] as u16) << 4;        

        // now that it is a signed value can convert it back to 12 bits and if it is negative the sign will be dragged along
        // also turn it back to a 12 bit number
        let x_moved:i16 = (x_moved_unsigned as i16) >> 4; 
        
        // see above comments
        let y_moved_unsigned:u16= ((buf[3] as u16) << 8) | ((buf[2] & 0xF0) as u16);
        let y_moved:i16 = (y_moved_unsigned as i16) >> 4; 

        println!("mouse moved (x,y) = ({},{})", x_moved, y_moved); 
        
        runs = runs + 1;    
    }     

    handle.release_interface(0)?;

    Ok(())
}

/******************
 * Decoding of the report description 
 * Can compare to lsusb -d XXXX:XXXX -v values
Item                  			Value (Hex)
Usage Page (Generic Desktop),   05 01
Usage (Mouse),                  09 02
Collection (Application),       A1 01
  Usage (Pointer),              09 01
  Collection (Physical),        A1 00
    Usage Page (Buttons),       05 09
    Usage Minimum (01),         19 01	
    Usage Maximum (03),         29 03
    Logical Minimum (0),        15 00	
    Logical Maximum (1),        25 01
	Report Size (1 bit),        75 01
    Report Count (3),           95 03    
    Input 
	--(Data, Var, Absolute),	81 02 ;3 button bits
	Report Size (5),			75 05
    Report Count (1),           95 01  
	Input (Constant), 			81 01 ;5 bit padding
	Usage Page 
	--(Generic Desktop),		05 01
	Usage (X),					09 30
	Usage (Y),					09 31
	
	Logical Minimum (2B),		16 [01 F8 ] = -2047
	Logical Maximum (2B),		26 [FF 07 ] =  2047
	
	Report Size (12 bits),		75 0C
	Report Count (2),			95 02
	Input
	--(Data, Var, Relative), 	81 06 ;2 position bytes (X & Y)
	
	Usage (Wheel) 				09 38

	Logical Minimum (1B),		15 [81] = -127
	Logical Maximum (1B),		25 [7F] =  127

	Report Size  (8 bits)		75 08
    Report Count (1)			95 01
    Input
	--(Data, Var, Relative),	81 06
  End Collection,				C0
End Collection					C0
*******************/
