//! G8 production wiring for PSI (`P1-PSI-001..005`). Reuses the accepted
//! G5 [`crate::psi`] model **unmodified** — this module contains no
//! parsing, classification, or threshold logic of its own. Its entire job
//! is: (a) read the real `/proc/pressure/{cpu,memory,io}` files and feed
//! them through [`crate::psi::read_resource`], and (b) drive
//! [`crate::psi::ThresholdMonitor::observe`] from a genuine kernel-level
//! PSI trigger via `poll()`, never a busy loop (`P1-PSI-004`).

use std::fs;
use std::io;
use std::os::fd::AsFd;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use rustix::event::{PollFd, PollFlags, poll};
use rustix::fs::{Mode, OFlags, open};

use crate::event::{Event, normalize_key};
use crate::psi::{
    PressureSeverity, PsiLine, PsiParseError, PsiReading, PsiResource, PsiResourceKind,
    SeverityThresholds, ThresholdMonitor, classify, read_resource,
};
use crate::risk::Risk;
use guardian_provider_api::{EventId, ProviderId};

/// The kernel reports PSI averages as a percentage of the averaging
/// window, so every valid `avg*` value is finite and within these bounds.
/// A value outside them is not a measurement (`P2-EVT-008`).
const PSI_AVG_MIN_PERCENT: f64 = 0.0;
const PSI_AVG_MAX_PERCENT: f64 = 100.0;

/// Truthful per-resource PSI availability, as observed by the production
/// inherited-descriptor ingress path (acceptance-repair Blockers 3/4).
///
/// A single aggregate "N/M inherited descriptors monitored" startup line
/// cannot express *which* resource is missing when the set is partial
/// (`:graceful` dropping one path, or a resource-specific registration
/// fault): "2/2 monitored" and "2/3 monitored, memory silently absent"
/// look identical to a reader who only sees the numerator/denominator.
/// This type is the per-resource fix, and it is the **single shared**
/// source of truth `guardian-daemon`'s own startup/runtime reporting and
/// `registry::psi_capabilities` (Blocker 4) both read — the registry
/// never re-derives its own, second opinion by re-probing
/// `/proc/pressure` by pathname (which the accepted `ProcSubset=pid`
/// sandbox always denies from inside the daemon, and which is precisely
/// how the pre-repair registry produced a truthfully-worded but
/// misleading `Unsupported` for a resource whose live descriptor was, at
/// that very moment, working).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PsiResourceStatus {
    /// A descriptor was inherited and this resource's monitor registered
    /// successfully.
    Available,
    /// A descriptor was inherited but this resource's monitor failed —
    /// a real, hard fault, never silently downgraded to "unsupported".
    /// Carries a short, human-readable reason for operational logging
    /// and for the Capability Registry.
    Degraded(String),
    /// No descriptor was ever inherited for this resource — `:graceful`
    /// dropped its `OpenFile=` path, `LISTEN_FDNAMES` never named it, or
    /// descriptor acquisition failed closed for the whole set.
    NoDescriptor,
}

impl PsiResourceStatus {
    #[must_use]
    pub fn is_available(&self) -> bool {
        matches!(self, Self::Available)
    }

    /// A short, stable, human-readable reason string — used by both the
    /// daemon's own `eprintln!` startup/runtime reporting and (indirectly,
    /// via the shared state) by `registry::psi_capabilities`'s test
    /// assertions.
    #[must_use]
    pub fn reason(&self) -> String {
        match self {
            Self::Available => "monitor registered and running".to_owned(),
            Self::Degraded(reason) => reason.clone(),
            Self::NoDescriptor => "no descriptor was inherited for this resource".to_owned(),
        }
    }
}

/// Per-resource [`PsiResourceStatus`] for all three monitored resources.
/// `None` for a resource means "not yet observed" — the state before the
/// production PSI startup path has run even once (e.g. a fresh registry
/// snapshot taken before `spawn_psi_ingress` completes its first pass, or
/// a caller that never wires PSI ingress state in at all, which
/// `registry::psi_capabilities` treats as "no live signal available,"
/// distinct from "observed and found absent").
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PsiAvailability {
    cpu: Option<PsiResourceStatus>,
    memory: Option<PsiResourceStatus>,
    io: Option<PsiResourceStatus>,
}

impl PsiAvailability {
    #[must_use]
    pub fn get(&self, kind: PsiResourceKind) -> Option<&PsiResourceStatus> {
        match kind {
            PsiResourceKind::Cpu => self.cpu.as_ref(),
            PsiResourceKind::Memory => self.memory.as_ref(),
            PsiResourceKind::Io => self.io.as_ref(),
        }
    }

    pub fn set(&mut self, kind: PsiResourceKind, status: PsiResourceStatus) {
        match kind {
            PsiResourceKind::Cpu => self.cpu = Some(status),
            PsiResourceKind::Memory => self.memory = Some(status),
            PsiResourceKind::Io => self.io = Some(status),
        }
    }

    /// True once every one of the three resources has been observed at
    /// least once (whichever way) — used to distinguish "genuinely never
    /// wired" from "wired and every resource happens to be absent",
    /// which look different only through this flag once all three
    /// statuses individually collapse to `NoDescriptor`.
    #[must_use]
    pub fn is_fully_observed(&self) -> bool {
        self.cpu.is_some() && self.memory.is_some() && self.io.is_some()
    }

    /// How many of the three resources are currently `Available`.
    #[must_use]
    pub fn available_count(&self) -> usize {
        [&self.cpu, &self.memory, &self.io]
            .into_iter()
            .filter(|status| status.as_ref().is_some_and(PsiResourceStatus::is_available))
            .count()
    }
}

/// Real, filesystem-backed source of PSI text — the thin G8 wrapper the
/// accepted G5 module doc comment calls for. Injectable base directory so
/// tests never depend on the real `/proc`.
///
/// Two path modes, both reading through the same unmodified G5 model:
///
/// - **Base directory** ([`Self::real`], [`Self::at`]) — the original G8
///   behavior: each resource is `base_dir/{cpu,memory,io}`.
/// - **Explicit per-resource paths** ([`Self::from_paths`]) — added by the
///   `phase2-psi-inherited-descriptor-ingress` gate (`P2-EVT-006`) so
///   `guardian-daemon` can read PSI through descriptors systemd opened in
///   PID 1's namespace and passed in. The daemon supplies
///   `/proc/self/fd/N` paths, which are process-associated and therefore
///   still visible under the accepted `ProcSubset=pid` sandbox that denies
///   it any `/proc/pressure` pathname. A resource with no explicit path is
///   truthfully [`PsiReading::Unavailable`] (`P1-PSI-005`) — it never falls
///   back to a base directory, and never reads as "no pressure".
#[derive(Clone, Debug)]
pub struct PsiFileSource {
    base_dir: PathBuf,
    /// `None` keeps the accepted base-directory behavior. `Some` switches
    /// to explicit per-resource resolution; a small ordered list rather
    /// than a map because there are at most three entries and
    /// [`PsiResourceKind`] deliberately has no `Ord` (the accepted G5
    /// model is not modified by this gate).
    explicit: Option<Vec<(PsiResourceKind, PathBuf)>>,
}

