#![no_main]
#![no_std]

use uefi::boot::check_event;
use uefi::prelude::*;
use uefi::println;

#[entry]
fn main() -> Status {
    uefi::helpers::init().expect("Failed to init UEFI");

    println!("Realtime mode Input Integration Test");

    #[cfg(feature = "alloc")]
    uefi_input2::with_stdins(|stdins| {
        use uefi::proto::console::text::Key::{Printable, Special};
        use uefi::proto::console::text::ScanCode;
        use uefi_input2::key_data::KeyData;
        use core::hint::spin_loop;
        use uefi::print;

        for keyboard in stdins.iter_mut() {
            match KeyData::realtime_init(keyboard, true) {
                Ok(_) => println!("Set state successful (realtime mode should be on)."),
                Err(status) => println!("Failed to set state: {:?}", status),
            }
        }

        loop {
            spin_loop();

            for keyboard in stdins.iter_mut() {
                if let Some(event) = keyboard.wait_for_key_event() {
                    if check_event(&event)? {
                        if let Some(key_data) = keyboard.read_key_stroke_ex() {
                            if key_data.supports_modifiers() && key_data.ctrl() {
                                print!("[Ctrl] ");
                            }
                            match key_data.key {
                                Printable(c) if u16::from(c) >= 0x20 => print!("{}", c),
                                Special(code) if code == ScanCode::ESCAPE => {
                                    println!("Exiting...");
                                    return Ok(());
                                },
                                _ => {}
                            }
                        }
                    }
                }
            }
        }
    }).expect("Failed to get stdins");


    Status::SUCCESS
}
