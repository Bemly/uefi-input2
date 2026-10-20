#![no_main]
#![no_std]

use uefi::boot::{check_event, ScopedProtocol};
use uefi::{print, println, Result};
use uefi::prelude::*;
use uefi::proto::console::text::Key::Printable;
use uefi_input2::hotplug::KeyboardHotPlugMonitor;
use uefi_input2::input::Input;
use uefi_input2::key_data::KeyData;

#[entry]
fn main() -> Status {

    // executes only once during initialization
    fn on_init(index: usize, input: &mut ScopedProtocol<Input>) -> Result {
        KeyData::realtime_init(input, true)?;
        Ok(println!("[Keyboard {} init] {:?}", index, input))
    }

    // this callback function will be executed repeatedly.
    fn on_event(index: usize, input: &mut ScopedProtocol<Input>) -> Result {
        // could select specified input device.
        if index == 0 {
            KeyData::realtime_init(input, true)?;
        }

        let Some(event) = input.wait_for_key_event() else { return Ok(()) };
        if !check_event(&event)? { return Ok(()) }

        match input.read_key_stroke_ex() {
            Some(KeyData { key: Printable(c), .. }) => {
                match u16::from(c) {
                    0x0D => Ok(print!("\r\n")),
                    0x20.. => Ok(print!("[{}]{} ", index, c)),
                    _ => Ok(()),
                }
            },
            _ => Ok(()),
        }
    }

    // it will execute when the keyboard is plugged in.
    fn on_update(index: usize, input: &mut ScopedProtocol<Input>) -> Result {
        KeyData::realtime_init(input, true)?;
        Ok(println!("[Keyboard {} update] {:?}", index, input))
    }

    unsafe {
        KeyboardHotPlugMonitor::notify_mode_loop(on_init, on_update, on_event)
            .expect("Failed to start keyboard hotplug monitor");
    }

    Status::SUCCESS
}