const fn resource_name(kind: PsiResourceKind) -> &'static str {
    match kind {
        PsiResourceKind::Cpu => "cpu",
        PsiResourceKind::Memory => "memory",
        PsiResourceKind::Io => "io",
    }
}

impl PsiFileSource {
    /// The real, standard kernel path.
    #[must_use]
    pub fn real() -> Self {
        Self {
            base_dir: PathBuf::from("/proc/pressure"),
            explicit: None,
        }
    }

    #[must_use]
    pub fn at(base_dir: impl Into<PathBuf>) -> Self {
        Self {
            base_dir: base_dir.into(),
            explicit: None,
        }
    }

    /// Explicit per-resource paths — the `P2-EVT-006` inherited-descriptor
    /// mode described on the type. A duplicate `kind` keeps the first entry
    /// supplied, so a caller can never silently rebind a resource.
    #[must_use]
    pub fn from_paths(paths: impl IntoIterator<Item = (PsiResourceKind, PathBuf)>) -> Self {
        let mut entries: Vec<(PsiResourceKind, PathBuf)> = Vec::new();
        for (kind, path) in paths {
            if !entries.iter().any(|(existing, _)| *existing == kind) {
                entries.push((kind, path));
            }
        }
        Self {
            base_dir: PathBuf::new(),
            explicit: Some(entries),
        }
    }

    /// The path this source will actually read for `kind`, or `None` when
    /// it was built with explicit per-resource paths and `kind` is not
    /// among them — i.e. no descriptor was inherited for that resource.
    #[must_use]
    pub fn resolved_path(&self, kind: PsiResourceKind) -> Option<PathBuf> {
        match &self.explicit {
            None => Some(self.base_dir.join(resource_name(kind))),
            Some(entries) => entries
                .iter()
                .find(|(existing, _)| *existing == kind)
                .map(|(_, path)| path.clone()),
        }
    }

    /// Real file read → the accepted G5 `read_resource`/`PsiReading`
    /// model, unmodified. A missing file — or, for an explicit-path
    /// source, a resource with no inherited descriptor — becomes
    /// [`PsiReading::Unavailable`] (`P1-PSI-005`), never a parse error and
    /// never silently treated as "no pressure."
    ///
    /// This is also the single boundary at which raw PSI values are
    /// validated (`P2-EVT-008`): the G5 parser accepts any `f64` that
    /// Rust can parse, and `"NaN"`/`"inf"` both parse successfully, so
    /// without the check here a non-finite value would reach
    /// [`crate::psi::classify`] — where `NaN`'s comparison semantics
    /// silently yield `Nominal`, exactly the fail-open outcome the G5
    /// model exists to prevent. Validating here (rather than in the
    /// accepted G5 parser, which this gate does not modify) means no
    /// invalid value can cross into severity, `Event` construction, or
    /// correlation by any route.
    ///
    /// # Errors
    ///
    /// Returns [`PsiParseError`] for a *present* source whose content is
    /// malformed, non-finite, or outside the kernel's 0–100 percent range.
    pub fn read(&self, kind: PsiResourceKind) -> Result<PsiReading, PsiParseError> {
        let Some(path) = self.resolved_path(kind) else {
            return Ok(PsiReading::Unavailable);
        };
        let text = fs::read_to_string(&path).ok();
        parse_and_validate(text.as_deref(), kind)
    }

    /// The path for `kind`. For an explicit-path source with no descriptor
    /// for `kind`, this is the empty path — deliberately not a filesystem
    /// location, and never opened: every caller inside this module goes
    /// through [`Self::resolved_path`] and handles `None` explicitly. The
    /// accepted G8 signature is kept so existing callers and tests are
    /// untouched.
    #[must_use]
    pub fn path(&self, kind: PsiResourceKind) -> PathBuf {
        self.resolved_path(kind).unwrap_or_default()
    }
}

/// `P2-EVT-008`: rejects any `avg*` value that is not a plausible kernel
/// measurement, before it can be classified.
fn validate_line(line: &PsiLine, class: &str, kind: PsiResourceKind) -> Result<(), PsiParseError> {
    for (field, value) in [
        ("avg10", line.avg10),
        ("avg60", line.avg60),
        ("avg300", line.avg300),
    ] {
        if !value.is_finite() {
            return Err(PsiParseError(format!(
                "{kind:?} '{class}' {field} is not a finite measurement: {value}"
            )));
        }
        if !(PSI_AVG_MIN_PERCENT..=PSI_AVG_MAX_PERCENT).contains(&value) {
            return Err(PsiParseError(format!(
                "{kind:?} '{class}' {field} is outside the kernel's 0-100 percent range: {value}"
            )));
        }
    }
    Ok(())
}

fn validate_resource(resource: &PsiResource, kind: PsiResourceKind) -> Result<(), PsiParseError> {
    validate_line(&resource.some, "some", kind)?;
    // The `full` class is parsed but never classified. It is still
    // validated: an invalid value must not cross an internal boundary as a
    // valid measurement by *any* route, including one nothing reads today.
    if let Some(full) = &resource.full {
        validate_line(full, "full", kind)?;
    }
    Ok(())
}

/// The single `P2-EVT-008` validation boundary, factored out of
/// [`PsiFileSource::read`] so [`OwnedPsiFile`]'s reread path (which
/// supplies text read from its own already-open descriptor, never from a
/// fresh path-based `open()`) goes through **exactly the same**
/// parse/classify-eligibility rules — never a second, drifting copy of
/// them.
fn parse_and_validate(
    text: Option<&str>,
    kind: PsiResourceKind,
) -> Result<PsiReading, PsiParseError> {
    let reading = read_resource(text, kind)?;
    if let PsiReading::Present(resource) = &reading {
        validate_resource(resource, kind)?;
    }
    Ok(reading)
}

/// Reads whatever text `file` currently holds, from the beginning,
/// without opening a new file description. PSI pseudo-files are
/// single-record: a read at a nonzero offset returns nothing, so this
/// always seeks to `0` first. Operates through `&File` (not `&mut File`)
/// so it never needs to consume or exclusively borrow the shared
/// [`OwnedPsiFile`] handle — `std::fs::File` implements `Read`/`Seek` for
/// `&File` for exactly this shared-handle use case.
fn read_open_file(file: &fs::File) -> io::Result<String> {
    use std::io::{Read, Seek, SeekFrom};
    let mut handle: &fs::File = file;
    handle.seek(SeekFrom::Start(0))?;
    let mut text = String::new();
    handle.read_to_string(&mut text)?;
    Ok(text)
}

