//! Controller decoding: raw kernel `input_event` records in, semantic edge
//! events out. No I/O — `ogm-cli` owns the device files and feeds bytes here.
//!
//! Only the inputs the overlay binds are decoded: left stick and D-pad for
//! navigation, A/Start to launch, B to quit. Everything else is ignored.

/// `struct input_event` on 64-bit Linux: 16-byte timeval, then `__u16 type`,
/// `__u16 code`, `__s32 value`.
pub const EVENT_SIZE: usize = 24;

const EV_KEY: u16 = 0x01;
const EV_ABS: u16 = 0x03;

// evdev names the four action buttons by position, not by label. On an
// Xbox-layout pad (what the 8BitDo reports in XInput mode) the top button is
// Y, so BTN_NORTH is Y and BTN_WEST is X.
const BTN_SOUTH: u16 = 304; // A
const BTN_EAST: u16 = 305; // B
const BTN_NORTH: u16 = 307; // Y
const BTN_WEST: u16 = 308; // X
const BTN_SELECT: u16 = 314; // Select/Back/"-"
const BTN_TL: u16 = 310; // LB
const BTN_TR: u16 = 311; // RB
const BTN_TL2: u16 = 312; // LT, on pads with digital triggers
const BTN_TR2: u16 = 313; // RT, likewise
const BTN_START: u16 = 315;

const ABS_X: u16 = 0;
const ABS_Y: u16 = 1;
const ABS_Z: u16 = 2; // LT, analog
const ABS_RZ: u16 = 5; // RT, analog
const ABS_HAT0X: u16 = 16;
const ABS_HAT0Y: u16 = 17;

// Analog triggers are usually 0..255, but some pads report 0..1023 or a
// signed range. Engaging at 128 and releasing at 64 behaves sensibly for all
// three: a resting trigger sits at or below its minimum either way.
const TRIGGER_ENGAGE: i32 = 128;
const TRIGGER_RELEASE: i32 = 64;

// Stick travel is ±32767. Engage past ~55% so a resting or drifting stick
// never moves the cursor, and hold until it falls back under ~34% so a stick
// wobbling around the threshold doesn't chatter press/release.
const ENGAGE: i32 = 18_000;
const RELEASE: i32 = 11_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    Up,
    Down,
    Left,
    Right,
}

/// The order the held-direction bitmap uses; `Direction::index` matches it.
const DIRECTIONS: [Direction; 4] = [
    Direction::Up,
    Direction::Down,
    Direction::Left,
    Direction::Right,
];

