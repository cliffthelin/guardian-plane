//! G6 EVIDENCE-ONLY PROTOTYPE — NOT PRODUCTION CODE.
//!
//! Candidate spike for the G6 "Indicator decision" gate (TDD contract
//! §30; `docs/guardian/30_TDD/GUARDIAN_G6_IMPLEMENTATION_HANDOFF.md`).
//! Evaluates candidate 3 ("direct Rust SNI + canonical DBusMenu, e.g.
//! `ksni`") under real GNOME 50/Xfce 4.20 sessions in a disposable VM.
//!
//! This is deliberately the *simplest possible* StatusNotifierItem tray
//! icon plus a small usable menu -- just enough to exercise §30's
//! required-test list (icon appears, menu opens, menu action invokes a
//! handler, status/icon can change, degraded state is representable). It
//! contains no Guardian authorization, transaction, provider, diagnostic,
//! or recorder logic -- it never talks to `guardian-core` at all, per the
//! G6 handoff's thin-client-boundary requirement (§ "Thin-client
//! boundary" in the task brief; contract §31).
//!
//! DISPOSABLE: built and run only inside a disposable VM
//! (`/tmp/g6-evidence-vm` at authoring time), never on a primary
//! workstation. Not wired into `guardian-daemon`, not part of any
//! production build, not referenced by the Cargo workspace at
//! `/home/Cliff/SysProjects/Ubuntu_Guardian_Plane/Cargo.toml`.

use ksni::TrayMethods;
use ksni::menu::{CheckmarkItem, StandardItem};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Status {
    Healthy,
    Degraded,
}

#[derive(Debug)]
struct EvidenceTray {
    status: Status,
    menu_clicks: u64,
}

impl ksni::Tray for EvidenceTray {
    fn id(&self) -> String {
        "guardian-g6-evidence-ksni".into()
    }

    fn icon_name(&self) -> String {
        match self.status {
            Status::Healthy => "emblem-default".into(),
            Status::Degraded => "dialog-warning".into(),
        }
    }

    fn title(&self) -> String {
        match self.status {
            Status::Healthy => "Guardian G6 evidence (healthy)".into(),
            Status::Degraded => "Guardian G6 evidence (DEGRADED)".into(),
        }
    }

    fn tool_tip(&self) -> ksni::ToolTip {
        ksni::ToolTip {
            title: self.title(),
            description: format!("menu_clicks={}", self.menu_clicks),
            ..Default::default()
        }
    }

    fn menu(&self) -> Vec<ksni::MenuItem<Self>> {
        vec![
            StandardItem {
                label: format!("Click me (clicks so far: {})", self.menu_clicks),
                activate: Box::new(|this: &mut Self| {
                    this.menu_clicks += 1;
                    eprintln!(
                        "[g6-evidence] menu item activated, menu_clicks={}",
                        this.menu_clicks
                    );
                }),
                ..Default::default()
            }
            .into(),
            CheckmarkItem {
                label: "Simulate degraded status".into(),
                checked: self.status == Status::Degraded,
                activate: Box::new(|this: &mut Self| {
                    this.status = match this.status {
                        Status::Healthy => Status::Degraded,
                        Status::Degraded => Status::Healthy,
                    };
                    eprintln!("[g6-evidence] status toggled to {:?}", this.status);
                }),
                ..Default::default()
            }
            .into(),
            StandardItem {
                label: "Exit".into(),
                icon_name: "application-exit".into(),
                activate: Box::new(|_| {
                    eprintln!("[g6-evidence] exit requested via menu");
                    std::process::exit(0);
                }),
                ..Default::default()
            }
            .into(),
        ]
    }
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    eprintln!(
        "[g6-evidence] G6 EVIDENCE-ONLY ksni prototype starting, pid={}",
        std::process::id()
    );
    let tray = EvidenceTray {
        status: Status::Healthy,
        menu_clicks: 0,
    };
    let handle = match tray.spawn().await {
        Ok(h) => {
            eprintln!("[g6-evidence] tray.spawn() succeeded, StatusNotifierItem registered");
            h
        }
        Err(error) => {
            eprintln!("[g6-evidence] tray.spawn() FAILED: {error:?}");
            std::process::exit(1);
        }
    };

    // Keep the process alive; allow SIGTERM for clean teardown from the
    // evidence-gathering harness.
    let _ = handle;
    let mut term = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
        .expect("install SIGTERM handler");
    term.recv().await;
    eprintln!("[g6-evidence] SIGTERM received, exiting");
}
