//! Linux raw MIDI, with one pinned destination and a bounded byte queue.
//!
//! No ALSA userspace API is required. Device names and direction hints come
//! from procfs; the actual open is the final test of writability.

use std::collections::VecDeque;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::os::unix::fs::{FileTypeExt, OpenOptionsExt};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use super::Message;
use super::clock::TRANSPORT_LEAD;

const RESCAN: Duration = Duration::from_secs(1);
const MAX_AGE: Duration = Duration::from_millis(30);
const QUEUE_CAPACITY: usize = 128;
// Linux x86_64 and aarch64 O_NONBLOCK.
const O_NONBLOCK: i32 = 0x800;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct Identity {
    card_id: String,
    device: u32,
    usb_id: String,
    location: PathBuf,
}

#[derive(Debug)]
pub(super) struct RawPort {
    path: PathBuf,
    identity: Identity,
    output: bool,
    input: bool,
    directions_known: bool,
    product: String,
    output_owner: Option<u32>,
}

impl RawPort {
    fn label(&self) -> String {
        if self.product.is_empty() || self.product == self.identity.card_id {
            format!("{} ({})", self.identity.card_id, self.path.display())
        } else {
            format!(
                "{} [{}] ({})",
                self.product,
                self.identity.card_id,
                self.path.display()
            )
        }
    }

    fn kind(&self) -> &'static str {
        if !self.identity.usb_id.is_empty() {
            "USB"
        } else if self
            .identity
            .location
            .to_string_lossy()
            .contains("/virtual/")
            || self
                .identity
                .card_id
                .to_ascii_lowercase()
                .starts_with("virmidi")
        {
            "VIRTUAL"
        } else {
            "OTHER / UNKNOWN"
        }
    }

    fn direction_label(&self) -> &'static str {
        if !self.directions_known {
            "UNKNOWN"
        } else if self.output && self.input {
            "IN / OUT"
        } else if self.output {
            "OUT"
        } else {
            "INPUT ONLY"
        }
    }
}

/// ALSA procfs reports each substream separately. Owner PID in an input
/// section must not be reported as the output owner.
fn port_details(text: &str) -> (bool, bool, Option<u32>) {
    let mut output = false;
    let mut input = false;
    let mut in_output = false;
    let mut owner = None;
    for line in text.lines().map(str::trim) {
        if line.starts_with("Output ") {
            output = true;
            in_output = true;
        } else if line.starts_with("Input ") {
            input = true;
            in_output = false;
        } else if in_output
            && owner.is_none()
            && let Some((key, value)) = line.split_once(':')
            && key.trim() == "Owner PID"
        {
            owner = value.trim().parse().ok();
        }
    }
    (output, input, owner)
}

fn read_trimmed(path: impl AsRef<Path>) -> String {
    fs::read_to_string(path)
        .map(|value| value.trim().to_owned())
        .unwrap_or_default()
}

fn device_numbers(name: &str) -> Option<(u32, u32)> {
    let (card, device) = name.strip_prefix("midiC")?.split_once('D')?;
    if !card.bytes().all(|byte| byte.is_ascii_digit())
        || !device.bytes().all(|byte| byte.is_ascii_digit())
    {
        return None;
    }
    Some((card.parse().ok()?, device.parse().ok()?))
}

pub(super) fn discover() -> io::Result<Vec<RawPort>> {
    let mut ports = Vec::new();
    for entry in fs::read_dir("/dev/snd")?.filter_map(Result::ok) {
        let path = entry.path();
        let name = entry.file_name();
        let Some((card, device)) = device_numbers(&name.to_string_lossy()) else {
            continue;
        };
        if !fs::metadata(&path).is_ok_and(|meta| meta.file_type().is_char_device()) {
            continue;
        }
        let card_dir = format!("/proc/asound/card{card}");
        let mut card_id = read_trimmed(format!("{card_dir}/id"));
        if card_id.is_empty() {
            card_id = format!("CARD {card}");
        }
        let directions = read_trimmed(format!("{card_dir}/midi{device}"));
        // Unknown direction is not evidence of input-only. Try the open.
        let (has_output, input, output_owner) = port_details(&directions);
        let directions_known = has_output || input;
        let output = has_output || !directions_known;
        let product = directions
            .lines()
            .next()
            .unwrap_or_default()
            .trim()
            .to_owned();
        let usb_id = read_trimmed(format!("{card_dir}/usbid"));
        let location = fs::canonicalize(format!("/sys/class/sound/card{card}/device"))
            .or_else(|_| fs::canonicalize(format!("/sys/class/sound/card{card}")))
            .unwrap_or_else(|_| path.clone());
        ports.push(RawPort {
            path,
            identity: Identity {
                card_id,
                device,
                usb_id,
                location,
            },
            output,
            input,
            directions_known,
            product,
            output_owner,
        });
    }
    ports.sort_by(|left, right| left.path.cmp(&right.path));
    Ok(ports)
}

