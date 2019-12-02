use regex::{Regex, RegexBuilder};

pub fn frame_to_message(frame: &str) -> Result<&str, Box<dyn std::error::Error>> {
    lazy_static::lazy_static! {
        static ref CODE_REGEX: Regex = RegexBuilder::new("^\\s*Content-Type: text/plain\\s*Content-Length:(\\d+)\\s*(.*)\\s*$")
            .dot_matches_new_line(true)
            .build().unwrap();
    }
    let captures = CODE_REGEX
        .captures(frame)
        .ok_or("frame does not match required pattern")?;
    let content_length = usize::from_str_radix(captures.get(1).unwrap().as_str(), 10)?;
    let content = captures.get(2).unwrap().as_str();
    if content_length != content.len() {
        return Err("content_length does not match content.len()".into());
    }
    return Ok(content);
}

#[derive(Debug)]
pub struct Buffer {
    boundary: String,
    buffer: String,
}
impl Buffer {
    pub fn new(boundary: String) -> Self {
        return Self {
            boundary,
            buffer: String::new(),
        };
    }

    pub fn try_extract_frame(&mut self) -> Option<String> {
        let prefix = format!("--{}", self.boundary);
        let suffix = "\r\n\r\n";

        if self.buffer.len() < prefix.len() + suffix.len() {
            return None;
        }

        if let Some(prefix_position) = self.buffer.find(&prefix) {
            if prefix_position > 0 {
                log::warn!(
                    "Prefix found but not on the beginning ({}), truncating",
                    prefix_position
                );
                self.buffer = self.buffer[prefix_position..].to_owned();
            }
        } else {
            log::warn!("Buffer too large with no prefix, clearing");
            let skip_bytes = self.buffer.len() - prefix.len();
            self.buffer = self.buffer[skip_bytes..].to_owned();
            return None;
        }

        // self.buffer starts with prefix
        if let Some(suffix_position) = self.buffer[prefix.len()..].find(suffix) {
            let item = self.buffer[prefix.len()..suffix_position + prefix.len()].to_owned();
            self.buffer = self.buffer[prefix.len() + suffix_position + suffix.len()..].to_owned();
            return Some(item);
        } else {
            // FIXME: Possible infinite buffer increment
            return None;
        }
    }

    pub fn append(
        &mut self,
        input: &str,
    ) -> () {
        self.buffer.push_str(input);
        return ();
    }
}

#[cfg(test)]
mod tests_frame_to_message {
    use super::frame_to_message;

    #[test]
    fn test_ok_1() {
        let result = frame_to_message(
            "Content-Type: text/plain\r\nContent-Length:36\r\nCode=VideoBlind;action=Start;index=0",
        )
        .unwrap();
        assert_eq!(result, "Code=VideoBlind;action=Start;index=0");
    }

    #[test]
    fn test_ok_2() {
        let result = frame_to_message("Content-Type: text/plain\r\nContent-Length:0\r\n").unwrap();
        assert_eq!(result, "");
    }

    #[test]
    fn test_invalid_length_1() {
        frame_to_message(
            "Content-Type: text/plain\r\nContent-Length:37\r\nCode=VideoBlind;action=Start;index=0",
        )
        .unwrap_err();
    }
}

#[cfg(test)]
mod tests_buffer {
    use super::Buffer;

    #[test]
    fn test_empty() {
        let mut buffer = Buffer::new("myboundary".to_owned());
        assert_eq!(buffer.try_extract_frame(), None);
    }

    #[test]
    fn test_ok_1() {
        let mut buffer = Buffer::new("myboundary".to_owned());
        buffer.append("--myboundary\r\n\r\n");
        assert_eq!(buffer.try_extract_frame(), Some("".to_owned()));
        assert_eq!(buffer.try_extract_frame(), None);
    }

    #[test]
    fn test_ok_2a() {
        let mut buffer = Buffer::new("myboundary".to_owned());
        buffer.append("--myboundary1\r\n\r\n");
        buffer.append("--myboundary2\r\n\r\n");
        assert_eq!(buffer.try_extract_frame(), Some("1".to_owned()));
        assert_eq!(buffer.try_extract_frame(), Some("2".to_owned()));
        assert_eq!(buffer.try_extract_frame(), None);
    }

    #[test]
    fn test_ok_2b() {
        let mut buffer = Buffer::new("myboundary".to_owned());
        buffer.append("--myboundary1\r\n\r\n");
        assert_eq!(buffer.try_extract_frame(), Some("1".to_owned()));
        assert_eq!(buffer.try_extract_frame(), None);
        buffer.append("--myboundary2\r\n\r\n");
        assert_eq!(buffer.try_extract_frame(), Some("2".to_owned()));
        assert_eq!(buffer.try_extract_frame(), None);
    }

    #[test]
    fn test_carray() {
        let mut buffer = Buffer::new("myboundary".to_owned());
        buffer.append("--myboundary1\r\n\r\n--myboundary");
        assert_eq!(buffer.try_extract_frame(), Some("1".to_owned()));
        assert_eq!(buffer.try_extract_frame(), None);
        buffer.append("2");
        assert_eq!(buffer.try_extract_frame(), None);
        buffer.append("\r\n\r\n");
        assert_eq!(buffer.try_extract_frame(), Some("2".to_owned()));
        assert_eq!(buffer.try_extract_frame(), None);
    }
}