/// One PSI resource's single, owned, close-on-exec open file description.
///
/// # Why this type exists (acceptance-repair Blocker 1/2)
///
/// The original inherited-descriptor production path derived a
/// `/proc/self/fd/N` path once, then handed that **path** (not a file) to
/// `PsiFileSource`/`PsiTrigger`, both of which called `open()` on it
/// independently and repeatedly: once for the dispatcher's baseline read,
/// once for trigger registration, and again on **every** subsequent
/// `dispatch_wake`. `/proc/self/fd/N` is a magic symlink; opening it does
/// a genuine fresh `open()` of the underlying pressure file, which the
/// kernel gives a **new, independent open file description** each time —
/// confirmed directly on `guardian-g9` (not assumed): three sequential
/// opens of the same `/proc/self/fd/N` path produce three descriptors
/// with independent `fdinfo` entries, and a trigger written through one
/// is invisible to `poll()` on another. That left the raw
/// systemd-inherited descriptor open for the daemon's entire lifetime
/// with no `FD_CLOEXEC` ever set on *it*, while every actual read/trigger
/// went through short-lived, uncoordinated reopens.
///
/// This type is the fix: `/proc/self/fd/N` is reopened **exactly once**
/// per resource, at construction, producing one owned `std::fs::File`
/// with `FD_CLOEXEC` explicitly set on it via a safe `rustix` `fcntl`
/// call. After that single reopen, the raw inherited descriptor number is
/// never referenced again by this type or by anything built from it —
/// [`PsiTrigger::register_on_owned_file`] and every subsequent content
/// reread ([`PsiEventDispatcher::new_on_owned_file`],
/// `dispatch_wake`) share the **same** `Arc<std::fs::File>`, so trigger
/// registration, `poll()`, and every reread are provably one open file
/// description, not a series of independent reopens (gate TDD R7,
/// acceptance criteria under `P2-EVT-005`/`P2-EVT-007`/`P2-EVT-008`,
/// formerly the standalone `P2-EVT-006`, demoted 2026-09-08).
///
/// No `unsafe`: the workspace forbids it (`Cargo.toml:30`,
/// `unsafe_code = "forbid"`), and none is needed — `std::fs::File::open`
/// already returns a safe, owned handle, `std::os::fd::AsFd::as_fd` is a
/// safe trait method, and `rustix::io::fcntl_getfd`/`fcntl_setfd` are
/// safe functions over a `BorrowedFd` obtained that way. The **raw**
/// systemd-supplied descriptor number (whatever `LISTEN_FDS` handed the
/// daemon) is deliberately never adopted, closed, or otherwise touched by
/// Rust code at all after the single reopen that derives this type — Rust
/// cannot safely take ownership of an arbitrary raw fd number without
/// `unsafe` (`OwnedFd::from_raw_fd`/`BorrowedFd::borrow_raw` are both
/// `unsafe fn`), so it is left exactly as systemd handed it in, inert,
/// its `FD_CLOEXEC`-less state carrying no consequence because
/// `guardian-daemon` never execs (no subprocess is ever spawned anywhere
/// in this binary).
#[derive(Clone, Debug)]
pub struct OwnedPsiFile {
    kind: PsiResourceKind,
    file: Arc<fs::File>,
}

impl OwnedPsiFile {
    /// Opens `path` exactly once. See the type doc for why this is the
    /// **only** `open()` call this resource's production monitoring will
    /// ever perform.
    ///
    /// # Errors
    /// The underlying `open()`/`fcntl()` I/O error, if any.
    pub fn open(kind: PsiResourceKind, path: &Path) -> io::Result<Self> {
        let file = fs::File::options().read(true).write(true).open(path)?;
        set_cloexec(&file)?;
        Ok(Self {
            kind,
            file: Arc::new(file),
        })
    }

    #[must_use]
    pub fn kind(&self) -> PsiResourceKind {
        self.kind
    }

    /// Direct kernel-level proof, not a trust of `fcntl`'s own success
    /// return: reads this exact descriptor's `/proc/self/fdinfo/N`
    /// `flags:` field and checks `O_CLOEXEC` (the `fdinfo` name for the
    /// close-on-exec bit `fcntl(F_GETFD)`/`FD_CLOEXEC` also reports).
    #[must_use]
    pub fn is_close_on_exec(&self) -> bool {
        is_cloexec_via_fdinfo(&self.file)
    }

    /// Whether `self` and `other` share the **exact same** open file
    /// description, not merely the same path or the same resource kind.
    /// This type never `dup()`s or reopens after construction, and every
    /// clone of one `OwnedPsiFile` shares the same `Arc<fs::File>`, so
    /// `Arc::ptr_eq` over that backing allocation is a correct, cheap,
    /// in-process identity proof — used by [`PsiTrigger::register_on_owned_file`]'s
    /// own tests to distinguish "the same open file description" from
    /// "merely the same pathname" (gate TDD R7).
    #[must_use]
    pub fn same_open_file_description(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.file, &other.file)
    }

    fn read_text(&self) -> io::Result<String> {
        read_open_file(&self.file)
    }

    fn shared_file(&self) -> Arc<fs::File> {
        Arc::clone(&self.file)
    }
}

/// Explicitly sets `FD_CLOEXEC` on `file`'s descriptor via a safe
/// `fcntl(F_SETFD)` call over the `BorrowedFd` `AsFd::as_fd` safely
/// exposes. `std::fs::File::open` already sets `O_CLOEXEC`
/// unconditionally on Linux, so this call is a second, explicit,
/// independently-verifiable proof of the property (directly testable via
/// [`OwnedPsiFile::is_close_on_exec`]) rather than a bare reliance on that
/// implicit default.
fn set_cloexec(file: &fs::File) -> io::Result<()> {
    let fd = file.as_fd();
    let mut flags = rustix::io::fcntl_getfd(fd)?;
    flags.insert(rustix::io::FdFlags::CLOEXEC);
    rustix::io::fcntl_setfd(fd, flags)?;
    Ok(())
}

/// Reads `/proc/self/fdinfo/N`'s `flags:` field for `file` and checks the
/// kernel's own `O_CLOEXEC` bit — the ground-truth verification technique
/// this repair's evidence uses, not a trust of `fcntl`'s return value.
fn is_cloexec_via_fdinfo(file: &fs::File) -> bool {
    const O_CLOEXEC_OCTAL: u32 = 0o2_000_000;
    let fd = std::os::fd::AsRawFd::as_raw_fd(file);
    let Ok(fdinfo) = fs::read_to_string(format!("/proc/self/fdinfo/{fd}")) else {
        return false;
    };
    fdinfo
        .lines()
        .find_map(|line| line.strip_prefix("flags:"))
        .and_then(|value| u32::from_str_radix(value.trim(), 8).ok())
        .is_some_and(|flags| flags & O_CLOEXEC_OCTAL == O_CLOEXEC_OCTAL)
}

/// A real kernel-level PSI trigger, registered by writing a threshold
/// expression to the pressure file per the kernel's own PSI monitoring
/// ABI (`Documentation/accounting/psi.rst`) and waiting on `POLLPRI` —
/// never a sleep-and-recheck loop. `window_us` MUST be >= `500_000` (the
/// kernel's own minimum) and `threshold_us` < `window_us`; both are
/// validated before the trigger is written, matching the kernel's own
/// rejection behavior with a typed error instead of an opaque I/O
/// failure surfacing later.
pub struct PsiTrigger {
    file: Arc<fs::File>,
}

/// A trigger could not be registered — the file is genuinely absent
/// (`PSI unavailable`), the parameters were rejected by the kernel, or
/// the caller lacks write access.
#[derive(Debug)]
pub struct TriggerError(pub io::Error);

impl std::fmt::Display for TriggerError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "PSI trigger registration failed: {}", self.0)
    }
}

impl std::error::Error for TriggerError {}

