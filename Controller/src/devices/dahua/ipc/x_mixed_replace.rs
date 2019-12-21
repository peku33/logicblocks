use regex::{Regex, RegexBuilder};

#[derive(Debug)]
pub struct Buffer {
    buffer: String,
    frame_regex: Regex,
}
impl Buffer {
    pub fn new(boundary: String) -> Self {
        let frame_regex = RegexBuilder::new(&format!(
            r"--{}(\r\n)Content-Type: text/plain(\r\n)Content-Length:(\d+)(\r\n){{1,2}}(.+?)(\r\n\r\n)",
            boundary
        ))
        .dot_matches_new_line(true)
        .build()
        .unwrap();

        return Self {
            buffer: String::new(),
            frame_regex,
        };
    }
    pub fn try_extract_frame(&mut self) -> Option<String> {
        let captures = self.frame_regex.captures(&self.buffer)?;

        // Match frame boundaries
        let match_all = captures.get(0).unwrap();
        if match_all.start() != 0 {
            log::warn!(
                "detected offset ({}) in frame, probably wrongly formatted data",
                match_all.start()
            );
        }

        // Match content length
        let content_length = usize::from_str_radix(captures.get(3).unwrap().as_str(), 10);

        // Extract frame contents
        let content = captures.get(5).unwrap().as_str().to_owned();

        // Cut frame
        self.buffer = self.buffer[match_all.end()..].to_owned();

        // Final checks
        let content_length = match content_length {
            Ok(content_length) => content_length,
            Err(error) => {
                log::warn!("Cannot decode content_length: {}", error);
                return None;
            }
        };

        if content_length != content.len() {
            log::warn!(
                "Mismatched content_length ({}) and content.len() ({})",
                content_length,
                content.len()
            );
            return None;
        }

        return Some(content.to_owned());
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
mod tests_buffer {
    use super::Buffer;

    #[test]
    fn test_empty() {
        let mut buffer = Buffer::new("myboundary".to_owned());
        assert_eq!(buffer.try_extract_frame(), None);
    }

    #[test]
    fn test_1() {
        let mut buffer = Buffer::new("myboundary".to_owned());
        assert_eq!(buffer.try_extract_frame(), None);
        buffer.append("--myboundary\r\nContent-Type: text/plain\r\nContent-Length:39\r\n\r\nCode=AudioMutation;action=Start;index=0\r\n\r\n");
        assert_eq!(
            buffer.try_extract_frame(),
            Some("Code=AudioMutation;action=Start;index=0".to_owned())
        );
        assert_eq!(buffer.try_extract_frame(), None);
    }
    #[test]
    fn test_2() {
        let mut buffer = Buffer::new("myboundary".to_owned());
        assert_eq!(buffer.try_extract_frame(), None);
        buffer.append("--myboundary\r\nContent-Type: text/plain\r\nContent-Length:39\r\nCode=AudioMutation;action=Start;index=0\r\n\r\n");
        assert_eq!(
            buffer.try_extract_frame(),
            Some("Code=AudioMutation;action=Start;index=0".to_owned())
        );
        assert_eq!(buffer.try_extract_frame(), None);
    }
    #[test]
    fn test_3() {
        let mut buffer = Buffer::new("myboundary".to_owned());
        buffer.append("--myboundary\r\nContent-Type: text/plain\r\nContent-Length:39\r\nCode=AudioMutation;action=Start;index=0\r\n\r\n");
        buffer.append("--myboundary\r\nContent-Type: text/plain\r\nContent-Length:38\r\nCode=AudioMutation;action=Stop;index=0\r\n\r\n");
        assert_eq!(
            buffer.try_extract_frame(),
            Some("Code=AudioMutation;action=Start;index=0".to_owned())
        );
        assert_eq!(
            buffer.try_extract_frame(),
            Some("Code=AudioMutation;action=Stop;index=0".to_owned())
        );
        assert_eq!(buffer.try_extract_frame(), None);
    }
    #[test]
    fn test_4() {
        let mut buffer = Buffer::new("myboundary".to_owned());
        buffer.append("someshittttt--myboundary\r\nContent-Type: text/plain\r\nContent-Length:39\r\n\r\nCode=AudioMutation;action=Start;index=0\r\n\r\nsomeshit2");
        buffer.append("moreshitheeere--myboundary\r\nContent-Type: text/plain\r\nContent-Length:38\r\n\r\nCode=AudioMutation;action=Stop;index=0\r\n\r\nandhere");
        assert_eq!(
            buffer.try_extract_frame(),
            Some("Code=AudioMutation;action=Start;index=0".to_owned())
        );
        assert_eq!(
            buffer.try_extract_frame(),
            Some("Code=AudioMutation;action=Stop;index=0".to_owned())
        );
        assert_eq!(buffer.try_extract_frame(), None);
    }
}
