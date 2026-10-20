#![no_main]
#![no_std]

use uefi::boot::{check_event, ScopedProtocol};
use uefi::{print, println, Result};
use uefi::prelude::*;
use uefi::proto::console::text::Key::Printable;
use uefi_input2::hotplug::KeyboardHotPlugMonitor;
use uefi_input2::input::Input;
use uefi_input2::key_data::KeyData;
use uefi_input2::config::REFRESH_POSITIVE_INPUT_DEVICE_TIME;

#[entry]
fn main() -> Status {
    fn on_init(index: usize, input: &mut ScopedProtocol<Input>) -> Result {
        KeyData::realtime_init(input, true)?;
        Ok(println!("[Keyboard {} init] {:?}", index, input))
    }

    fn on_main_loop(index: usize, input: &mut ScopedProtocol<Input>) -> Result {
        // could select specified input device.
        if index == 0 {
            KeyData::realtime_init(input, true)?;
        }

        let Some(event) = input.wait_for_key_event() else { return Ok(()) };
        if !check_event(&event)? { return Ok(()) }

        match input.read_key_stroke_ex() {
            Some(KeyData { key: Printable(c), .. }) => {
                match u16::from(c) {
                    0x20.. => Ok(print!("[{}]{} ", index, c)),
                    _ => Ok(()),
                }
            },
            _ => Ok(()),
        }
    }

    fn on_update(index: usize, input: &mut ScopedProtocol<Input>) -> Result {
        KeyData::realtime_init(input, true)?;
        Ok(println!("[Keyboard {} update] {:?}", index, input))
    }

    // The delay can be modified before polling mode loop.
    REFRESH_POSITIVE_INPUT_DEVICE_TIME::set(1_0000_0000);

    unsafe {
        KeyboardHotPlugMonitor::polling_mode_loop(on_init, on_update, on_main_loop)
            .expect("Failed to start keyboard hotplug monitor");
    }

    Status::SUCCESS
}