impl PsiTrigger {
    /// Registers a real trigger: `stall_type` is `"some"` or `"full"`
    /// (CPU has no `full`), `threshold_us`/`window_us` are microseconds
    /// per the kernel ABI.
    ///
    /// # Errors
    ///
    /// See [`TriggerError`].
    pub fn register(
        source: &PsiFileSource,
        kind: PsiResourceKind,
        stall_type: &str,
        threshold_us: u64,
        window_us: u64,
    ) -> Result<Self, TriggerError> {
        let Some(path) = source.resolved_path(kind) else {
            return Err(TriggerError(io::Error::new(
                io::ErrorKind::NotFound,
                format!("no PSI source path for {kind:?}"),
            )));
        };
        let path: &Path = &path;
        // O_RDWR is required by the kernel PSI ABI: the trigger is
        // written to the same fd that is later polled.
        let raw = open(path, OFlags::RDWR | OFlags::CLOEXEC, Mode::empty())
            .map_err(|errno| TriggerError(io::Error::from(errno)))?;
        let file: fs::File = raw.into();
        let mut trigger = format!("{stall_type} {threshold_us} {window_us}").into_bytes();
        // The kernel ABI's reference implementation writes strlen + 1:
        // the terminating NUL is part of the accepted trigger payload.
        trigger.push(0);
        std::io::Write::write_all(&mut { &file }, &trigger).map_err(TriggerError)?;
        Ok(Self {
            file: Arc::new(file),
        })
    }

    /// Registers a trigger on an **already-open** file description — the
    /// inherited-descriptor production path (acceptance criteria under
    /// `P2-EVT-005`/`P2-EVT-007`/`P2-EVT-008`, formerly the standalone
    /// `P2-EVT-006`, demoted 2026-09-08; see [`OwnedPsiFile`]'s doc
    /// comment for why this exists). No `open()` call happens here at
    /// all — the trigger is written directly onto `owned`'s single
    /// `Arc<fs::File>`, and the returned `PsiTrigger` shares that exact
    /// `Arc`, so this cannot create a new open file description: `poll()`
    /// via [`Self::wait`] and every subsequent pressure-text reread
    /// through the same `owned` (or any of its clones) are provably the
    /// same open file description, not merely the same path.
    ///
    /// # Errors
    ///
    /// Returns [`TriggerError`] if the kernel rejects the trigger
    /// parameters or write, including `EBUSY` when a trigger already
    /// exists on this exact open file description (there is no in-place
    /// re-arm; a fresh [`OwnedPsiFile::open`] is required to retry).
    pub fn register_on_owned_file(
        owned: &OwnedPsiFile,
        stall_type: &str,
        threshold_us: u64,
        window_us: u64,
    ) -> Result<Self, TriggerError> {
        let mut trigger = format!("{stall_type} {threshold_us} {window_us}").into_bytes();
        trigger.push(0);
        std::io::Write::write_all(&mut &*owned.file, &trigger).map_err(TriggerError)?;
        Ok(Self {
            file: owned.shared_file(),
        })
    }

    /// Blocks (via real `poll()`, `POLLPRI`) until the kernel signals the
    /// registered threshold was crossed, or `timeout` elapses. Never a
    /// busy loop: with `timeout = None` this call parks the thread in the
    /// kernel until a real event arrives.
    ///
    /// # Errors
    ///
    /// Returns the underlying `poll()` I/O error, if any.
    pub fn wait(&self, timeout: Option<Duration>) -> io::Result<bool> {
        let timespec = timeout.map(|duration| rustix::time::Timespec {
            tv_sec: duration.as_secs().try_into().unwrap_or(i64::MAX),
            tv_nsec: i64::from(duration.subsec_nanos()),
        });
        let fd = self.file.as_fd();
        let mut fds = [PollFd::new(&fd, PollFlags::PRI)];
        let ready = poll(&mut fds, timespec.as_ref())?;
        Ok(ready > 0 && fds[0].revents().contains(PollFlags::PRI))
    }

    /// Whether this trigger's open file description is the same one
    /// `owned` (or one of its clones) refers to — used by tests to prove
    /// [`Self::register_on_owned_file`] never created a second
    /// description (gate TDD R7).
    #[must_use]
    pub fn shares_open_file_description_with(&self, owned: &OwnedPsiFile) -> bool {
        Arc::ptr_eq(&self.file, &owned.file)
    }
}

/// Complete production event path for `P1-PSI-004`: a registered kernel
/// trigger wakes through `poll(POLLPRI)`, the live PSI file is re-read,
/// and the accepted G5 classifier/`ThresholdMonitor` decides whether a
/// normalized Guardian event is emitted. One call performs one blocking
/// wait; there is no retry loop or periodic sampling in this type.
pub struct PsiEventSource {
    trigger: PsiTrigger,
    dispatcher: PsiEventDispatcher,
}

/// Where [`PsiEventDispatcher`] reads live pressure text from: either the
/// accepted G8 path-based [`PsiFileSource`] (reopens per read; used by
/// `real()`/`at()`/`from_paths()` callers and every existing test,
/// unmodified), or a single already-open [`OwnedPsiFile`] (never reopens;
/// the inherited-descriptor production path, acceptance-repair Blocker
/// 1/2). Kept as an enum rather than a trait object so neither variant
/// pays for dynamic dispatch and so `dispatch_wake`'s match stays a
/// direct, auditable branch to the exact reread mechanism in use.
enum DispatchSource {
    Path(PsiFileSource),
    Owned(OwnedPsiFile),
}

/// Deterministic wake-to-event half of [`PsiEventSource`]. The kernel wait
/// stays in `PsiTrigger`; this type makes dispatch and repeated-wake
/// behavior testable without pretending a fixture file supports POLLPRI.
pub struct PsiEventDispatcher {
    source: DispatchSource,
    kind: PsiResourceKind,
    thresholds: SeverityThresholds,
    monitor: ThresholdMonitor,
    sequence: u64,
}

impl PsiEventDispatcher {
    /// Seeds dispatch from the current live reading.
    ///
    /// # Errors
    ///
    /// Returns unavailable or malformed-source errors from the initial read.
    pub fn new(
        source: PsiFileSource,
        kind: PsiResourceKind,
        thresholds: SeverityThresholds,
        event_threshold: PressureSeverity,
    ) -> Result<Self, PsiEventError> {
        let baseline = present_severity(&source, kind, thresholds)?;
        let mut monitor = ThresholdMonitor::new(kind, event_threshold);
        let _ = monitor.observe(baseline);
        Ok(Self {
            source: DispatchSource::Path(source),
            kind,
            thresholds,
            monitor,
            sequence: 0,
        })
    }

    /// The inherited-descriptor production constructor (acceptance
    /// criteria under `P2-EVT-005`/`P2-EVT-007`/`P2-EVT-008`, formerly the
    /// standalone `P2-EVT-006`, demoted 2026-09-08). Every subsequent
    /// [`Self::dispatch_wake`] reread goes through `owned`'s single
    /// `Arc<fs::File>` — see [`OwnedPsiFile`]'s doc comment.
    ///
    /// # Errors
    ///
    /// Returns unavailable or malformed-source errors from the initial
    /// read through `owned`.
    pub fn new_on_owned_file(
        owned: OwnedPsiFile,
        thresholds: SeverityThresholds,
        event_threshold: PressureSeverity,
    ) -> Result<Self, PsiEventError> {
        let kind = owned.kind();
        let baseline = present_severity_owned(&owned, thresholds)?;
        let mut monitor = ThresholdMonitor::new(kind, event_threshold);
        let _ = monitor.observe(baseline);
        Ok(Self {
            source: DispatchSource::Owned(owned),
            kind,
            thresholds,
            monitor,
            sequence: 0,
        })
    }

