//! G6 EVIDENCE-ONLY PROTOTYPE — NOT PRODUCTION CODE.
//!
//! Candidate spike for the G6 "Indicator decision" gate (TDD contract
//! §30; `docs/guardian/30_TDD/GUARDIAN_G6_IMPLEMENTATION_HANDOFF.md`).
//! Evaluates candidate 1 ("legacy GTK3 Ayatana AppIndicator") under real
//! GNOME 50/Xfce 4.20 sessions in a disposable VM.
//!
//! Uses a minimal, hand-written `extern "C"` FFI surface against
//! `libayatana-appindicator3` plus the `gtk`/`gtk-sys` crates for
//! constructing the `GtkMenu`/`GtkMenuItem` widgets `app_indicator_set_menu`
//! requires. See `build.rs` for why: the only published Rust `-sys` crate
//! for this library (`libayatana-appindicator-sys` 0.2.0) fails to build
//! against Ubuntu 26.04's current glib headers -- a real, reproducible
//! tooling-compatibility finding, not a shortcut. The function/enum
//! declarations below were verified against the real installed header
//! (`/usr/include/libayatana-appindicator3-0.1/libayatana-appindicator/app-indicator.h`),
//! not guessed.
//!
//! Contains no Guardian authorization, transaction, provider, diagnostic,
//! or recorder logic -- never references `guardian-core`. DISPOSABLE:
//! built and run only inside a disposable VM, never on a primary
//! workstation.

use std::ffi::{CString, c_char, c_void};

use gtk::prelude::*;

#[repr(C)]
struct AppIndicator {
    _private: [u8; 0],
}

#[allow(non_camel_case_types)]
#[repr(C)]
#[derive(Clone, Copy)]
enum AppIndicatorCategory {
    ApplicationStatus = 0,
    #[allow(dead_code)]
    Communications = 1,
    #[allow(dead_code)]
    SystemServices = 2,
    #[allow(dead_code)]
    Hardware = 3,
    #[allow(dead_code)]
    Other = 4,
}

#[allow(non_camel_case_types)]
#[repr(C)]
#[derive(Clone, Copy)]
enum AppIndicatorStatus {
    #[allow(dead_code)]
    Passive = 0,
    Active = 1,
    Attention = 2,
}

#[link(name = "ayatana-appindicator3")]
unsafe extern "C" {
    fn app_indicator_new(
        id: *const c_char,
        icon_name: *const c_char,
        category: AppIndicatorCategory,
    ) -> *mut AppIndicator;
    fn app_indicator_set_status(indicator: *mut AppIndicator, status: AppIndicatorStatus);
    fn app_indicator_set_menu(indicator: *mut AppIndicator, menu: *mut c_void);
}

fn cstr(s: &str) -> CString {
    CString::new(s).expect("no interior NUL")
}

fn main() {
    eprintln!(
        "[g6-evidence] G6 EVIDENCE-ONLY ayatana-gtk3 prototype starting, pid={}",
        std::process::id()
    );

    gtk::init().expect("gtk::init failed -- is a real X11/Wayland display available?");

    let id = cstr("guardian-g6-evidence-ayatana-gtk3");
    let icon = cstr("emblem-default");

    // SAFETY: `id`/`icon` are valid, NUL-terminated C strings kept alive
    // for this call; `app_indicator_new` copies what it needs internally
    // (standard GObject construction convention).
    let indicator = unsafe {
        app_indicator_new(id.as_ptr(), icon.as_ptr(), AppIndicatorCategory::ApplicationStatus)
    };
    if indicator.is_null() {
        eprintln!("[g6-evidence] app_indicator_new FAILED (returned null)");
        std::process::exit(1);
    }
    eprintln!("[g6-evidence] app_indicator_new succeeded");

    let menu = gtk::Menu::new();

    let click_item = gtk::MenuItem::with_label("Click me (see stderr for count)");
    let click_count = std::rc::Rc::new(std::cell::Cell::new(0_u64));
    {
        let click_count = click_count.clone();
        click_item.connect_activate(move |_| {
            click_count.set(click_count.get() + 1);
            eprintln!(
                "[g6-evidence] menu item activated, menu_clicks={}",
                click_count.get()
            );
        });
    }
    menu.append(&click_item);

    let degrade_item = gtk::CheckMenuItem::with_label("Simulate degraded status");
    {
        let indicator_addr = indicator as usize;
        degrade_item.connect_toggled(move |item| {
            let indicator = indicator_addr as *mut AppIndicator;
            let degraded = item.is_active();
            let status = if degraded {
                AppIndicatorStatus::Attention
            } else {
                AppIndicatorStatus::Active
            };
            // SAFETY: `indicator` is a process-lifetime singleton, never
            // freed before exit.
            unsafe { app_indicator_set_status(indicator, status) };
            eprintln!(
                "[g6-evidence] status toggled to {}",
                if degraded { "Degraded" } else { "Healthy" }
            );
        });
    }
    menu.append(&degrade_item);

    let exit_item = gtk::MenuItem::with_label("Exit");
    exit_item.connect_activate(|_| {
        eprintln!("[g6-evidence] exit requested via menu");
        std::process::exit(0);
    });
    menu.append(&exit_item);

    menu.show_all();

    // SAFETY: `menu`'s underlying GObject is kept alive for the process
    // lifetime via `mem::forget` below; the raw pointer handed to
    // `app_indicator_set_menu` (via glib's ToGlibPtr, which yields the
    // real `GtkMenu*`) remains valid for exactly that long. `c_void` is
    // used on the FFI side since this file declares no `GtkMenu` type of
    // its own -- the C function only ever dereferences it as an opaque
    // GObject pointer.
    unsafe {
        use glib::translate::ToGlibPtr;
        let raw: *mut gtk_sys::GtkMenu = menu.to_glib_none().0;
        app_indicator_set_menu(indicator, raw.cast::<c_void>());
        app_indicator_set_status(indicator, AppIndicatorStatus::Active);
    }
    eprintln!("[g6-evidence] app_indicator_set_menu + set_status(ACTIVE) done, entering gtk::main()");

    std::mem::forget(menu);

    gtk::main();
}
