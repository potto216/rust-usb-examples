use std::time::Duration;
use std::str::FromStr;
use std::u16;

use bpaf::{Parser, short, Bpaf};

use rusb::{
    Context, Device, DeviceDescriptor, DeviceHandle, Direction, Recipient, RequestType, Result,
    UsbContext,
};

#[derive(Debug, Clone)]
enum HciCommand {
    HciReset,
    HciReadBdAddr
}
    
    // For the HCI Control and Baseband commands, the OGF is defined as 0x03.
    // HCI_Reset OCF = 0x0003 Status pg 1972
    // Opcode MSB LSB Param Length = 0
    // OGF | OCF
    // 0000 11 | 00 0000 0011 = 0x0C03
    static  HCI_RESET_CMD_VALUES: [u8;3] = [0x03, 0x0C, 0x00];

    // For Informational Parameters commands, the OGF is defined as 0x04.
    // HCI_Read_BD_ADDR OCF = 0x0009 pg 2122
    // Opcode MSB LSB Param Length = 0
    // OGF | OCF
    // 0001 00 | 00 0000 1001 = 0x1009
    //[0x03, 0x10, 0x00];
    static  HCI_READ_BD_ADDR_CMD_VALUES: [u8;3] = [0x09, 0x10, 0x00];

impl HciCommand 
{
    fn value(&self) -> &'static [u8] {
        match *self {
            HciCommand::HciReset => &HCI_RESET_CMD_VALUES,
            HciCommand::HciReadBdAddr => &HCI_READ_BD_ADDR_CMD_VALUES,
        }                
    } 


}

impl FromStr for HciCommand {
    type Err = String;
    fn from_str(s: &str) -> std::result::Result<Self, String>
    where
        Self: Sized,
    {
        match s {
            "hci_reset" => Ok(HciCommand::HciReset),
            "hci_read_bd_addr" => Ok(HciCommand::HciReadBdAddr),
             _ => Err("Expected hci_reset|hci_read_bd_addr".to_string()),
        }
    }
}




fn verbose() -> impl Parser<usize> {
    short('v')
        .long("verbose")
        .help("Increase output verbosity, can be used several times")
        .req_flag(())
        .many()
        .map(|v| v.len())
}

/*
fn reset() -> impl Parser<bool> {
short('r') 
.long("reset") 
.help("Reset the USB device") 
.switch()
}
*/

fn command() -> impl Parser<HciCommand> {    
    short('c')
    .long("command")
    .help("choose between hci_reset or hci_read_bd_addr")
    .argument::<HciCommand>("CMD")
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

    #[bpaf(external)]
    command:HciCommand, 

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
    let command = opts.command;

    println!("bluetooth_le_controller v0.0.1 will open device {}:{}", vid, pid);

    match Context::new() {
        Ok(mut context) => match open_device(&mut context, vid, pid) {
            Some((mut device, device_desc, mut handle)) => {
                read_device(&mut device, &device_desc, &mut handle, command, reset_device).unwrap()
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
    command: HciCommand,
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

    let timeout_cmd = Duration::from_secs(1);
    let req_t = rusb::request_type(Direction::Out, RequestType::Class, Recipient::Device);  //Should be 0x20
    println!("Writing to control req_t = {}",req_t);


    match handle.write_control( req_t,
         0,        
         0x00,
         0x00,
         &command.value(),
         timeout_cmd) {
        Ok(len) => {
            println!(" - sent: {:?} bytes", len);
        }
        Err(err) => println!("could not read from endpoint: {}", err),
    }  
    
    /*
    println!("Reading from control req_t = {}",req_t);
    match handle.read_control( rusb::request_type(Direction::In, RequestType::Class, Recipient::Device),
        0,        
        0x00,
        0x00,        
        &mut hci_cmd_buf_read,
        timeout_cmd) {
       Ok(len) => {
           println!(" -len {}, read: {:?} bytes",len, &hci_cmd_buf_read[..len]);
       }
       Err(err) => println!("could not read from endpoint: {}", err),
   } 
   */ 
    /*
    match handle.write_control( rusb::request_type(Direction::Out, RequestType::Standard, Recipient::Endpoint),
        0x00,        
         0x00,
         0x00,
         &mut hci_cmd_buf,
         timeout_cmd) {
        Ok(len) => {
            println!(" - sent: {:?} bytes", len);
        }
        Err(err) => println!("could not read from endpoint: {}", err),
    }

    */


    handle.claim_interface(0).unwrap();
       
    
    // bEndpointAddress     0x81  EP 1 IN ; Transfer Type            Interrupt
    // When the reset has been performed, an HCI_Command_Complete event shall be generated.
/*
page 2188
HCI_Command_-
Complete
0x0E Num_HCI_Command_Packets,
Command_Opcode,
Return_Parameters 
Num_HCI_Command_Packets: Size: 1 octet
Command_Opcode: Size: 2 octets
Event Event Code Event Parameters
HCI_Command_-
Complete
0x0E Num_HCI_Command_Packets,
Command_Opcode,
Return_Parameters
Value Parameter Description
0xXX The Number of HCI Command packets which are allowed to be sent to the
Controller from the Host.
Range: 0 to 255
Value Parameter Description
0x0000 No associated command
0xXXXX (non-zero) Opcode of the command which caused this event.
 read: [14, 4, 1, 3, 12, 0]
*/
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
        
    handle.release_interface(0).unwrap();

    Ok(())
}
