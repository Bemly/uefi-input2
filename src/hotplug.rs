// Copyright (c) Bemly, January 2026
// You may copy and distribute this file freely.  Any queries and
// complaints should be forwarded to bemly_@petalmail.com.
// If you make any changes to this file, please do not distribute
// the results under the name `bemly'.

use alloc::vec::Vec;
use core::ffi::c_void;
use core::hint::spin_loop;
use core::ptr::NonNull;
use core::time::Duration;
use uefi::boot::{check_event, create_event, locate_handle_buffer, open_protocol_exclusive, register_protocol_notify, set_timer, ScopedProtocol, SearchType, TimerTrigger};
use uefi::{Event, Identify, Result};
use uefi_raw::table::boot::{EventType, Tpl};
use crate::config::REFRESH_POSITIVE_INPUT_DEVICE_TIME;
use crate::input::Input;

/// input event notification wrapper
#[allow(unused)]
pub struct KeyboardHotPlugMonitor {
    event: Event,
    key: SearchType<'static>,
}

extern "efiapi" fn on_keyboard_hotplug(_event: Event, ctx: Option<NonNull<c_void>>) {
    if let Some(ctx_ptr) = ctx {
        let flag = ctx_ptr.as_ptr() as *mut bool;
        unsafe { *flag = true; }
    }
}

/// Warn: Unpredictable! Unsafe!
/// This may cause the program to hang.
impl KeyboardHotPlugMonitor {
    /// enable global keyboard hotplug listener
    /// dirty_flag_ptr: recommend static bool pointer
    pub unsafe fn new(dirty_flag_ptr: NonNull<bool>) -> Result<Self> {
        let ctx = dirty_flag_ptr.cast::<c_void>();

        let event = unsafe {
            create_event(
                EventType::NOTIFY_SIGNAL,
                Tpl::CALLBACK,
                Some(on_keyboard_hotplug),
                Some(ctx),
            )?
        };

        let key = register_protocol_notify(
            &Input::GUID,
            &event,
        )?;

        Ok(Self { event, key })
    }

    /// Clears the keyboard list, effectively dropping all `ScopedProtocol` instances.
    ///
    /// This triggers the RAII cleanup logic: each `ScopedProtocol` will automatically
    /// call the underlying UEFI `close_protocol` service upon being dropped. This is
    /// essential to release exclusive hardware locks before a re-scan.
    fn refresh_negative(existing_keyboards: &mut Vec<ScopedProtocol<Input>>) {
        // Brute-force cleanup: Discard all ScopedProtocols without hesitation.
        existing_keyboards.clear();

        let Ok(handles) = locate_handle_buffer(SearchType::ByProtocol(&Input::GUID)) else { return; };

        for handle in handles.iter() {
            if let Ok(keyboard) = open_protocol_exclusive::<Input>(*handle) {
                existing_keyboards.push(keyboard);
            }
        }
    }

    /// Scan for all handles
    fn refresh_positive(existing_keyboards: &mut Vec<ScopedProtocol<Input>>) {
        existing_keyboards.clear();

        let Ok(handles) = locate_handle_buffer(SearchType::AllHandles) else { return; };

        for handle in handles.iter() {
            // Some handles are system ConIn, and some are physical USB; try them all.
            if let Ok(keyboard) = open_protocol_exclusive::<Input>(*handle) {
                existing_keyboards.push(keyboard);
            }
        }
    }