pub(super) fn list() {
    match discover() {
        Ok(ports) => {
            println!("Raw MIDI device nodes: {}", ports.len());
            for port in ports {
                println!(
                    "  {} [{}] {} {}{}",
                    port.label(),
                    port.direction_label(),
                    port.kind(),
                    port.identity.usb_id,
                    port.output_owner
                        .map_or_else(String::new, |pid| format!(" owner PID {pid}"))
                );
            }
        }
        Err(error) => eprintln!("Cannot enumerate /dev/snd: {error}"),
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub(super) enum Selection {
    Auto,
    Name(String),
    Path(PathBuf),
    Identity(Identity),
    Off,
}

fn select<'a>(
    ports: &'a [RawPort],
    selection: &Selection,
    pinned: Option<&Identity>,
) -> Result<&'a RawPort, &'static str> {
    let prefer_usb = pinned.is_none()
        && matches!(selection, Selection::Auto)
        && ports.iter().any(|port| port.output && port.kind() == "USB");
    let mut matches = ports.iter().filter(|port| {
        port.output
            && if let Some(identity) = pinned {
                &port.identity == identity
            } else {
                match selection {
                    Selection::Auto => !prefer_usb || port.kind() == "USB",
                    Selection::Name(name) => port.label().to_lowercase().contains(name),
                    Selection::Path(path) => &port.path == path,
                    Selection::Identity(identity) => &port.identity == identity,
                    Selection::Off => false,
                }
            }
    });
    let first = matches.next().ok_or("NO MATCH")?;
    if matches.next().is_some() {
        return Err("AMBIGUOUS - SELECT PORT");
    }
    Ok(first)
}

#[derive(Debug)]
struct Pending {
    message: Message,
    offset: usize,
    queued_at: Instant,
}

/// Drain a finite queue in order; never resend bytes already accepted by the
/// kernel. Backpressure leaves the remaining bytes queued for the next poll.
fn drain(
    writer: &mut impl Write,
    queue: &mut VecDeque<Pending>,
    now: Instant,
    sent_bytes: &mut u64,
    first_clock_after: &mut Option<Instant>,
) -> io::Result<()> {
    while let Some(message) = queue.front_mut() {
        if now.saturating_duration_since(message.queued_at) >= MAX_AGE {
            return Err(io::Error::new(
                io::ErrorKind::TimedOut,
                "MIDI queue expired",
            ));
        }
        if message.message.is_clock() && first_clock_after.is_some_and(|after| now < after) {
            return Ok(());
        }
        let bytes = message.message.bytes();
        match writer.write(&bytes[message.offset..]) {
            Ok(0) => return Err(io::ErrorKind::WriteZero.into()),
            Ok(count) => {
                message.offset += count;
                *sent_bytes = sent_bytes.saturating_add(count as u64);
                if message.offset == bytes.len() {
                    if message.message.starts_clock() {
                        // Backpressure can delay Start/Continue beyond its
                        // scheduled send time. Leave the gap after the write.
                        *first_clock_after = Some(Instant::now() + TRANSPORT_LEAD);
                    } else if message.message.is_clock() {
                        *first_clock_after = None;
                    }
                    let _ = queue.pop_front();
                }
            }
            Err(error)
                if matches!(
                    error.kind(),
                    io::ErrorKind::WouldBlock | io::ErrorKind::Interrupted
                ) =>
            {
                return Ok(());
            }
            Err(error) => return Err(error),
        }
    }
    Ok(())
}

fn queue_message(
    queue: &mut VecDeque<Pending>,
    message: Message,
    now: Instant,
) -> Result<bool, String> {
    // Keep only the newest pending clock during backpressure. A one-byte
    // clock cannot be partially written, so removing it preserves framing.
    let mut skipped_clock = false;
    if message.is_clock()
        && let Some(index) = queue.iter().position(|pending| pending.message.is_clock())
    {
        queue.remove(index);
        skipped_clock = true;
    }
    if queue.len() >= QUEUE_CAPACITY {
        return Err("Raw MIDI queue full".to_owned());
    }
    queue.push_back(Pending {
        message,
        offset: 0,
        queued_at: now,
    });
    Ok(skipped_clock)
}

pub(super) struct RawOutput {
    selection: Selection,
    pinned: Option<Identity>,
    file: Option<File>,
    bound_path: Option<PathBuf>,
    queue: VecDeque<Pending>,
    next_scan: Instant,
    label: String,
    needs_cleanup: bool,
    first_clock_after: Option<Instant>,
    sent_bytes: u64,
    faults: u64,
}

impl RawOutput {
    pub(super) fn new(selection: Selection) -> Self {
        let mut output = Self {
            selection,
            pinned: None,
            file: None,
            bound_path: None,
            queue: VecDeque::with_capacity(QUEUE_CAPACITY),
            next_scan: Instant::now(),
            label: "RAW WAIT".to_owned(),
            needs_cleanup: false,
            first_clock_after: None,
            sent_bytes: 0,
            faults: 0,
        };
        output.service(Instant::now());
        output
    }

    pub(super) fn connected(&self) -> bool {
        self.file.is_some()
    }

    pub(super) fn label(&self) -> &str {
        &self.label
    }

    fn set_label(&mut self, label: String) {
        if self.label != label {
            println!("MIDI: {label}");
            self.label = label;
        }
    }

    pub(super) fn enqueue(&mut self, message: Message) -> Result<bool, String> {
        if !self.connected() {
            return Err("Raw MIDI disconnected".to_owned());
        }
        queue_message(&mut self.queue, message, Instant::now())
    }

    /// Called only after the MIDI engine has flushed owned note releases.
    pub(super) fn cycle(&mut self, direction: i32) {
        let ports = match discover() {
            Ok(ports) => ports,
            Err(error) => {
                eprintln!("MIDI selection scan failed: {error}");
                Vec::new()
            }
        };
        let mut choices = vec![Selection::Auto];
        choices.extend(
            ports
                .iter()
                .filter(|port| port.output)
                .map(|port| Selection::Identity(port.identity.clone())),
        );
        choices.push(Selection::Off);
        let current = choices
            .iter()
            .position(|choice| choice == &self.selection)
            .or_else(|| {
                self.pinned.as_ref().and_then(|identity| {
                    choices
                        .iter()
                        .position(|choice| choice == &Selection::Identity(identity.clone()))
                })
            })
            .unwrap_or(0);
        let next = if direction > 0 {
            (current + 1) % choices.len()
        } else {
            (current + choices.len() - 1) % choices.len()
        };
        if self.needs_cleanup {
            eprintln!("MIDI: previous device may require external panic after disconnect");
        }
        self.file = None;
        self.bound_path = None;
        self.queue.clear();
        self.first_clock_after = None;
        self.pinned = None;
        self.needs_cleanup = false;
        self.selection = choices.swap_remove(next);
        self.next_scan = Instant::now();
        if matches!(self.selection, Selection::Off) {
            self.set_label("RAW OFF".to_owned());
        } else {
            self.service(Instant::now());
        }
    }

    fn bind(&mut self, now: Instant) {
        self.next_scan = now + RESCAN;
        let ports = match discover() {
            Ok(ports) => ports,
            Err(error) => {
                self.set_label(format!("RAW WAIT: {error}"));
                return;
            }
        };
        let port = match select(&ports, &self.selection, self.pinned.as_ref()) {
            Ok(port) => port,
            Err(reason) => {
                let target = match &self.selection {
                    Selection::Auto => "AUTO".to_owned(),
                    Selection::Name(name) => name.clone(),
                    Selection::Path(path) => path.display().to_string(),
                    Selection::Identity(identity) => identity.card_id.clone(),
                    Selection::Off => "OFF".to_owned(),
                };
                self.set_label(format!("RAW WAIT {target}: {reason}"));
                return;
            }
        };
        match OpenOptions::new()
            .write(true)
            .custom_flags(O_NONBLOCK)
            .open(&port.path)
        {
            Ok(file) => {
                self.pinned = Some(port.identity.clone());
                self.file = Some(file);
                self.bound_path = Some(port.path.clone());
                let mode = if matches!(self.selection, Selection::Auto) {
                    "AUTO"
                } else {
                    "SELECT"
                };
                self.set_label(format!("RAW {mode}: {}", port.label()));
                // Only after a fault, on the previously pinned device. Clear
                // potentially orphaned notes on Khaṇa's four channels before
                // sending any new attacks. Never replay old musical events.
                if self.needs_cleanup {
                    for channel in 0_u8..4 {
                        self.queue.push_back(Pending {
                            message: Message::Channel([0xB0 | channel, 120, 0]),
                            offset: 0,
                            queued_at: now,
                        });
                    }
                    self.needs_cleanup = false;
                }
            }
            Err(error) => {
                let owner = port
                    .output_owner
                    .map_or_else(String::new, |pid| format!(" (owner PID {pid})"));
                self.set_label(format!("RAW WAIT {}: {error}{owner}", port.label()));
            }
        }
    }

    pub(super) fn fault(&mut self, error: &str) {
        self.file = None;
        self.bound_path = None;
        self.queue.clear();
        self.first_clock_after = None;
        self.next_scan = Instant::now() + RESCAN;
        self.needs_cleanup = true;
        self.faults = self.faults.saturating_add(1);
        self.set_label(format!("RAW LOST: {error}"));
    }

    pub(super) fn service(&mut self, now: Instant) {
        if matches!(self.selection, Selection::Off) {
            return;
        }
        if now >= self.next_scan {
            self.next_scan = now + RESCAN;
            if self.connected() {
                // Detect unplug even during silence. Never switch to another
                // card if the original identity disappears.
                let present = discover().is_ok_and(|ports| {
                    ports.iter().any(|port| {
                        Some(&port.identity) == self.pinned.as_ref()
                            && Some(&port.path) == self.bound_path.as_ref()
                    })
                });
                if !present {
                    self.fault("device removed or changed");
                }
            } else {
                self.bind(now);
            }
        }
        if let Some(file) = &mut self.file
            && let Err(error) = drain(
                file,
                &mut self.queue,
                now,
                &mut self.sent_bytes,
                &mut self.first_clock_after,
            )
        {
            self.fault(&error.to_string());
        }
    }

    /// Bounded cleanup/audition flush; not used in the musical hot path.
    pub(super) fn flush(&mut self) -> Result<(), String> {
        let deadline = Instant::now() + MAX_AGE;
        while self.connected() && !self.queue.is_empty() && Instant::now() < deadline {
            self.service(Instant::now());
            if !self.queue.is_empty() {
                std::thread::sleep(Duration::from_millis(1));
            }
        }
        if !self.connected() {
            return Err(self.label.clone());
        }
        if !self.queue.is_empty() {
            self.fault("flush timeout");
            return Err(self.label.clone());
        }
        Ok(())
    }

    pub(super) fn report(&self) {
        println!(
            "RAW MIDI | {} | bytes={} queued={} faults={}",
            self.label,
            self.sent_bytes,
            self.queue.len(),
            self.faults
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct ShortWriter {
        bytes: Vec<u8>,
        block_next: bool,
    }

    impl Write for ShortWriter {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            if self.block_next {
                self.block_next = false;
                return Err(io::ErrorKind::WouldBlock.into());
            }
            self.bytes.push(buf[0]);
            self.block_next = true;
            Ok(1)
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    #[test]
    fn short_writes_and_backpressure_preserve_byte_order() {
        let now = Instant::now();
        let mut queue = VecDeque::new();
        for message in [
            Message::Realtime(0xFA),
            Message::Channel([0x90, 60, 100]),
            Message::Realtime(0xF8),
            Message::Channel([0x80, 60, 0]),
        ] {
            assert!(!queue_message(&mut queue, message, now).unwrap());
        }
        assert!(queue_message(&mut queue, Message::Realtime(0xF8), now).unwrap());
        assert!(!queue_message(&mut queue, Message::Realtime(0xFC), now).unwrap());
        let mut writer = ShortWriter {
            bytes: Vec::new(),
            block_next: false,
        };
        let mut sent = 0;
        let mut first_clock_after = None;
        for _ in 0..9 {
            let drain_at = first_clock_after.unwrap_or(now);
            drain(
                &mut writer,
                &mut queue,
                drain_at,
                &mut sent,
                &mut first_clock_after,
            )
            .unwrap();
        }
        assert!(queue.is_empty());
        assert_eq!(writer.bytes, [0xFA, 0x90, 60, 100, 0x80, 60, 0, 0xF8, 0xFC]);
        assert_eq!(sent, 9);
    }

    #[test]
    fn first_clock_waits_after_a_delayed_transport_write() {
        let now = Instant::now();
        let mut queue = VecDeque::new();
        queue_message(&mut queue, Message::Realtime(0xFB), now).unwrap();
        queue_message(&mut queue, Message::Realtime(0xF8), now).unwrap();
        let mut sent = 0;
        let mut first_clock_after = None;
        let mut bytes = Vec::new();
        drain(
            &mut bytes,
            &mut queue,
            now,
            &mut sent,
            &mut first_clock_after,
        )
        .unwrap();
        assert_eq!(bytes, [0xFB]);
        let ready = first_clock_after.unwrap();
        drain(
            &mut bytes,
            &mut queue,
            ready,
            &mut sent,
            &mut first_clock_after,
        )
        .unwrap();
        assert_eq!(bytes, [0xFB, 0xF8]);
    }

    #[test]
    fn stale_queue_is_not_replayed() {
        let now = Instant::now();
        let mut queue = VecDeque::from([Pending {
            message: Message::Channel([0x90, 60, 100]),
            offset: 0,
            queued_at: now,
        }]);
        let mut sent = 0;
        assert_eq!(
            drain(
                &mut Vec::new(),
                &mut queue,
                now + MAX_AGE,
                &mut sent,
                &mut None
            )
            .unwrap_err()
            .kind(),
            io::ErrorKind::TimedOut
        );
        assert_eq!(sent, 0);
    }

    fn port(card: u32, id: &str, location: &str) -> RawPort {
        RawPort {
            path: PathBuf::from(format!("/dev/snd/midiC{card}D0")),
            identity: Identity {
                card_id: id.to_owned(),
                device: 0,
                usb_id: "1234:5678".to_owned(),
                location: PathBuf::from(location),
            },
            output: true,
            input: true,
            directions_known: true,
            product: String::new(),
            output_owner: None,
        }
    }

    #[test]
    fn automatic_selection_never_guesses_between_two_devices() {
        let ports = [port(1, "A", "/usb/1"), port(2, "B", "/usb/2")];
        assert!(select(&ports, &Selection::Auto, None).is_err());
        assert_eq!(
            select(&ports, &Selection::Name("b".to_owned()), None)
                .unwrap()
                .identity
                .card_id,
            "B"
        );
    }

    #[test]
    fn reconnect_matches_pinned_identity_not_reused_card_number() {
        let original = port(1, "A", "/usb/1");
        let ports = [port(1, "B", "/usb/2"), port(3, "A", "/usb/1")];
        let selected = select(&ports, &Selection::Auto, Some(&original.identity)).unwrap();
        assert_eq!(selected.path, PathBuf::from("/dev/snd/midiC3D0"));
        assert!(select(&ports[..1], &Selection::Auto, Some(&original.identity)).is_err());
    }

    #[test]
    fn proc_directions_and_output_owner_are_reported() {
        let text = "QT-2host\n\nOutput 0\n  Owner PID    : 4091\nInput 0\n  Owner PID    : 22\n";
        assert_eq!(port_details(text), (true, true, Some(4091)));
        assert_eq!(
            port_details("Input 0\n Owner PID : 22"),
            (false, true, None)
        );
        assert_eq!(port_details(""), (false, false, None));
    }

    #[test]
    fn virtual_ports_are_not_usb_and_auto_prefers_physical_usb() {
        let mut virtual_port = port(0, "VirMIDI", "/devices/virtual/sound/card0");
        virtual_port.identity.usb_id.clear();
        assert_eq!(virtual_port.kind(), "VIRTUAL");
        let ports = [virtual_port, port(1, "QT2host", "/usb/1")];
        assert_eq!(
            select(&ports, &Selection::Auto, None)
                .unwrap()
                .identity
                .card_id,
            "QT2host"
        );
        assert!(select(&ports, &Selection::Off, None).is_err());
    }

    #[test]
    fn device_names_exclude_non_midi_nodes() {
        assert_eq!(device_numbers("midiC12D3"), Some((12, 3)));
        for name in ["seq", "controlC1", "pcmC1D0p", "midiCD0", "midiC1D0x"] {
            assert_eq!(device_numbers(name), None);
        }
    }
}