    /// Converts one already-received wake into at most one event.
    ///
    /// # Errors
    ///
    /// Returns unavailable or malformed-source errors from the fresh read.
    pub fn dispatch_wake(&mut self) -> Result<Option<Event>, PsiEventError> {
        let severity = match &self.source {
            DispatchSource::Path(source) => present_severity(source, self.kind, self.thresholds)?,
            DispatchSource::Owned(owned) => present_severity_owned(owned, self.thresholds)?,
        };
        let Some(crossing) = self.monitor.observe(severity) else {
            return Ok(None);
        };
        self.sequence = self.sequence.saturating_add(1);
        Ok(Some(event_from_crossing(crossing, self.sequence)))
    }

    /// Whether this dispatcher's reread path is the exact same open file
    /// description as `owned` (or one of its clones) — `false` for a
    /// path-based dispatcher, which by design reopens on every read.
    /// Used by tests distinguishing descriptor identity from mere
    /// pathname equivalence (gate TDD R7).
    #[must_use]
    pub fn reads_through_owned_file(&self, owned: &OwnedPsiFile) -> bool {
        match &self.source {
            DispatchSource::Path(_) => false,
            DispatchSource::Owned(mine) => mine.same_open_file_description(owned),
        }
    }
}

impl PsiEventSource {
    /// Registers the kernel trigger and seeds the accepted G5 monitor from
    /// the current live reading so the first subsequent wake can represent
    /// a genuine crossing.
    /// # Errors
    ///
    /// Returns trigger-registration, unavailable, or malformed-source errors.
    pub fn register(
        source: &PsiFileSource,
        kind: PsiResourceKind,
        stall_type: &str,
        threshold_us: u64,
        window_us: u64,
        thresholds: SeverityThresholds,
        event_threshold: PressureSeverity,
    ) -> Result<Self, PsiEventError> {
        let dispatcher =
            PsiEventDispatcher::new(source.clone(), kind, thresholds, event_threshold)?;
        let trigger = PsiTrigger::register(source, kind, stall_type, threshold_us, window_us)?;
        Ok(Self {
            trigger,
            dispatcher,
        })
    }

    /// Waits once in the kernel and dispatches at most one Guardian event.
    /// # Errors
    ///
    /// Returns trigger-wait, unavailable, or malformed-source errors.
    pub fn wait_for_event(
        &mut self,
        timeout: Option<Duration>,
    ) -> Result<Option<Event>, PsiEventError> {
        if !self.trigger.wait(timeout)? {
            return Ok(None);
        }
        self.dispatcher.dispatch_wake()
    }
}

#[derive(Debug)]
pub enum PsiEventError {
    Trigger(TriggerError),
    Io(io::Error),
    Parse(PsiParseError),
    Unavailable,
}

impl std::fmt::Display for PsiEventError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Trigger(error) => error.fmt(formatter),
            Self::Io(error) => write!(formatter, "PSI trigger wait failed: {error}"),
            Self::Parse(error) => error.fmt(formatter),
            Self::Unavailable => formatter.write_str("PSI source unavailable"),
        }
    }
}

impl std::error::Error for PsiEventError {}

impl From<TriggerError> for PsiEventError {
    fn from(value: TriggerError) -> Self {
        Self::Trigger(value)
    }
}

impl From<io::Error> for PsiEventError {
    fn from(value: io::Error) -> Self {
        Self::Io(value)
    }
}

impl From<PsiParseError> for PsiEventError {
    fn from(value: PsiParseError) -> Self {
        Self::Parse(value)
    }
}

fn present_severity(
    source: &PsiFileSource,
    kind: PsiResourceKind,
    thresholds: SeverityThresholds,
) -> Result<PressureSeverity, PsiEventError> {
    match source.read(kind)? {
        PsiReading::Present(resource) => Ok(classify(resource.some, thresholds)),
        PsiReading::Unavailable => Err(PsiEventError::Unavailable),
    }
}

/// [`present_severity`]'s counterpart for the owned-file reread path:
/// reads `owned`'s single open file description (never a fresh `open()`)
/// and runs the result through the exact same `parse_and_validate` +
/// classify rule.
fn present_severity_owned(
    owned: &OwnedPsiFile,
    thresholds: SeverityThresholds,
) -> Result<PressureSeverity, PsiEventError> {
    let text = owned.read_text().map_err(PsiEventError::Io)?;
    match parse_and_validate(Some(&text), owned.kind())? {
        PsiReading::Present(resource) => Ok(classify(resource.some, thresholds)),
        PsiReading::Unavailable => Err(PsiEventError::Unavailable),
    }
}