    /// support keyboard hotplug (notify + polling flag mode)
    ///
    /// #### param
    /// - `init`: start register input callback function
    /// - `update`: update keyboard callback function
    /// - `tick`: main loop callback function
    ///
    /// #### Usage
    /// ```rust,no_run
    /// fn f(index: usize, input: &mut ScopedProtocol<Input>) -> Result { Ok(()) }
    /// KeyboardHotPlugMonitor::notify_mode_loop(f, f, f)?;
    /// ```
    /// More see examples/test_input_hotplug.rs
    pub unsafe fn notify_mode_loop<Init, Update, Loop, Res>
    (mut init: Init, mut update: Update, mut tick: Loop) -> Result<Res>
    where
        Init: FnMut(usize, &mut ScopedProtocol<Input>) -> Result<Res>,
        Update: FnMut(usize, &mut ScopedProtocol<Input>) -> Result<Res>,
        Loop: FnMut(usize, &mut ScopedProtocol<Input>) -> Result<Res>,
    {
        let mut keyboard_dirty = false;
        let mut keyboards: Vec<ScopedProtocol<Input>> = Vec::new();

        // during initialization, scan the existing keyboard first.
        Self::refresh_negative(&mut keyboards);

        // start hot-plug monitoring
        let flag_ptr = NonNull::from(&mut keyboard_dirty);

        macro_rules! f {
            ($f:ident) => {
                for (i, k) in keyboards.iter_mut().enumerate() { $f(i, k)?; }
            };
        }

        unsafe {
            // `_monitor` lifetime cover the unsafe block
            let _monitor = KeyboardHotPlugMonitor::new(flag_ptr)?;

            // User init operation
            f!(init);

            loop {
                spin_loop();

                if keyboard_dirty {
                    // reset immediately to prevent any new (un)plugs
                    // from being missed during processing
                    keyboard_dirty = false;
                    Self::refresh_negative(&mut keyboards);

                    // User update operation
                    f!(update);
                }

                // User main loop operation
                f!(tick);
            }
        }
    }

    /// support keyboard hotplug (pure polling mode)
    ///
    /// warn: Poor performance! frequent opening and closing of protocols.
    ///
    /// #### param
    /// - `init`: start register input callback function
    /// - `update`: update keyboard callback function
    /// - `tick`: main loop callback function
    ///
    /// #### Usage
    /// ```rust,no_run
    /// fn f(index: usize, input: &mut ScopedProtocol<Input>) -> Result { Ok(()) }
    /// KeyboardHotPlugMonitor::polling_mode_loop(f, f, f)?;
    /// ```
    /// More see examples/test_polling_hotplug.rs
    ///
    /// #### config
    /// [`REFRESH_POSITIVE_INPUT_DEVICE_TIME::set`] configuration can be modified
    /// before this [`KeyboardHotPlugMonitor::polling_mode_loop`] is used.
    pub unsafe fn polling_mode_loop<Init, Update, Loop, Res>
    (mut init: Init, mut update: Update, mut tick: Loop) -> Result<Res>
    where
        Init: FnMut(usize, &mut ScopedProtocol<Input>) -> Result<Res>,
        Update: FnMut(usize, &mut ScopedProtocol<Input>) -> Result<Res>,
        Loop: FnMut(usize, &mut ScopedProtocol<Input>) -> Result<Res>,
    {
        let mut keyboards: Vec<ScopedProtocol<Input>> = Vec::new();
        // Initial device discovery
        Self::refresh_positive(&mut keyboards);

        // --- Timer Setup ---
        // Create a timer event. In UEFI, timer events are used to signal intervals.
        let timer_event = unsafe {
            create_event(
                EventType::TIMER,
                Tpl::CALLBACK,
                None, // No callback function needed, we will poll it
                None,
            )?
        };

        // 配置值单位是 UEFI SetTimer 的 100ns（见 config 注释），转 Duration。
        set_timer(
            &timer_event,
            TimerTrigger::Periodic(Duration::from_nanos(
                REFRESH_POSITIVE_INPUT_DEVICE_TIME::get().saturating_mul(100),
            )),
        )?;

        macro_rules! f {
            ($f:ident) => {
                for (i, k) in keyboards.iter_mut().enumerate() { $f(i, k)?; }
            };
        }

        // Run initial user-defined init
        f!(init);
        loop {
            // Perform the main loop tick for every keyboard
            f!(tick);
            spin_loop();

            // Check if the timer has fired (non-blocking)
            // check_event returns Ok(true) if signaled, or an Error if not yet signaled.
            if let Ok(true) = check_event(&timer_event) {
                Self::refresh_positive(&mut keyboards);
                // Run initial user-defined update
                f!(update);
            }
        }
    }
}

