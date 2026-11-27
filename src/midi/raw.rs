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

const RESCAN: Duration = Duration::from_secs(1);
const MAX_AGE: Duration = Duration::from_millis(30);
const QUEUE_CAPACITY: usize = 128;
// Linux x86_64 and aarch64 O_NONBLOCK.
const O_NONBLOCK: i32 = 0x800;

#[derive(Clone, Debug, Eq, PartialEq)]
struct Identity {
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
}

impl RawPort {
    fn label(&self) -> String {
        format!("{} ({})", self.identity.card_id, self.path.display())
    }
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
        let output = directions.contains("Output") || !directions.contains("Input");
        let usb_id = read_trimmed(format!("{card_dir}/usbid"));
        let location = fs::canonicalize(format!("/sys/class/sound/card{card}/device"))
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
                    "  {} [{}] USB {}",
                    port.label(),
                    if port.output {
                        "OUT / UNKNOWN"
                    } else {
                        "INPUT ONLY"
                    },
                    port.identity.usb_id
                );
            }
        }
        Err(error) => eprintln!("Cannot enumerate /dev/snd: {error}"),
    }
}

#[derive(Debug)]
pub(super) enum Selection {
    Auto,
    Name(String),
    Path(PathBuf),
}

fn select<'a>(
    ports: &'a [RawPort],
    selection: &Selection,
    pinned: Option<&Identity>,
) -> Result<&'a RawPort, &'static str> {
    let mut matches = ports.iter().filter(|port| {
        port.output
            && if let Some(identity) = pinned {
                &port.identity == identity
            } else {
                match selection {
                    Selection::Auto => true,
                    Selection::Name(name) => port.label().to_lowercase().contains(name),
                    Selection::Path(path) => &port.path == path,
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
    bytes: [u8; 3],
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
) -> io::Result<()> {
    while let Some(message) = queue.front_mut() {
        if now.saturating_duration_since(message.queued_at) >= MAX_AGE {
            return Err(io::Error::new(
                io::ErrorKind::TimedOut,
                "MIDI queue expired",
            ));
        }
        match writer.write(&message.bytes[message.offset..]) {
            Ok(0) => return Err(io::ErrorKind::WriteZero.into()),
            Ok(count) => {
                message.offset += count;
                *sent_bytes = sent_bytes.saturating_add(count as u64);
                if message.offset == message.bytes.len() {
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

pub(super) struct RawOutput {
    selection: Selection,
    pinned: Option<Identity>,
    file: Option<File>,
    bound_path: Option<PathBuf>,
    queue: VecDeque<Pending>,
    next_scan: Instant,
    label: String,
    needs_cleanup: bool,
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

    pub(super) fn enqueue(&mut self, bytes: [u8; 3]) -> Result<(), String> {
        if !self.connected() {
            return Err("Raw MIDI disconnected".to_owned());
        }
        if self.queue.len() >= QUEUE_CAPACITY {
            return Err("Raw MIDI queue full".to_owned());
        }
        self.queue.push_back(Pending {
            bytes,
            offset: 0,
            queued_at: Instant::now(),
        });
        Ok(())
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
                self.set_label(format!("RAW WAIT: {reason}"));
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
                self.set_label(format!("RAW: {}", port.label()));
                // Only after a fault, on the previously pinned device. Clear
                // potentially orphaned notes on Khaṇa's four channels before
                // sending any new attacks. Never replay old musical events.
                if self.needs_cleanup {
                    for channel in 0_u8..4 {
                        self.queue.push_back(Pending {
                            bytes: [0xB0 | channel, 120, 0],
                            offset: 0,
                            queued_at: now,
                        });
                    }
                    self.needs_cleanup = false;
                }
            }
            Err(error) => self.set_label(format!("RAW WAIT: {error}")),
        }
    }

    pub(super) fn fault(&mut self, error: &str) {
        self.file = None;
        self.bound_path = None;
        self.queue.clear();
        self.next_scan = Instant::now() + RESCAN;
        self.needs_cleanup = true;
        self.faults = self.faults.saturating_add(1);
        self.set_label(format!("RAW LOST: {error}"));
    }

    pub(super) fn service(&mut self, now: Instant) {
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
            && let Err(error) = drain(file, &mut self.queue, now, &mut self.sent_bytes)
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
        let mut queue = VecDeque::from([
            Pending {
                bytes: [0x90, 60, 100],
                offset: 0,
                queued_at: now,
            },
            Pending {
                bytes: [0x80, 60, 0],
                offset: 0,
                queued_at: now,
            },
        ]);
        let mut writer = ShortWriter {
            bytes: Vec::new(),
            block_next: false,
        };
        let mut sent = 0;
        for _ in 0..6 {
            drain(&mut writer, &mut queue, now, &mut sent).unwrap();
        }
        assert!(queue.is_empty());
        assert_eq!(writer.bytes, [0x90, 60, 100, 0x80, 60, 0]);
        assert_eq!(sent, 6);
    }

    #[test]
    fn stale_queue_is_not_replayed() {
        let now = Instant::now();
        let mut queue = VecDeque::from([Pending {
            bytes: [0x90, 60, 100],
            offset: 0,
            queued_at: now,
        }]);
        let mut sent = 0;
        assert_eq!(
            drain(&mut Vec::new(), &mut queue, now + MAX_AGE, &mut sent)
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
    fn device_names_exclude_non_midi_nodes() {
        assert_eq!(device_numbers("midiC12D3"), Some((12, 3)));
        for name in ["seq", "controlC1", "pcmC1D0p", "midiCD0", "midiC1D0x"] {
            assert_eq!(device_numbers(name), None);
        }
    }
}