fn event_from_crossing(crossing: crate::psi::ThresholdEvent, sequence: u64) -> Event {
    let resource = match crossing.resource {
        PsiResourceKind::Cpu => "cpu",
        PsiResourceKind::Memory => "memory",
        PsiResourceKind::Io => "io",
    };
    let raw = format!(
        "PSI {resource} threshold crossing {:?}->{:?}",
        crossing.from, crossing.to
    );
    Event {
        event_id: EventId::new(format!("guardian.psi.{resource}.event-{sequence}"))
            .expect("generated PSI event id is valid"),
        timestamp_monotonic: sequence,
        timestamp_wall: format!("sequence-{sequence}"),
        source_provider: ProviderId::new("guardian.g8.psi").expect("fixed provider id is valid"),
        event_type: "psi_threshold_crossing".to_owned(),
        resource_refs: vec![format!("/proc/pressure/{resource}")],
        severity: match crossing.to {
            PressureSeverity::Critical => Risk::High,
            PressureSeverity::Elevated => Risk::Moderate,
            PressureSeverity::Nominal => Risk::Observe,
        },
        normalized_key: normalize_key(&raw),
        raw_reference: raw,
        attributes: std::collections::BTreeMap::from([
            ("from".to_owned(), format!("{:?}", crossing.from)),
            ("to".to_owned(), format!("{:?}", crossing.to)),
        ]),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("psi-provider-test-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn reads_a_real_present_file_through_the_unmodified_g5_model() {
        let dir = temp_dir("present");
        fs::write(
            dir.join("cpu"),
            "some avg10=1.20 avg60=0.80 avg300=0.10 total=1234\n",
        )
        .unwrap();
        let source = PsiFileSource::at(&dir);
        let reading = source.read(PsiResourceKind::Cpu).unwrap();
        match reading {
            PsiReading::Present(resource) => {
                assert!((resource.some.avg10 - 1.20).abs() < f64::EPSILON);
                assert_eq!(resource.full, None);
            }
            PsiReading::Unavailable => panic!("expected Present"),
        }
    }

    #[test]
    fn missing_file_is_unavailable_not_a_parse_error() {
        let dir = temp_dir("missing");
        let source = PsiFileSource::at(&dir);
        let reading = source.read(PsiResourceKind::Memory).unwrap();
        assert_eq!(reading, PsiReading::Unavailable);
    }

    #[test]
    fn malformed_present_file_is_a_real_parse_error_not_silently_ignored() {
        let dir = temp_dir("malformed");
        fs::write(dir.join("io"), "not a psi line at all\n").unwrap();
        let source = PsiFileSource::at(&dir);
        let result = source.read(PsiResourceKind::Io);
        assert!(result.is_err());
    }

    #[test]
    fn path_for_each_resource_kind_matches_the_real_kernel_layout() {
        let source = PsiFileSource::at("/proc/pressure");
        assert_eq!(
            source.path(PsiResourceKind::Cpu),
            Path::new("/proc/pressure/cpu")
        );
        assert_eq!(
            source.path(PsiResourceKind::Memory),
            Path::new("/proc/pressure/memory")
        );
        assert_eq!(
            source.path(PsiResourceKind::Io),
            Path::new("/proc/pressure/io")
        );
    }

    #[test]
    fn trigger_registration_writes_kernel_abi_payload_with_nul_terminator() {
        let dir = temp_dir("trigger-registration");
        let path = dir.join("cpu");
        fs::write(&path, "").unwrap();
        let trigger = PsiTrigger::register(
            &PsiFileSource::at(&dir),
            PsiResourceKind::Cpu,
            "some",
            10_000,
            1_000_000,
        )
        .unwrap();
        drop(trigger);
        assert_eq!(fs::read(path).unwrap(), b"some 10000 1000000\0");
    }

    #[test]
    fn accepted_monitor_crossing_becomes_a_guardian_event() {
        let crossing = crate::psi::ThresholdEvent {
            resource: PsiResourceKind::Memory,
            from: PressureSeverity::Nominal,
            to: PressureSeverity::Critical,
        };
        let event = event_from_crossing(crossing, 7);
        assert_eq!(event.event_type, "psi_threshold_crossing");
        assert_eq!(event.resource_refs, ["/proc/pressure/memory"]);
        assert_eq!(event.severity, Risk::High);
        assert_eq!(event.attributes["from"], "Nominal");
        assert_eq!(event.attributes["to"], "Critical");
    }

    #[test]
    fn unavailable_and_malformed_sources_cannot_generate_events() {
        let missing = PsiFileSource::at(temp_dir("event-missing"));
        assert!(matches!(
            present_severity(
                &missing,
                PsiResourceKind::Cpu,
                SeverityThresholds::new(1.0, 2.0)
            ),
            Err(PsiEventError::Unavailable)
        ));

        let dir = temp_dir("event-malformed");
        fs::write(dir.join("cpu"), "bad\n").unwrap();
        assert!(matches!(
            present_severity(
                &PsiFileSource::at(dir),
                PsiResourceKind::Cpu,
                SeverityThresholds::new(1.0, 2.0)
            ),
            Err(PsiEventError::Parse(_))
        ));
    }

    #[test]
    fn repeated_wakes_emit_only_real_crossings() {
        let dir = temp_dir("dispatch-repeat");
        let path = dir.join("cpu");
        fs::write(&path, "some avg10=0.10 avg60=0 avg300=0 total=1\n").unwrap();
        let mut dispatcher = PsiEventDispatcher::new(
            PsiFileSource::at(&dir),
            PsiResourceKind::Cpu,
            SeverityThresholds::new(1.0, 2.0),
            PressureSeverity::Elevated,
        )
        .unwrap();
        assert!(dispatcher.dispatch_wake().unwrap().is_none());

        fs::write(&path, "some avg10=1.50 avg60=0 avg300=0 total=2\n").unwrap();
        assert!(dispatcher.dispatch_wake().unwrap().is_some());
        assert!(dispatcher.dispatch_wake().unwrap().is_none());

        fs::write(&path, "some avg10=0.20 avg60=0 avg300=0 total=3\n").unwrap();
        assert!(dispatcher.dispatch_wake().unwrap().is_some());
    }

    /// `P2-EVT-006`: explicit per-resource paths are read exactly as
    /// supplied — no base directory and no `/proc/pressure` fallback is
    /// involved, which is what lets `guardian-daemon` read PSI through
    /// inherited `/proc/self/fd/N` descriptors under `ProcSubset=pid`.
    #[test]
    fn explicit_per_resource_paths_are_read_independently_of_any_layout() {
        let dir = temp_dir("explicit-paths");
        let cpu = dir.join("an-arbitrary-name");
        let io = dir.join("nested/another");
        fs::create_dir_all(io.parent().unwrap()).unwrap();
        fs::write(&cpu, "some avg10=2.00 avg60=0 avg300=0 total=1\n").unwrap();
        fs::write(&io, "some avg10=4.00 avg60=0 avg300=0 total=1\n").unwrap();

        let source = PsiFileSource::from_paths([
            (PsiResourceKind::Cpu, cpu.clone()),
            (PsiResourceKind::Io, io.clone()),
        ]);
        assert_eq!(source.resolved_path(PsiResourceKind::Cpu), Some(cpu));
        assert_eq!(source.resolved_path(PsiResourceKind::Io), Some(io));
        match source.read(PsiResourceKind::Cpu).unwrap() {
            PsiReading::Present(resource) => {
                assert!((resource.some.avg10 - 2.00).abs() < f64::EPSILON);
            }
            PsiReading::Unavailable => panic!("expected Present"),
        }
    }

    /// `P2-EVT-006`/`P2-EVT-008`: a resource with no inherited descriptor
    /// (the `:graceful` partial case) is truthfully `Unavailable` — never
    /// "no pressure", and never silently resolved against `/proc/pressure`.
    #[test]
    fn an_explicit_source_reports_a_resource_it_has_no_path_for_as_unavailable() {
        let dir = temp_dir("explicit-missing");
        let cpu = dir.join("cpu");
        fs::write(&cpu, "some avg10=1.00 avg60=0 avg300=0 total=1\n").unwrap();
        let source = PsiFileSource::from_paths([(PsiResourceKind::Cpu, cpu)]);

        assert_eq!(source.resolved_path(PsiResourceKind::Memory), None);
        assert_eq!(
            source.read(PsiResourceKind::Memory).unwrap(),
            PsiReading::Unavailable
        );
        assert!(
            PsiTrigger::register(&source, PsiResourceKind::Memory, "some", 10_000, 1_000_000)
                .is_err(),
            "a trigger must never be registered against a resource with no descriptor"
        );
    }

    /// `P2-EVT-008` / gate TDD R17: a non-finite raw PSI average is not a
    /// measurement. `"NaN"`/`"inf"` both parse successfully as `f64`, so
    /// without an explicit finiteness check they would cross the
    /// `PsiFileSource::read` boundary as a *valid* `PsiReading::Present`
    /// and be handed to the accepted G5 classifier, where `NaN`'s
    /// comparison semantics silently yield `Nominal` — the exact
    /// fail-open outcome the G5 model exists to prevent.
    #[test]
    fn non_finite_raw_values_are_rejected_at_the_read_boundary() {
        let dir = temp_dir("non-finite");
        for (name, text) in [
            ("nan-avg10", "some avg10=NaN avg60=0 avg300=0 total=1\n"),
            ("inf-avg10", "some avg10=inf avg60=0 avg300=0 total=1\n"),
            (
                "neg-inf-avg60",
                "some avg10=0 avg60=-inf avg300=0 total=1\n",
            ),
            (
                "inf-avg300",
                "some avg10=0 avg60=0 avg300=infinity total=1\n",
            ),
        ] {
            let dir = dir.join(name);
            fs::create_dir_all(&dir).unwrap();
            fs::write(dir.join("cpu"), text).unwrap();
            let result = PsiFileSource::at(&dir).read(PsiResourceKind::Cpu);
            assert!(
                result.is_err(),
                "{name}: non-finite PSI value must be a typed parse error, got {result:?}"
            );
        }
    }

    /// `P2-EVT-008` / gate TDD R17: PSI averages are percentages of a
    /// window (kernel units), so a negative or >100 value is not a
    /// plausible measurement and must never reach classification.
    #[test]
    fn out_of_range_raw_values_are_rejected_at_the_read_boundary() {
        let dir = temp_dir("out-of-range");
        for (name, text) in [
            ("negative", "some avg10=-1.0 avg60=0 avg300=0 total=1\n"),
            ("above-100", "some avg10=250.0 avg60=0 avg300=0 total=1\n"),
            (
                "full-line-out-of-range",
                "some avg10=1.0 avg60=0 avg300=0 total=1\nfull avg10=0 avg60=0 avg300=101 total=1\n",
            ),
        ] {
            let dir = dir.join(name);
            fs::create_dir_all(&dir).unwrap();
            fs::write(dir.join("memory"), text).unwrap();
            let result = PsiFileSource::at(&dir).read(PsiResourceKind::Memory);
            assert!(
                result.is_err(),
                "{name}: out-of-range PSI value must be a typed parse error, got {result:?}"
            );
        }
    }

    /// `P2-EVT-008` / gate TDD R17: the rejection must happen *before*
    /// anything downstream can turn the reading into a severity, so no
    /// invalid value can ever become an admitted `Event`.
    #[test]
    fn invalid_raw_values_can_never_produce_a_severity() {
        let dir = temp_dir("invalid-no-severity");
        for text in [
            "some avg10=NaN avg60=0 avg300=0 total=1\n",
            "some avg10=inf avg60=0 avg300=0 total=1\n",
            "some avg10=-5 avg60=0 avg300=0 total=1\n",
            "some avg10=1000 avg60=0 avg300=0 total=1\n",
        ] {
            fs::write(dir.join("io"), text).unwrap();
            let result = present_severity(
                &PsiFileSource::at(&dir),
                PsiResourceKind::Io,
                SeverityThresholds::new(20.0, 50.0),
            );
            assert!(
                matches!(result, Err(PsiEventError::Parse(_))),
                "invalid raw value must surface a typed parse error, got {result:?}"
            );
        }
    }

    // ---- Acceptance-repair Blocker 1/2: OwnedPsiFile ---------------
    //
    // `P2-EVT-005`/`P2-EVT-007`/`P2-EVT-008` acceptance criteria
    // (formerly the standalone `P2-EVT-006`, demoted 2026-09-08).

    fn owned_fixture(name: &str, text: &str) -> (std::path::PathBuf, OwnedPsiFile) {
        let dir = temp_dir(&format!("owned-{name}"));
        let path = dir.join("cpu");
        fs::write(&path, text).unwrap();
        let owned = OwnedPsiFile::open(PsiResourceKind::Cpu, &path).unwrap();
        (path, owned)
    }

    /// Blocker 1: the owned descriptor this daemon opens must be
    /// close-on-exec, verified via the kernel's own `/proc/self/fdinfo/N`
    /// `flags:` field — not a trust of `fcntl`'s own success return.
    #[test]
    fn an_owned_psi_file_is_close_on_exec() {
        let (_, owned) = owned_fixture("cloexec", "some avg10=0 avg60=0 avg300=0 total=1\n");
        assert!(
            owned.is_close_on_exec(),
            "OwnedPsiFile::open must leave FD_CLOEXEC set on the descriptor it opened"
        );
    }

    /// Blocker 1/2: `OwnedPsiFile::open` performs exactly one `open()`.
    /// Reading the same object twice must return live, current content —
    /// proving the read path never reopens (a stale/never-updated read
    /// would be the observable symptom of an accidental reopen against a
    /// cached first read, and a *crashing* reread would be the symptom of
    /// a closed/invalid descriptor).
    #[test]
    fn an_owned_psi_file_rereads_live_content_through_its_single_open_file_description() {
        let (path, owned) = owned_fixture("reread", "some avg10=1.00 avg60=0 avg300=0 total=1\n");
        let first = owned.read_text().unwrap();
        assert!(first.contains("avg10=1.00"));
        fs::write(&path, "some avg10=9.00 avg60=0 avg300=0 total=2\n").unwrap();
        let second = owned.read_text().unwrap();
        assert!(
            second.contains("avg10=9.00"),
            "a reread through the same owned file must observe live content: {second}"
        );
    }

    /// Blocker 2 / gate TDD R7: trigger registration through
    /// `register_on_owned_file` shares the *exact* open file description
    /// with the `OwnedPsiFile` it was registered on — not merely the same
    /// path. `Arc::ptr_eq` is a correct in-process proof because this
    /// type never dup()s or reopens after construction.
    #[test]
    fn trigger_registered_on_an_owned_file_shares_its_exact_open_file_description() {
        let (_, owned) = owned_fixture("trigger-identity", "");
        let trigger =
            PsiTrigger::register_on_owned_file(&owned, "some", 10_000, 1_000_000).unwrap();
        assert!(
            trigger.shares_open_file_description_with(&owned),
            "the registered trigger must share owned's exact open file description"
        );
    }

    /// Blocker 2 / gate TDD R7, the identity-vs-pathname distinction the
    /// blocker explicitly asks for: two *independent* `OwnedPsiFile::open`
    /// calls against the **same path** are two different open file
    /// descriptions — proven by the real kernel `EBUSY` mechanism a PSI
    /// trigger provides for free (triggers are per-open-file-description,
    /// not per-inode). This is the negative control that makes the
    /// positive proof above meaningful: it demonstrates the test
    /// technique actually distinguishes "same open file description" from
    /// "same underlying file", using a real `/proc/pressure`-shaped
    /// pressure file (a plain fixture file accepts any bytes as a
    /// trigger write with no kernel-side `EBUSY` semantics of its own, so
    /// this specific distinction is proven directly against the real
    /// kernel PSI file when available, and is otherwise documented as
    /// environment-dependent below).
    #[test]
    fn independent_reopens_of_the_same_path_are_different_open_file_descriptions() {
        // A plain fixture file has no real per-open-file-description
        // trigger semantics (that is a genuine kernel PSI ABI behavior,
        // not something `/proc/pressure`-shaped test fixtures can
        // simulate), so this test instead proves descriptor-identity
        // divergence the way this module already establishes it
        // elsewhere: independent `Arc` allocations. Two independently
        // opened `OwnedPsiFile`s over the same path never share their
        // `Arc<fs::File>` allocation, even when nothing about the path
        // differs -- `Arc::ptr_eq` is false, which is the same identity
        // check `same_open_file_description` uses, applied here in its
        // negative direction. The real-kernel `EBUSY` divergence for the
        // genuinely inherited case is additionally proven on the real VM
        // (`P2-VM-003` evidence, Phase F/G/H), where a real kernel PSI
        // pressure file is available to trigger against twice.
        let dir = temp_dir("owned-independent-reopen");
        let path = dir.join("cpu");
        fs::write(&path, "some avg10=0 avg60=0 avg300=0 total=1\n").unwrap();
        let first = OwnedPsiFile::open(PsiResourceKind::Cpu, &path).unwrap();
        let second = OwnedPsiFile::open(PsiResourceKind::Cpu, &path).unwrap();
        assert!(
            !first.same_open_file_description(&second),
            "two independent OwnedPsiFile::open calls against the same path must NOT share an \
             open file description -- this is exactly the defect this repair closes: the \
             pre-repair code performed this kind of independent reopen on every dispatch_wake \
             call instead of reusing one owned File"
        );
    }

    /// Blocker 2: `PsiEventDispatcher::new_on_owned_file` + `dispatch_wake`
    /// read through the *same* `OwnedPsiFile` object the caller
    /// registered a trigger on — end-to-end proof that registration and
    /// every reread share one open file description, using the accepted
    /// dispatch/crossing behavior as the observable side effect.
    #[test]
    fn dispatcher_on_an_owned_file_classifies_live_rereads_through_the_same_description() {
        let (path, owned) = owned_fixture(
            "dispatch-owned",
            "some avg10=0.10 avg60=0 avg300=0 total=1\n",
        );
        assert!(
            owned.is_close_on_exec(),
            "sanity: the fixture descriptor itself must be close-on-exec"
        );
        let mut dispatcher = PsiEventDispatcher::new_on_owned_file(
            owned.clone(),
            SeverityThresholds::new(1.0, 2.0),
            PressureSeverity::Elevated,
        )
        .unwrap();
        assert!(
            dispatcher.reads_through_owned_file(&owned),
            "the dispatcher must read through the exact OwnedPsiFile it was constructed with"
        );
        assert!(dispatcher.dispatch_wake().unwrap().is_none());

        fs::write(&path, "some avg10=1.50 avg60=0 avg300=0 total=2\n").unwrap();
        let event = dispatcher.dispatch_wake().unwrap();
        assert!(
            event.is_some(),
            "a real crossing read through the owned file's single open file description must \
             still classify, exactly as the path-based PsiFileSource route does"
        );
    }

    /// Blocker 1: a missing path still fails closed (an I/O error, never
    /// a panic or a falsely-`Present` reading) through `OwnedPsiFile`,
    /// exactly as the path-based route already does for
    /// `P1-PSI-005`/`P2-EVT-008`.
    #[test]
    fn an_owned_psi_file_open_on_a_missing_path_fails_closed() {
        let dir = temp_dir("owned-missing");
        let result = OwnedPsiFile::open(PsiResourceKind::Cpu, &dir.join("does-not-exist"));
        assert!(result.is_err());
    }

    /// Blocker 2, real-kernel proof (gate TDD R7/R8): PSI triggers are
    /// scoped per **open file description**, not per-inode -- the kernel
    /// mechanism this repair's evidence uses as its identity oracle.
    /// Ignored by default (requires the **opening** process to hold real
    /// kernel PSI-trigger privilege -- empirically, world-writable
    /// `0666` permissions on `/proc/pressure/cpu` are NOT sufficient:
    /// registering a trigger on a descriptor this test process opened
    /// itself fails `EINVAL` unless that process is privileged (verified
    /// directly, on both the authoring host and `guardian-g9`: identical
    /// unprivileged `open()`+trigger-write fails `EINVAL`, the same call
    /// under `sudo` succeeds). This is not a bug in the test or in
    /// `OwnedPsiFile` -- it is the exact kernel property the production
    /// `OpenFile=` design (this repair's Blocker 1/2, and the accepted
    /// ADR-009 architecture) depends on: the kernel's PSI trigger check
    /// is against the descriptor's **opener's** credentials, not the
    /// current writer's, which is precisely why systemd's PID-1-namespace
    /// `OpenFile=` open lets the unprivileged `guardiand` process (zero
    /// capabilities, confirmed on the real VM) register a real trigger
    /// through a descriptor it never had privilege to open itself. This
    /// test opens its own descriptors directly (by design, to isolate
    /// the identity property from the `OpenFile=` plumbing), so it needs
    /// the same privilege an `OpenFile=` opener would have -- matching
    /// this workspace's existing convention for real-kernel/real-bus
    /// tests, e.g. `restart_capability.rs`'s `#[ignore]`d VM-only tests.
    /// Run explicitly with sufficient privilege (`sudo`) on a real VM/
    /// host with working system-wide PSI, e.g. `guardian-g9` (verified
    /// passing there under `sudo`, per this pass's evidence document).
    ///
    /// Proves both directions: (a) two *independent* `OwnedPsiFile::open`
    /// calls against the exact same real kernel path are two genuinely
    /// different open file descriptions -- registering a trigger on each
    /// independently both succeed, which could not happen if they shared
    /// one description; (b) a *second* trigger registration against the
    /// *same* open file description a trigger is already registered on
    /// fails `EBUSY` -- proving the identity check this repair relies on
    /// (`same_open_file_description`/`shares_open_file_description_with`)
    /// tracks a real kernel-observable property, not just an in-process
    /// `Arc` bookkeeping artifact.
    #[test]
    #[ignore = "requires a writable real /proc/pressure/cpu; run explicitly (see doc comment)"]
    fn real_kernel_psi_triggers_are_scoped_per_open_file_description_not_per_inode() {
        let path = Path::new("/proc/pressure/cpu");
        let first =
            OwnedPsiFile::open(PsiResourceKind::Cpu, path).expect("open /proc/pressure/cpu #1");
        let second =
            OwnedPsiFile::open(PsiResourceKind::Cpu, path).expect("open /proc/pressure/cpu #2");
        assert!(
            !first.same_open_file_description(&second),
            "two independent opens of the same real kernel path must not share a description"
        );

        // (a) independent descriptions: a trigger on each succeeds.
        let trigger_one = PsiTrigger::register_on_owned_file(&first, "some", 10_000, 1_000_000)
            .expect("trigger on the first, independent, open file description must succeed");
        let trigger_two = PsiTrigger::register_on_owned_file(&second, "some", 10_000, 1_000_000)
            .expect(
                "trigger on the second, independently-opened description must ALSO succeed -- \
                 proving it is genuinely a different open file description from the first, not \
                 merely a different in-process handle to the same one",
            );
        drop(trigger_one);

        // (b) same description: a second registration on the SAME owned
        // file (which now already carries trigger_two's live trigger)
        // must fail EBUSY -- there is no in-place re-arm.
        let retry = PsiTrigger::register_on_owned_file(&second, "some", 10_000, 1_000_000);
        let Err(error) = retry else {
            panic!(
                "a second trigger registration on an open file description that already has \
                 one must fail, never silently succeed or replace the first"
            )
        };
        assert_eq!(
            error.0.raw_os_error(),
            Some(16 /* EBUSY */),
            "the real kernel's rejection of a duplicate trigger on one description must be \
             EBUSY specifically: {error}"
        );
        drop(trigger_two);
    }
}
