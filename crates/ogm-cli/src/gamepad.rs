//! `ogm gamepad`: stream controller input as line-delimited JSON on stdout.
//!
//! The overlay spawns this and parses one event per line. Devices are found
//! by capability rather than by udev symlink, so USB, Bluetooth and virtual
//! pads all work; discovery reruns periodically so a wireless pad that
//! connects (or wakes) after the menu is open still works. Decoding itself
//! lives in `ogm-core`.

use std::collections::HashSet;
use std::fs::File;
use std::io::{Read, Write};
use std::path::PathBuf;
use std::sync::mpsc::{channel, Sender};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use anyhow::Result;
use ogm_core::gamepad::PadDevice;
use ogm_core::gamepad::{event_to_json, parse_event, parse_input_devices, Decoder, EVENT_SIZE};

const PROC_DEVICES: &str = "/proc/bus/input/devices";
const RESCAN_INTERVAL: Duration = Duration::from_secs(2);

fn detect_pads() -> Vec<PadDevice> {
    std::fs::read_to_string(PROC_DEVICES)
        .map(|t| parse_input_devices(&t))
        .unwrap_or_default()
}

fn device_path(pad: &PadDevice) -> PathBuf {
    PathBuf::from("/dev/input").join(&pad.event)
}

/// Read one device until it errors or disappears, emitting decoded edges.
fn read_device(path: PathBuf, tx: Sender<String>) {
    let Ok(mut file) = File::open(&path) else {
        return;
    };
    let mut decoder = Decoder::new();
    let mut buf = [0u8; EVENT_SIZE];
    // A short read means the device went away mid-record; a disconnected pad
    // is normal, so just stop reading this one and let rescan pick it back up.
    while file.read_exact(&mut buf).is_ok() {
        let Some(ev) = parse_event(&buf) else {
            break;
        };
        for out in decoder.feed(ev) {
            if tx.send(event_to_json(out)).is_err() {
                return;
            }
        }
    }
    // The pad went away. Anything still held has to be let go explicitly or
    // the overlay keeps repeating that direction.
    for out in decoder.release_all() {
        if tx.send(event_to_json(out)).is_err() {
            return;
        }
    }
}

pub fn run() -> Result<()> {
    let (tx, rx) = channel::<String>();
    let open: Arc<Mutex<HashSet<PathBuf>>> = Arc::new(Mutex::new(HashSet::new()));

    {
        let open = Arc::clone(&open);
        thread::spawn(move || loop {
            for pad in detect_pads() {
                let path = device_path(&pad);
                let fresh = open
                    .lock()
                    .map(|mut s| s.insert(path.clone()))
                    .unwrap_or(false);
                if !fresh {
                    continue;
                }
                let tx = tx.clone();
                let open = Arc::clone(&open);
                thread::spawn(move || {
                    read_device(path.clone(), tx);
                    // Let a reconnect of the same node be picked up again.
                    if let Ok(mut s) = open.lock() {
                        s.remove(&path);
                    }
                });
            }
            thread::sleep(RESCAN_INTERVAL);
        });
    }

    let mut stdout = std::io::stdout();
    for line in rx {
        // A closed pipe means the overlay is gone — exit quietly.
        if writeln!(stdout, "{line}").is_err() || stdout.flush().is_err() {
            return Ok(());
        }
    }
    Ok(())
}

/// `ogm gamepad --list`: what the overlay would read from.
pub fn list() -> Result<()> {
    let pads = detect_pads();
    if pads.is_empty() {
        println!("no gamepads found in {PROC_DEVICES}");
        return Ok(());
    }
    for pad in pads {
        let path = device_path(&pad);
        println!(
            "{}  {}  [{}]",
            path.display(),
            pad.name,
            if File::open(&path).is_ok() {
                "readable"
            } else {
                "NOT READABLE — add your user to the 'input' group"
            }
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn raw(kind: u16, code: u16, value: i32) -> Vec<u8> {
        let mut v = vec![0u8; EVENT_SIZE];
        v[16..18].copy_from_slice(&kind.to_ne_bytes());
        v[18..20].copy_from_slice(&code.to_ne_bytes());
        v[20..24].copy_from_slice(&value.to_ne_bytes());
        v
    }

    #[test]
    fn event_node_maps_to_dev_input() {
        let pad = PadDevice {
            name: "pad".into(),
            event: "event24".into(),
        };
        assert_eq!(device_path(&pad), PathBuf::from("/dev/input/event24"));
    }

    /// The read loop over a file of raw records: a regular file behaves like
    /// the device up to EOF, which exercises parse + decode + emit in order.
    #[test]
    fn reads_a_stream_of_records_into_json_lines() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("events");
        let mut f = File::create(&path).unwrap();
        for (kind, code, value) in [
            (0x03u16, 1u16, 30_000i32), // stick down
            (0x03, 1, 0),               // released
            (0x03, 17, -1),             // d-pad up
            (0x03, 17, 0),              // released
            (0x01, 304, 1),             // A
            (0x01, 305, 1),             // B
            (0x01, 310, 1),             // LB
            (0x01, 316, 1),             // Mode/Home — not bound
        ] {
            f.write_all(&raw(kind, code, value)).unwrap();
        }
        drop(f);

        let (tx, rx) = channel();
        read_device(path, tx);
        let lines: Vec<String> = rx.into_iter().collect();
        assert_eq!(
            lines,
            vec![
                r#"{"type":"dir","value":"down","pressed":true}"#,
                r#"{"type":"dir","value":"down","pressed":false}"#,
                r#"{"type":"dir","value":"up","pressed":true}"#,
                r#"{"type":"dir","value":"up","pressed":false}"#,
                r#"{"type":"button","value":"select"}"#,
                r#"{"type":"button","value":"quit"}"#,
                r#"{"type":"button","value":"prev_category"}"#,
            ]
        );
    }

    #[test]
    fn a_device_vanishing_mid_hold_emits_the_release() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("events");
        let mut f = File::create(&path).unwrap();
        // Press down and then "disconnect" (EOF) without ever releasing.
        f.write_all(&raw(0x03, 1, 30_000)).unwrap();
        drop(f);

        let (tx, rx) = channel();
        read_device(path, tx);
        let lines: Vec<String> = rx.into_iter().collect();
        assert_eq!(
            lines,
            vec![
                r#"{"type":"dir","value":"down","pressed":true}"#,
                r#"{"type":"dir","value":"down","pressed":false}"#,
            ]
        );
    }

    #[test]
    fn a_missing_device_is_not_an_error() {
        let (tx, rx) = channel();
        read_device(PathBuf::from("/nonexistent/event99"), tx);
        assert!(rx.into_iter().next().is_none());
    }

    #[test]
    fn a_truncated_trailing_record_is_dropped() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("events");
        let mut f = File::create(&path).unwrap();
        f.write_all(&raw(0x01, 304, 1)).unwrap();
        f.write_all(&[0u8; 7]).unwrap(); // half a record, then EOF
        drop(f);

        let (tx, rx) = channel();
        read_device(path, tx);
        let lines: Vec<String> = rx.into_iter().collect();
        assert_eq!(lines, vec![r#"{"type":"button","value":"select"}"#]);
    }
}
