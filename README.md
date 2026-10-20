UEFI Simple Text Input Ex Protocol Wrapper
==============================================
[![CI](https://github.com/Bemly/uefi-input2/workflows/CI/badge.svg)](https://github.com/Bemly/uefi-input2/actions/workflows/ci.yml)
[![Docs.rs](https://docs.rs/uefi-input2/badge.svg)](https://docs.rs/uefi-input2)
[![Crates.io](https://img.shields.io/crates/v/uefi-input2.svg)](https://crates.io/crates/uefi-input2)
[![License: Zed](https://img.shields.io/badge/License-Zed-yellow.svg)](https://spdx.org/licenses/Zed.html)
[![Rust](https://img.shields.io/badge/rust-2024%20edtion-blue.svg)](https://www.rust-lang.org)
[![UEFI](https://img.shields.io/badge/target-*--unknown--uefi-orange.svg)](https://doc.rust-lang.org/nightly/rustc/platform-support/unknown-uefi.html)

This library provides a safe, idiomatic Rust wrapper for the `EFI_SIMPLE_TEXT_INPUT_EX_PROTOCOL`.
Unlike the standard `SimpleTextInput`, this protocol allows for advanced key tracking,
including shift state (Ctrl, Alt, Shift) and toggle state (Caps Lock, Num Lock).

Purpose
----------------------------------------------
- **Seamless Migration**: Designed as a **drop-in, painless replacement**
    for the standard `uefi::system::with_stdin`.
- **Safe Resource Management**: Uses the `with_stdin` pattern to ensure the protocol is opened
    exclusively and closed automatically.
- **Extended Key Data**: Access to `KeyShiftState` and `KeyToggleState`.
- **No-Std Compatible**: Designed specifically for UEFI environments.

Feature List
----------------------------------------------
- **alloc**: Enables `Vec` support. For example, `with_stdins` requires 
    the `alloc` feature to more than 8 multiple input handles via `find_handles`.


Minimal Example
----------------------------------------------
Simply replace your import and use the same closure-based pattern:

```rust,no_run
#![no_main]
#![no_std]
use uefi::prelude::*;
use uefi::{print, println};
use uefi::proto::console::text::Key::{Printable, Special};
use uefi::proto::console::text::ScanCode;
use uefi::boot::check_event;

#[entry]
fn main() -> Status {
    uefi::helpers::init().unwrap();

    uefi_input2::with_stdin(|input| {
        loop {
            // Performance Note: Using wait_for_key_event + check_event conforms to UEFI best practices 
            // by reducing CPU overhead and bus traffic. However, for maximum loop throughput 
            // (e.g., high-frequency GOP rendering), consider calling read_key_stroke_ex directly 
            // to save the extra protocol call.
            let Some(event) = input.wait_for_key_event() else { continue };
            if !check_event(event)? { continue }
            
            if let Some(data) = input.read_key_stroke_ex() {
                if data.shift() { println!("Shift is being held!") }
                match data.key {
                   Printable(c) if u16::from(c) == 0x0D => print!("\r\n"),
                   Printable(c) => print!("{}", c),
                   Special(code) if code == ScanCode::ESCAPE => {
                       println!("Exiting...");
                       return Ok(())
                   },
                   _ => {}
               }
            }
        }
        Ok(())
    }).unwrap();

    Status::SUCCESS
}
```

Manual Build
----------------------------------------------
reference example:
```shell
git clone https://github.com/Bemly/uefi-input2.git
cd uefi-input2
# enable all features on x64 platform
cargo build --release --target x86_64-unknown-uefi --all-features
# enable alloc feature on arm64 platform
cargo build --release --target aarch64-unknown-uefi --features alloc 
# disable default feature on x32 platform for debug mode
cargo build --target i686-unknown-uefi --no-default-features 
```
then use this crate in your project
```toml
[dependencies]
uefi-input2 = { version = "", path = "../uefi-input2" }
```

Test
----------------------------------------------
Due to a reverse-execution bug in the RustRover README runner,
this script is intentionally authored in reverse order for compatibility.
```shell
qemu-system-x86_64 -drive if=pflash,format=raw,file=qemu/OVMF.fd -drive format=raw,file=fat:rw:qemu -m 4G -device usb-ehci -device usb-tablet -smp 4 -cpu max -monitor stdio
mv -Force .\target\x86_64-unknown-uefi\release\examples\*.efi .\qemu\EFI\BOOT\BOOTX64.EFI
rm .\qemu\EFI\BOOT\BOOTX64.EFI
cargo build --example test_polling_hotplug --all-features --release
```

About Version
----------------------------------------------
`41.yy.mm` (e.g. `41.26.10`): major tracks the `uefi` crate generation
it builds against (`41` == `uefi 0.41 / uefi-raw 0.17`); `yy.mm` is the
release date. Also monotonic over the legacy `36.1.x` line (`41 > 36`).
Same-month re-releases have no number left — cut them next month.

License
----------------------------------------------
This project is licensed under the Zed License.