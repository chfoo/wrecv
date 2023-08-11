const ESCAPE_CHAR: char = char::REPLACEMENT_CHARACTER;
const LITERAL_SEQ_CHAR: char = '\u{E007F}';

pub fn parse_utf8_escaped(mut input: &[u8]) -> String {
    let mut buf = String::new();

    loop {
        match std::str::from_utf8(input) {
            Ok(valid) => {
                escape_escape_char(valid, &mut buf);
                break;
            }
            Err(error) => {
                let (valid, after_valid) = input.split_at(error.valid_up_to());
                let valid = std::str::from_utf8(valid).unwrap();
                escape_escape_char(valid, &mut buf);

                if let Some(invalid_sequence_length) = error.error_len() {
                    for byte in &after_valid[0..invalid_sequence_length] {
                        buf.push(ESCAPE_CHAR);
                        buf.push(byte_to_escape_seq(*byte));
                    }

                    input = &after_valid[invalid_sequence_length..]
                } else {
                    break;
                }
            }
        }
    }

    buf
}

fn byte_to_escape_seq(byte: u8) -> char {
    match byte {
        0..=0x0f => char::from_u32(0xFE00 + byte as u32).unwrap(),
        0x10..=0xff => char::from_u32(0xE0100 + byte as u32 - 0x10).unwrap(),
    }
}

fn escape_escape_char(input: &str, output: &mut String) {
    for ch in input.chars() {
        output.push(ch);

        if ch == ESCAPE_CHAR {
            output.push(LITERAL_SEQ_CHAR);
        }
    }
}

pub fn utf8_escaped_to_bytes<S: AsRef<str>>(input: S) -> Vec<u8> {
    let input = input.as_ref();
    let mut buf = Vec::<u8>::new();
    let mut state = EscapedToBytesState::Literal;

    for ch in input.chars() {
        match state {
            EscapedToBytesState::Literal => {
                if ch == ESCAPE_CHAR {
                    state = EscapedToBytesState::Escape;
                }
                let mut char_buf = [0u8; 4];
                buf.extend_from_slice(ch.encode_utf8(&mut char_buf).as_bytes());
            }
            EscapedToBytesState::Escape => {
                decode_escape_seq(ch, &mut buf);
                state = EscapedToBytesState::Literal;
            }
        }
    }

    buf
}

fn decode_escape_seq(ch: char, buf: &mut Vec<u8>) {
    if ch != LITERAL_SEQ_CHAR {
        match escape_seq_to_byte(ch) {
            Some(byte) => {
                buf.pop();
                buf.pop();
                buf.pop();
                buf.push(byte)
            }
            None => {
                let mut char_buf = [0u8; 4];
                buf.extend_from_slice(ch.encode_utf8(&mut char_buf).as_bytes());
            }
        }
    }
}

fn escape_seq_to_byte(ch: char) -> Option<u8> {
    match ch {
        '\u{FE00}'..='\u{FE0F}' => Some((ch as u32 - 0xFE00) as u8),
        '\u{E0100}'..='\u{E01EF}' => Some((ch as u32 - 0xE0100 + 0x10) as u8),
        _ => None,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EscapedToBytesState {
    Literal,
    Escape,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_utf8_escaped() {
        assert_eq!(parse_utf8_escaped(b"_\x80_"), "_\u{FFFD}\u{E0170}_");

        assert_eq!(parse_utf8_escaped(b"_\xEF\xBF\xBD_"), "_\u{FFFD}\u{E007F}_");
    }

    #[test]
    fn test_utf8_escaped_to_bytes() {
        assert_eq!(utf8_escaped_to_bytes("_\u{FFFD}\u{E0170}_"), b"_\x80_");

        assert_eq!(
            utf8_escaped_to_bytes("_\u{FFFD}_\u{FFFD}\u{E007F}_"),
            b"_\xEF\xBF\xBD_\xEF\xBF\xBD_"
        );
    }
}
