use bpaf::*;
use std::time::Duration;

use rusb::{
    Context, Device, DeviceDescriptor, DeviceHandle, Direction, Result, RequestType, UsbContext, Recipient
};


#[derive(Debug)]
#[repr(C)]
struct WaveFile {
    /// Marks the file as riff file, value should be: "RIFF"
    buttons: [u8; 4],
    /// Size of the overall file
    file_size: u32,
    /// File Type Header. Always should equal "WAVE"
    wave: [u8; 4],
    /// Format chunk marker, sample value: "fmt" with null termination character
    format: [u8; 4],
    /// Length of format data (should be 16 bytes as above)
    format_len: u32,
    /// 1 - PCM, 3 - IEEE float, 6 - 8bit A law, 7 - 8bit mu law
    format_type: [u8; 2],
    /// Number of Channels
    num_of_channels: [u8; 2],
    /// Sample Rate in Hz
    sample_rate: [u8; 4],
    /// bytes per second of recording (Sample Rate * BitsPerSample * Channels) / 8
    byte_rate: [u8; 4],
    /// 1 - 8bit mono, 2 - 8bit stereo / 16bit mono, 4 - 16bit stereo
    audio_type: [u8; 2],
    /// Bits per sample
    bits_per_sample: [u8; 2],
    /// Sample value: “data” - data chunk header. Marks the beginning of the data section.
    data: [u8; 4],
    /// Size of data section
    size_of_data: [u8; 4],
}


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
    device: &mut Device<T>,
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
            println!(" - received a report descriptor of length: {:?} bytes which equal {:?}", len, &cmd_buf[..len]);
        }
        Err(err) => println!("could not read from endpoint: {}", err),
    }

    handle.claim_interface(0)?;

    // bEndpointAddress     0x81  EP 1 IN ; Transfer Type            Interrupt
    let mut buf = [0; 256];
    let timeout = Duration::from_secs(1);
    let endpoint_address = 0x81;

    let mut runs=0;
    while runs < 40
    {
        println!("Reading event from interrupt");
        match handle.read_interrupt(endpoint_address, &mut buf, timeout) {
            Ok(len) => {
                println!(" - read: {:?}", &buf[..len]);
            }
            Err(err) => println!("Endpoint message: {}", err),        
        }
        println!(" - read bytes: {:?}", &buf[..5]);
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
        // the bottom half
        // 00000000 0012
        // 01234567 8901 MSB

        let xpos_unsigned:u16= (((buf[2] & 0x0F) as u16) << 12) | (buf[1] as u16) << 4;
        //let xpos_unsigned:u16= ((buf[1] as u16) << 8) | ((buf[2] & 0xF0) as u16) << 4;
        let xpos:i16 = xpos_unsigned as i16; 

        let ypos_unsigned:u16= ((buf[3] as u16) << 8) | ((buf[2] & 0xF0) as u16);
        let ypos:i16 = ypos_unsigned as i16; 
        println!("x,y = ({},{})", (xpos >> 4), (ypos >> 4)); 
        println!("wheel = {}", buf[4]);



        runs = runs + 1;

    
    }

        

    handle.release_interface(0)?;


    Ok(())
}