impl Direction {
    fn index(self) -> usize {
        match self {
            Direction::Up => 0,
            Direction::Down => 1,
            Direction::Left => 2,
            Direction::Right => 3,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Direction::Up => "up",
            Direction::Down => "down",
            Direction::Left => "left",
            Direction::Right => "right",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Button {
    /// A or Start — launch the highlighted game.
    Select,
    /// B — close the overlay.
    Quit,
    /// LB — previous category filter.
    PrevCategory,
    /// RB — next category filter.
    NextCategory,
    /// X — flip the sort direction.
    ToggleSortDir,
    /// Y — step to the next sort key.
    CycleSortKey,
    /// LT — one cover size smaller.
    PrevSize,
    /// RT — one cover size larger.
    NextSize,
    /// Select — rescan and fetch any artwork that is still missing.
    Refresh,
}

impl Button {
    pub fn as_str(self) -> &'static str {
        match self {
            Button::Select => "select",
            Button::Quit => "quit",
            Button::PrevCategory => "prev_category",
            Button::NextCategory => "next_category",
            Button::ToggleSortDir => "toggle_sort_dir",
            Button::CycleSortKey => "cycle_sort_key",
            Button::PrevSize => "prev_size",
            Button::NextSize => "next_size",
            Button::Refresh => "refresh",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PadEvent {
    /// A direction became held or was let go. The overlay drives its own
    /// key-repeat from these edges, so the same acceleration applies whether
    /// the input came from the stick, the D-pad or the arrow keys.
    Direction {
        dir: Direction,
        pressed: bool,
    },
    Button(Button),
}

/// One raw record off the device.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InputEvent {
    pub kind: u16,
    pub code: u16,
    pub value: i32,
}

/// Decode one 24-byte record. `None` when the slice is the wrong length.
pub fn parse_event(buf: &[u8]) -> Option<InputEvent> {
    if buf.len() != EVENT_SIZE {
        return None;
    }
    Some(InputEvent {
        kind: u16::from_ne_bytes([buf[16], buf[17]]),
        code: u16::from_ne_bytes([buf[18], buf[19]]),
        value: i32::from_ne_bytes([buf[20], buf[21], buf[22], buf[23]]),
    })
}

/// Tracks which directions are currently held so that press/release edges can
/// be emitted. Stick and D-pad are merged: holding a direction on both and
/// releasing one keeps the direction held, which is what the hand expects.
#[derive(Debug, Default)]
pub struct Decoder {
    stick_h: Option<Direction>,
    stick_v: Option<Direction>,
    hat_h: Option<Direction>,
    hat_v: Option<Direction>,
    trigger_l: bool,
    trigger_r: bool,
}

/// Analog trigger crossing its threshold, with hysteresis. `held` is whether
/// this trigger already counts as pressed.
fn trigger_held(value: i32, held: bool) -> bool {
    value
        >= if held {
            TRIGGER_RELEASE
        } else {
            TRIGGER_ENGAGE
        }
}

/// Analog axis with hysteresis: `current` is what this axis already reports.
fn stick_dir(
    value: i32,
    current: Option<Direction>,
    neg: Direction,
    pos: Direction,
) -> Option<Direction> {
    let threshold = if current.is_some() { RELEASE } else { ENGAGE };
    if value <= -threshold {
        Some(neg)
    } else if value >= threshold {
        Some(pos)
    } else {
        None
    }
}

/// D-pad hat axis is already digital (-1, 0, 1).
fn hat_dir(value: i32, neg: Direction, pos: Direction) -> Option<Direction> {
    match value {
        v if v < 0 => Some(neg),
        v if v > 0 => Some(pos),
        _ => None,
    }
}

impl Decoder {
    pub fn new() -> Self {
        Self::default()
    }

    /// Currently held directions, merged across stick and D-pad.
    fn held(&self) -> [bool; 4] {
        let mut out = [false; 4];
        for d in [self.stick_h, self.stick_v, self.hat_h, self.hat_v]
            .into_iter()
            .flatten()
        {
            out[d.index()] = true;
        }
        out
    }

    /// Release everything still held. A pad that disconnects (or sleeps)
    /// mid-hold would otherwise leave the overlay repeating that direction
    /// forever, since the release edge never arrives.
    pub fn release_all(&mut self) -> Vec<PadEvent> {
        let held = self.held();
        *self = Decoder::default();
        DIRECTIONS
            .into_iter()
            .filter(|d| held[d.index()])
            .map(|dir| PadEvent::Direction {
                dir,
                pressed: false,
            })
            .collect()
    }

    /// Feed one record; returns the edges it produced (usually none).
    pub fn feed(&mut self, ev: InputEvent) -> Vec<PadEvent> {
        if ev.kind == EV_KEY {
            // value 1 = press, 2 = kernel autorepeat, 0 = release. Buttons
            // are one-shot actions, so only the initial press counts.
            if ev.value != 1 {
                return Vec::new();
            }
            let button = match ev.code {
                BTN_SOUTH | BTN_START => Button::Select,
                BTN_EAST => Button::Quit,
                BTN_TL => Button::PrevCategory,
                BTN_TR => Button::NextCategory,
                BTN_TL2 => Button::PrevSize,
                BTN_TR2 => Button::NextSize,
                BTN_SELECT => Button::Refresh,
                BTN_WEST => Button::ToggleSortDir,
                BTN_NORTH => Button::CycleSortKey,
                _ => return Vec::new(),
            };
            return vec![PadEvent::Button(button)];
        }
        if ev.kind != EV_ABS {
            return Vec::new();
        }

        // Analog triggers act as one-shot buttons, not held directions, so
        // they are handled before the directional bookkeeping.
        if ev.code == ABS_Z || ev.code == ABS_RZ {
            let left = ev.code == ABS_Z;
            let was = if left { self.trigger_l } else { self.trigger_r };
            let now = trigger_held(ev.value, was);
            if left {
                self.trigger_l = now;
            } else {
                self.trigger_r = now;
            }
            if !(now && !was) {
                return Vec::new();
            }
            let button = if left {
                Button::PrevSize
            } else {
                Button::NextSize
            };
            return vec![PadEvent::Button(button)];
        }

        let before = self.held();
        match ev.code {
            // evdev Y grows downward, so a negative value is up.
            ABS_X => {
                self.stick_h = stick_dir(ev.value, self.stick_h, Direction::Left, Direction::Right)
            }
            ABS_Y => {
                self.stick_v = stick_dir(ev.value, self.stick_v, Direction::Up, Direction::Down)
            }
            ABS_HAT0X => self.hat_h = hat_dir(ev.value, Direction::Left, Direction::Right),
            ABS_HAT0Y => self.hat_v = hat_dir(ev.value, Direction::Up, Direction::Down),
            _ => return Vec::new(),
        }
        let after = self.held();

        let mut out = Vec::new();
        for dir in DIRECTIONS {
            let i = dir.index();
            if before[i] != after[i] {
                out.push(PadEvent::Direction {
                    dir,
                    pressed: after[i],
                });
            }
        }
        out
    }
}

/// EV_ABS bit in a device's `B: EV=` bitmap.
const EV_ABS_BIT: u32 = 3;

/// Is bit `bit` set in a `/proc/bus/input/devices` bitmap? The words are
/// 64-bit hex, printed most-significant group first, so the last word holds
/// codes 0..63, the one before it 64..127, and so on.
fn bitmap_has(bitmap: &str, bit: u32) -> bool {
    let words: Vec<&str> = bitmap.split_whitespace().collect();
    let from_end = (bit / 64) as usize;
    if from_end >= words.len() {
        return false;
    }
    let word = words[words.len() - 1 - from_end];
    u64::from_str_radix(word, 16)
        .map(|w| w >> (bit % 64) & 1 == 1)
        .unwrap_or(false)
}

/// A device worth reading, as named in `/proc/bus/input/devices`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PadDevice {
    pub name: String,
    /// The `eventN` handler, which maps to `/dev/input/eventN`.
    pub event: String,
}

/// Pick out the gamepads from `/proc/bus/input/devices`.
///
/// A device qualifies when it reports absolute axes *and* carries `BTN_SOUTH`
/// — the pair that distinguishes a controller from a mouse, a tablet, or a
/// keyboard that happens to expose a joystick interface (the ZSA Moonlander
/// does exactly that, and udev labels it `-event-joystick`).
pub fn parse_input_devices(text: &str) -> Vec<PadDevice> {
    let mut out = Vec::new();
    for block in text.split("\n\n") {
        let mut name = String::new();
        let mut event = String::new();
        let mut has_abs = false;
        let mut has_gamepad_button = false;
        for line in block.lines() {
            let line = line.trim();
            if let Some(rest) = line.strip_prefix("N: Name=") {
                name = rest.trim().trim_matches('"').to_string();
            } else if let Some(rest) = line.strip_prefix("H: Handlers=") {
                event = rest
                    .split_whitespace()
                    .find(|h| h.starts_with("event"))
                    .unwrap_or("")
                    .to_string();
            } else if let Some(rest) = line.strip_prefix("B: EV=") {
                has_abs = bitmap_has(rest.trim(), EV_ABS_BIT);
            } else if let Some(rest) = line.strip_prefix("B: KEY=") {
                has_gamepad_button = bitmap_has(rest.trim(), BTN_SOUTH as u32);
            }
        }
        if has_abs && has_gamepad_button && !event.is_empty() {
            out.push(PadDevice { name, event });
        }
    }
    out
}

/// One line of the `ogm gamepad` stream, as the QML side parses it.
pub fn event_to_json(ev: PadEvent) -> String {
    match ev {
        PadEvent::Direction { dir, pressed } => format!(
            r#"{{"type":"dir","value":"{}","pressed":{}}}"#,
            dir.as_str(),
            pressed
        ),
        PadEvent::Button(b) => format!(r#"{{"type":"button","value":"{}"}}"#, b.as_str()),
    }
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

    fn ev(kind: u16, code: u16, value: i32) -> InputEvent {
        InputEvent { kind, code, value }
    }

    #[test]
    fn parses_a_raw_record() {
        let bytes = raw(EV_ABS, ABS_X, -32768);
        assert_eq!(
            parse_event(&bytes),
            Some(InputEvent {
                kind: EV_ABS,
                code: ABS_X,
                value: -32768
            })
        );
    }

    #[test]
    fn rejects_a_short_record() {
        assert_eq!(parse_event(&[0u8; 12]), None);
        assert_eq!(parse_event(&[]), None);
    }

    #[test]
    fn a_and_start_select_b_quits() {
        let mut d = Decoder::new();
        assert_eq!(
            d.feed(ev(EV_KEY, BTN_SOUTH, 1)),
            vec![PadEvent::Button(Button::Select)]
        );
        assert_eq!(
            d.feed(ev(EV_KEY, BTN_START, 1)),
            vec![PadEvent::Button(Button::Select)]
        );
        assert_eq!(
            d.feed(ev(EV_KEY, BTN_EAST, 1)),
            vec![PadEvent::Button(Button::Quit)]
        );
    }

    #[test]
    fn button_release_and_autorepeat_are_ignored() {
        let mut d = Decoder::new();
        assert!(d.feed(ev(EV_KEY, BTN_SOUTH, 0)).is_empty());
        assert!(d.feed(ev(EV_KEY, BTN_SOUTH, 2)).is_empty());
    }

    #[test]
    fn shoulders_and_face_buttons_are_bound() {
        let mut d = Decoder::new();
        assert_eq!(
            d.feed(ev(EV_KEY, BTN_TL, 1)),
            vec![PadEvent::Button(Button::PrevCategory)]
        );
        assert_eq!(
            d.feed(ev(EV_KEY, BTN_TR, 1)),
            vec![PadEvent::Button(Button::NextCategory)]
        );
        assert_eq!(
            d.feed(ev(EV_KEY, BTN_WEST, 1)),
            vec![PadEvent::Button(Button::ToggleSortDir)]
        );
        assert_eq!(
            d.feed(ev(EV_KEY, BTN_NORTH, 1)),
            vec![PadEvent::Button(Button::CycleSortKey)]
        );
        assert_eq!(
            d.feed(ev(EV_KEY, BTN_SELECT, 1)),
            vec![PadEvent::Button(Button::Refresh)]
        );
    }

    #[test]
    fn analog_triggers_fire_once_per_pull() {
        let mut d = Decoder::new();
        assert!(d.feed(ev(EV_ABS, ABS_RZ, 0)).is_empty());
        assert!(d.feed(ev(EV_ABS, ABS_RZ, 100)).is_empty()); // under the threshold
        assert_eq!(
            d.feed(ev(EV_ABS, ABS_RZ, 200)),
            vec![PadEvent::Button(Button::NextSize)]
        );
        // Still held: a wobble must not re-fire.
        assert!(d.feed(ev(EV_ABS, ABS_RZ, 255)).is_empty());
        assert!(d.feed(ev(EV_ABS, ABS_RZ, 100)).is_empty());
        // Released, then pulled again.
        assert!(d.feed(ev(EV_ABS, ABS_RZ, 0)).is_empty());
        assert_eq!(
            d.feed(ev(EV_ABS, ABS_RZ, 200)),
            vec![PadEvent::Button(Button::NextSize)]
        );
    }

    #[test]
    fn left_trigger_shrinks_and_is_independent_of_the_right() {
        let mut d = Decoder::new();
        assert_eq!(
            d.feed(ev(EV_ABS, ABS_Z, 255)),
            vec![PadEvent::Button(Button::PrevSize)]
        );
        assert_eq!(
            d.feed(ev(EV_ABS, ABS_RZ, 255)),
            vec![PadEvent::Button(Button::NextSize)]
        );
    }

    #[test]
    fn digital_triggers_work_too() {
        let mut d = Decoder::new();
        assert_eq!(
            d.feed(ev(EV_KEY, BTN_TL2, 1)),
            vec![PadEvent::Button(Button::PrevSize)]
        );
        assert_eq!(
            d.feed(ev(EV_KEY, BTN_TR2, 1)),
            vec![PadEvent::Button(Button::NextSize)]
        );
    }

    #[test]
    fn triggers_do_not_disturb_the_held_directions() {
        let mut d = Decoder::new();
        d.feed(ev(EV_ABS, ABS_Y, 30_000)); // hold down
        assert_eq!(
            d.feed(ev(EV_ABS, ABS_RZ, 255)),
            vec![PadEvent::Button(Button::NextSize)]
        );
        // The held direction survives, and still releases normally.
        assert_eq!(
            d.feed(ev(EV_ABS, ABS_Y, 0)),
            vec![PadEvent::Direction {
                dir: Direction::Down,
                pressed: false
            }]
        );
    }

    #[test]
    fn unbound_buttons_are_ignored() {
        let mut d = Decoder::new();
        assert!(d.feed(ev(EV_KEY, 316, 1)).is_empty()); // Mode/Home
        assert!(d.feed(ev(EV_KEY, 317, 1)).is_empty()); // L3
    }

    #[test]
    fn resting_stick_emits_nothing() {
        let mut d = Decoder::new();
        assert!(d.feed(ev(EV_ABS, ABS_X, 0)).is_empty());
        assert!(d.feed(ev(EV_ABS, ABS_Y, -1)).is_empty());
        // Drift well inside the engage threshold stays silent.
        assert!(d.feed(ev(EV_ABS, ABS_X, 12_000)).is_empty());
    }

    #[test]
    fn stick_push_and_return_emit_one_press_and_one_release() {
        let mut d = Decoder::new();
        assert_eq!(
            d.feed(ev(EV_ABS, ABS_Y, 30_000)),
            vec![PadEvent::Direction {
                dir: Direction::Down,
                pressed: true
            }]
        );
        // Still past the release band: no repeat edge.
        assert!(d.feed(ev(EV_ABS, ABS_Y, 20_000)).is_empty());
        assert_eq!(
            d.feed(ev(EV_ABS, ABS_Y, 0)),
            vec![PadEvent::Direction {
                dir: Direction::Down,
                pressed: false
            }]
        );
    }

    #[test]
    fn stick_holds_through_the_hysteresis_band() {
        let mut d = Decoder::new();
        d.feed(ev(EV_ABS, ABS_X, 32_000));
        // Between RELEASE and ENGAGE: engaged, so it stays held.
        assert!(d.feed(ev(EV_ABS, ABS_X, 14_000)).is_empty());
        // Below RELEASE: now it lets go.
        assert_eq!(
            d.feed(ev(EV_ABS, ABS_X, 9_000)),
            vec![PadEvent::Direction {
                dir: Direction::Right,
                pressed: false
            }]
        );
    }

    #[test]
    fn stick_up_is_negative_y() {
        let mut d = Decoder::new();
        assert_eq!(
            d.feed(ev(EV_ABS, ABS_Y, -30_000)),
            vec![PadEvent::Direction {
                dir: Direction::Up,
                pressed: true
            }]
        );
    }

    #[test]
    fn swinging_across_centre_releases_then_presses() {
        let mut d = Decoder::new();
        d.feed(ev(EV_ABS, ABS_X, 30_000)); // right
        let out = d.feed(ev(EV_ABS, ABS_X, -30_000)); // hard left
        assert_eq!(
            out,
            vec![
                PadEvent::Direction {
                    dir: Direction::Left,
                    pressed: true
                },
                PadEvent::Direction {
                    dir: Direction::Right,
                    pressed: false
                },
            ]
        );
    }

    #[test]
    fn dpad_is_digital() {
        let mut d = Decoder::new();
        assert_eq!(
            d.feed(ev(EV_ABS, ABS_HAT0Y, -1)),
            vec![PadEvent::Direction {
                dir: Direction::Up,
                pressed: true
            }]
        );
        assert_eq!(
            d.feed(ev(EV_ABS, ABS_HAT0Y, 0)),
            vec![PadEvent::Direction {
                dir: Direction::Up,
                pressed: false
            }]
        );
    }

    #[test]
    fn stick_and_dpad_merge_into_one_held_direction() {
        let mut d = Decoder::new();
        // Push the stick down, then also press D-pad down.
        assert_eq!(
            d.feed(ev(EV_ABS, ABS_Y, 30_000)),
            vec![PadEvent::Direction {
                dir: Direction::Down,
                pressed: true
            }]
        );
        assert!(d.feed(ev(EV_ABS, ABS_HAT0Y, 1)).is_empty());
        // Releasing only the D-pad keeps it held — the stick is still down.
        assert!(d.feed(ev(EV_ABS, ABS_HAT0Y, 0)).is_empty());
        assert_eq!(
            d.feed(ev(EV_ABS, ABS_Y, 0)),
            vec![PadEvent::Direction {
                dir: Direction::Down,
                pressed: false
            }]
        );
    }

    #[test]
    fn diagonals_hold_both_axes_independently() {
        let mut d = Decoder::new();
        d.feed(ev(EV_ABS, ABS_X, 30_000));
        assert_eq!(
            d.feed(ev(EV_ABS, ABS_Y, 30_000)),
            vec![PadEvent::Direction {
                dir: Direction::Down,
                pressed: true
            }]
        );
    }

    #[test]
    fn non_input_event_types_are_ignored() {
        let mut d = Decoder::new();
        assert!(d.feed(ev(0x00, 0, 0)).is_empty()); // EV_SYN
        assert!(d.feed(ev(0x15, 80, 1)).is_empty()); // EV_FF
    }

    // Verbatim from /proc/bus/input/devices: an 8BitDo Ultimate 2 pad, the
    // keyboard interface the same dongle exposes, and a ZSA Moonlander whose
    // joystick interface udev labels `-event-joystick` despite having no
    // gamepad buttons at all.
    const PROC_DEVICES: &str = "\
I: Bus=0003 Vendor=2dc8 Product=310b Version=0114
N: Name=\"8BitDo Ultimate 2 Wireless Controller\"
P: Phys=usb-0000:18:00.4-1.3.3/input0
H: Handlers=event24 js0
B: PROP=0
B: EV=20000b
B: KEY=7cdb000000000000 0 0 0 0
B: ABS=3003f
B: FF=107030000 0

I: Bus=0003 Vendor=2dc8 Product=310b Version=0111
N: Name=\"8BitDo Ultimate 2 Wireless Controller for PC Keyboard\"
H: Handlers=sysrq kbd leds event25
B: PROP=0
B: EV=12001f
B: KEY=3f00733fff 0 0 483ffff17aff32d bfd4444600000000 1 130ff38b17d007 ffff7bfad9415fff ffbeffdfffefffff fffffffffffffffe
B: REL=1040

I: Bus=0003 Vendor=3297 Product=1969 Version=0111
N: Name=\"ZSA Technology Labs Moonlander Mark I\"
H: Handlers=event5 js1
B: PROP=0
B: EV=1b
B: KEY=3f000301ff 0 0 0 0
B: ABS=100000000
";

    #[test]
    fn finds_the_gamepad_and_rejects_keyboards() {
        let found = parse_input_devices(PROC_DEVICES);
        assert_eq!(
            found,
            vec![PadDevice {
                name: "8BitDo Ultimate 2 Wireless Controller".into(),
                event: "event24".into(),
            }]
        );
    }

    #[test]
    fn empty_or_garbage_input_yields_nothing() {
        assert!(parse_input_devices("").is_empty());
        assert!(parse_input_devices("not a device list").is_empty());
        // Gamepad buttons but no absolute axes is not something we can steer.
        assert!(parse_input_devices(
            "N: Name=\"x\"\nH: Handlers=event9 \nB: EV=3\nB: KEY=7cdb000000000000 0 0 0 0\n"
        )
        .is_empty());
    }

    #[test]
    fn bitmap_indexing_counts_from_the_last_word() {
        // BTN_SOUTH is 304: word 4 from the end, bit 48 within it.
        assert!(bitmap_has("7cdb000000000000 0 0 0 0", 304));
        assert!(bitmap_has("7cdb000000000000 0 0 0 0", 305)); // BTN_EAST
        assert!(bitmap_has("7cdb000000000000 0 0 0 0", 315)); // BTN_START
        assert!(!bitmap_has("7cdb000000000000 0 0 0 0", 306)); // BTN_C, absent
                                                               // A keyboard's bitmap has nothing up at 304.
        assert!(!bitmap_has("3f000301ff 0 0 0 0", 304));
        // Asking past the end of a short bitmap is false, not a panic.
        assert!(!bitmap_has("1", 304));
        assert!(!bitmap_has("", 0));
        assert!(!bitmap_has("zz", 0));
    }

    #[test]
    fn disconnecting_mid_hold_releases_what_was_held() {
        let mut d = Decoder::new();
        d.feed(ev(EV_ABS, ABS_Y, 30_000)); // down, still held
        d.feed(ev(EV_ABS, ABS_HAT0X, -1)); // left, still held
        let mut out = d.release_all();
        out.sort_by_key(|e| format!("{e:?}"));
        assert_eq!(
            out,
            vec![
                PadEvent::Direction {
                    dir: Direction::Down,
                    pressed: false
                },
                PadEvent::Direction {
                    dir: Direction::Left,
                    pressed: false
                },
            ]
        );
        // Nothing held any more, so a second call is silent.
        assert!(d.release_all().is_empty());
    }

    #[test]
    fn release_all_with_nothing_held_is_silent() {
        assert!(Decoder::new().release_all().is_empty());
    }

    #[test]
    fn json_lines_match_the_qml_contract() {
        assert_eq!(
            event_to_json(PadEvent::Direction {
                dir: Direction::Left,
                pressed: true
            }),
            r#"{"type":"dir","value":"left","pressed":true}"#
        );
        assert_eq!(
            event_to_json(PadEvent::Direction {
                dir: Direction::Up,
                pressed: false
            }),
            r#"{"type":"dir","value":"up","pressed":false}"#
        );
        assert_eq!(
            event_to_json(PadEvent::Button(Button::Select)),
            r#"{"type":"button","value":"select"}"#
        );
        assert_eq!(
            event_to_json(PadEvent::Button(Button::Quit)),
            r#"{"type":"button","value":"quit"}"#
        );
        assert_eq!(
            event_to_json(PadEvent::Button(Button::PrevCategory)),
            r#"{"type":"button","value":"prev_category"}"#
        );
        assert_eq!(
            event_to_json(PadEvent::Button(Button::NextCategory)),
            r#"{"type":"button","value":"next_category"}"#
        );
        assert_eq!(
            event_to_json(PadEvent::Button(Button::ToggleSortDir)),
            r#"{"type":"button","value":"toggle_sort_dir"}"#
        );
        assert_eq!(
            event_to_json(PadEvent::Button(Button::CycleSortKey)),
            r#"{"type":"button","value":"cycle_sort_key"}"#
        );
        assert_eq!(
            event_to_json(PadEvent::Button(Button::PrevSize)),
            r#"{"type":"button","value":"prev_size"}"#
        );
        assert_eq!(
            event_to_json(PadEvent::Button(Button::NextSize)),
            r#"{"type":"button","value":"next_size"}"#
        );
        assert_eq!(
            event_to_json(PadEvent::Button(Button::Refresh)),
            r#"{"type":"button","value":"refresh"}"#
        );
    }
}
