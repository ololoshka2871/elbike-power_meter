use std::{ops::DerefMut, str::from_utf8, time::Duration};

use anyhow::Result;
use serialport::SerialPort;

fn main() -> Result<()> {
    let mut port = serialport::new("/dev/ttyUSB0", 115200)
        .timeout(Duration::from_millis(100))
        .open()?;

    /*
    // Clone the port
    let mut port_clone = port.try_clone()?;

    // Send out 4 bytes every second
    thread::spawn(move || loop {
        port_clone
            .write_all(&[5, 6, 7, 8])
            .expect("Failed to write to serial port");
        thread::sleep(Duration::from_millis(1000));
    });
    */

    /*
    // Read the four bytes back from the cloned port
    let mut buffer: [u8; 1] = [0; 1];
    loop {
        match port.read(&mut buffer) {
            Ok(bytes) => {
                if bytes == 1 {
                    println!("Received: {:?}", buffer);
                }
            }
            Err(ref e) if e.kind() == io::ErrorKind::TimedOut => (),
            Err(e) => eprintln!("{:?}", e),
        }
    }
    */
    test_simple(port.deref_mut())?;

    draw_sinus(port.deref_mut())?;

    Ok(())
}

fn test_simple(port: &mut dyn SerialPort) -> Result<()> {
    const SRC: [u8; 12] = [0x41, 0, 0, 0, 0, 0, 0, 0, 42, 0, 0, 0];
    port.write_all(&SRC)?;
    port.flush()?;

    std::thread::sleep(Duration::from_millis(1000));

    let mut dest = [0u8; 65535];
    let read = port.read(&mut dest)?;

    print!("Res: {}", from_utf8(&dest[..read])?);

    Ok(())
}

fn draw_sinus(port: &mut dyn SerialPort) -> Result<()> {
    let mut template: [u8; 12] 
    				    = [0x41, 0, 0, 0, 0, 0, 0, 0, b'p', 0, 0, 0];

    for i in 0..128 {
        template[8] = (92.0 * (2.0 * std::f32::consts::PI * i as f32 / 128.0).sin().abs()) as u8;

        port.write_all(&template)?;
        port.flush()?;

        std::thread::sleep(Duration::from_millis(85));

	let mut dest = [0u8; 65535];
    	let read = port.read(&mut dest)?;

	print!("Res: {}", from_utf8(&dest[..read])?);
    }

    Ok(())
}
