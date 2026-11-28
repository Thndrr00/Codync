#![allow(dead_code, clippy::pedantic, clippy::style, clippy::perf)]

//! Pure helpers ported from hermes-agent's `csi.ts`: CSI byte ranges and terminal control sequence
//! builders for the TUI. Composes helpers already ported into this host. Only `botInsights` calls
//! this.

/// Escape plus the CSI introducer (CSI_PREFIX).
pub const CSI_PREFIX: &str = "\x1b[";

/// Move the cursor home (CSI H).
pub const CURSOR_HOME: &str = "\x1b[H";

/// Save the cursor position (CSI s).
pub const CURSOR_SAVE: &str = "\x1b[s";

/// Restore the cursor position (CSI u).
pub const CURSOR_RESTORE: &str = "\x1b[u";

/// Erase the whole line (CSI 2 K).
pub const ERASE_LINE: &str = "\x1b[2K";

/// Erase the whole screen (CSI 2 J).
pub const ERASE_SCREEN: &str = "\x1b[2J";

/// Whether a byte is a CSI parameter byte (0x30–0x3f).
pub fn is_csi_param(byte: u8) -> bool {
    (0x30..=0x3f).contains(&byte)
}

/// Whether a byte is a CSI intermediate byte (0x20–0x2f).
pub fn is_csi_intermediate(byte: u8) -> bool {
    (0x20..=0x2f).contains(&byte)
}

/// Whether a byte is a CSI final byte (@ through ~).
pub fn is_csi_final(byte: u8) -> bool {
    (0x40..=0x7e).contains(&byte)
}

/// Build a CSI sequence from parameters and a final byte (csi).
pub fn csi(params: &[String], final_byte: char) -> String {
    if params.is_empty() {
        format!("{CSI_PREFIX}{final_byte}")
    } else {
        format!("{CSI_PREFIX}{}{final_byte}", params.join(";"))
    }
}

/// Build a CSI sequence from numeric parameters (csiNumbers).
fn csi_numbers(params: &[u64], final_byte: char) -> String {
    csi(&params.iter().map(u64::to_string).collect::<Vec<_>>(), final_byte)
}

/// Move the cursor up n lines (cursorUp).
pub fn cursor_up(n: u64) -> String {
    if n == 0 { String::new() } else { csi_numbers(&[n], 'A') }
}

/// Move the cursor down n lines (cursorDown).
pub fn cursor_down(n: u64) -> String {
    if n == 0 { String::new() } else { csi_numbers(&[n], 'B') }
}

/// Move the cursor forward n columns (cursorForward).
pub fn cursor_forward(n: u64) -> String {
    if n == 0 { String::new() } else { csi_numbers(&[n], 'C') }
}

/// Move the cursor back n columns (cursorBack).
pub fn cursor_back(n: u64) -> String {
    if n == 0 { String::new() } else { csi_numbers(&[n], 'D') }
}

/// Move the cursor to a 1-indexed column (cursorTo).
pub fn cursor_to(column: u64) -> String {
    csi_numbers(&[column], 'G')
}

/// Move the cursor to a 1-indexed row and column (cursorPosition).
pub fn cursor_position(row: u64, column: u64) -> String {
    csi_numbers(&[row, column], 'H')
}

/// Move the cursor relative to its position (cursorMove).
pub fn cursor_move(x: i64, y: i64) -> String {
    let mut out = String::new();
    if x < 0 {
        out.push_str(&cursor_back(x.unsigned_abs()));
    } else if x > 0 {
        out.push_str(&cursor_forward(x as u64));
    }
    if y < 0 {
        out.push_str(&cursor_up(y.unsigned_abs()));
    } else if y > 0 {
        out.push_str(&cursor_down(y as u64));
    }
    out
}

/// Erase from the cursor to the end of the line (CSI K).
pub fn erase_to_end_of_line() -> String {
    csi(&[], 'K')
}

/// Erase from the cursor to the start of the line (CSI 1 K).
pub fn erase_to_start_of_line() -> String {
    csi_numbers(&[1], 'K')
}

/// Erase the whole line (CSI 2 K).
pub fn erase_line() -> String {
    ERASE_LINE.to_owned()
}

/// Erase from the cursor to the end of the screen (CSI J).
pub fn erase_to_end_of_screen() -> String {
    csi(&[], 'J')
}

/// Erase the whole screen (CSI 2 J).
pub fn erase_screen() -> String {
    ERASE_SCREEN.to_owned()
}

/// Erase the scrollback buffer (CSI 3 J).
pub fn erase_scrollback() -> String {
    csi_numbers(&[3], 'J')
}

/// Erase n lines from the cursor, moving up and ending at column 1 (eraseLines).
pub fn erase_lines(n: usize) -> String {
    if n == 0 {
        return String::new();
    }
    let mut out = String::new();
    for index in 0..n {
        out.push_str(ERASE_LINE);
        if index + 1 < n {
            out.push_str(&cursor_up(1));
        }
    }
    out.push_str(&cursor_to(1));
    out
}

/// Scroll up n lines (scrollUp).
pub fn scroll_up(n: u64) -> String {
    if n == 0 { String::new() } else { csi_numbers(&[n], 'S') }
}

/// Scroll down n lines (scrollDown).
pub fn scroll_down(n: u64) -> String {
    if n == 0 { String::new() } else { csi_numbers(&[n], 'T') }
}

/// Set the 1-indexed scroll region (DECSTBM).
pub fn set_scroll_region(top: u64, bottom: u64) -> String {
    csi_numbers(&[top, bottom], 'r')
}

/// Set the cursor style (DECSCUSR).
pub fn cursor_style(style: &str, blinking: bool) -> String {
    let parameter = match (style, blinking) {
        ("block", true) => 1,
        ("block", false) => 2,
        ("underline", true) => 3,
        ("underline", false) => 4,
        ("bar", true) => 5,
        ("bar", false) => 6,
        _ => 1,
    };
    csi_numbers(&[parameter], 'q')
}

/// Compact count label, reusing the shared formatter.
pub fn count_label(n: usize) -> String {
    crate::bdots_desk::compact_count(n as u64)
}

/// Summary rows for the card.
pub fn summary(sequences: usize, bytes: usize, erased: bool, saved: bool) -> Vec<(&'static str, String)> {
    vec![
        ("sequences", count_label(sequences)),
        ("bytes", count_label(bytes)),
        ("erased", if erased { "yes" } else { "no" }.to_owned()),
        ("saved", if saved { "yes" } else { "no" }.to_owned()),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn byte_ranges() {
        assert!(is_csi_param(b'0'));
        assert!(is_csi_param(b'?'));
        assert!(!is_csi_param(b'A'));
        assert!(is_csi_intermediate(b' '));
        assert!(is_csi_final(b'A'));
        assert!(is_csi_final(b'~'));
        assert!(!is_csi_final(b'['));
    }

    #[test]
    fn builders() {
        assert_eq!(csi(&[], 'K'), "\x1b[K");
        assert_eq!(csi_numbers(&[2, 3], 'H'), "\x1b[2;3H");
        assert_eq!(cursor_up(0), "");
        assert_eq!(cursor_up(3), "\x1b[3A");
        assert_eq!(cursor_down(2), "\x1b[2B");
        assert_eq!(cursor_forward(1), "\x1b[1C");
        assert_eq!(cursor_back(4), "\x1b[4D");
        assert_eq!(cursor_to(5), "\x1b[5G");
        assert_eq!(cursor_position(2, 7), "\x1b[2;7H");
        assert_eq!(cursor_move(-2, 3), "\x1b[2D\x1b[3B");
        assert_eq!(CURSOR_HOME, "\x1b[H");
        assert_eq!(CURSOR_SAVE, "\x1b[s");
        assert_eq!(CURSOR_RESTORE, "\x1b[u");
    }

    #[test]
    fn erasing_and_scrolling() {
        assert_eq!(erase_to_end_of_line(), "\x1b[K");
        assert_eq!(erase_to_start_of_line(), "\x1b[1K");
        assert_eq!(erase_line(), "\x1b[2K");
        assert_eq!(erase_to_end_of_screen(), "\x1b[J");
        assert_eq!(erase_screen(), "\x1b[2J");
        assert_eq!(erase_scrollback(), "\x1b[3J");
        assert_eq!(erase_lines(0), "");
        assert_eq!(erase_lines(2), "\x1b[2K\x1b[1A\x1b[2K\x1b[1G");
        assert_eq!(scroll_up(2), "\x1b[2S");
        assert_eq!(scroll_down(0), "");
        assert_eq!(set_scroll_region(1, 24), "\x1b[1;24r");
    }

    #[test]
    fn styles_and_summary() {
        assert_eq!(cursor_style("block", true), "\x1b[1q");
        assert_eq!(cursor_style("bar", false), "\x1b[6q");
        assert_eq!(cursor_style("underline", true), "\x1b[3q");
        assert_eq!(count_label(1500), "1.5K");
        assert_eq!(summary(4, 12, true, false).len(), 4);
    }
}
