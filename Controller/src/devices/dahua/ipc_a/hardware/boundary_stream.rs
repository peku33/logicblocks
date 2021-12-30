use anyhow::{ensure, Context, Error};
use lazy_static::lazy_static;
use regex::{Regex, RegexBuilder};
use std::{collections::VecDeque, str};

#[derive(Debug)]
pub struct Extractor {
    buffer: VecDeque<u8>,
}
impl Extractor {
    pub fn new() -> Self {
        let buffer = VecDeque::<u8>::new();

        Self { buffer }
    }

    pub fn push(
        &mut self,
        chunk: &str,
    ) {
        self.buffer.extend(chunk.bytes());
    }

    pub fn try_extract(&mut self) -> Result<Option<String>, Error> {
        lazy_static! {
            static ref PATTERN: Regex = RegexBuilder::new(r"--myboundary(\r\n)?Content-Type: text/plain(\r\n)Content-Length:(\d+)(\r\n){1,2}(.+?)(\r\n\r\n)")
            .dot_matches_new_line(true)
            .build()
            .unwrap();
        }

        let buffer = self.buffer.make_contiguous();
        let buffer = unsafe { str::from_utf8_unchecked(buffer) }; // SAFETY: buffer accepts &str only

        let capture = match PATTERN.captures(buffer) {
            Some(capture) => capture,
            None => return Ok(None),
        };

        let element_match = capture.get(0).unwrap();
        if element_match.start() != 0 {
            log::trace!("boundary not started on the beginning. noise?");
        }
        let element_end = element_match.end();

        let element = Self::try_extract_capture(capture).context("try_extract_capture");

        self.buffer.drain(0..element_end);

        match element {
            Ok(element) => Ok(Some(element)),
            Err(error) => Err(error),
        }
    }

    fn try_extract_capture(capture: regex::Captures) -> Result<String, Error> {
        let content_length = capture
            .get(3)
            .unwrap()
            .as_str()
            .parse::<usize>()
            .context("content_length parse")?;

        let element = capture.get(5).unwrap().as_str().to_owned();
        ensure!(content_length == element.len());

        Ok(element)
    }
}

#[cfg(test)]
mod tests_extractor {
    use super::Extractor;

    #[test]
    fn test_empty() {
        let mut buffer = Extractor::new();
        assert!(buffer.try_extract().unwrap().is_none());
    }

    #[test]
    fn test_1() {
        let mut buffer = Extractor::new();
        assert!(buffer.try_extract().unwrap().is_none());
        buffer.push("--myboundary\r\nContent-Type: text/plain\r\nContent-Length:39\r\n\r\nCode=AudioMutation;action=Start;index=0\r\n\r\n");
        assert_eq!(
            &buffer.try_extract().unwrap().unwrap(),
            "Code=AudioMutation;action=Start;index=0",
        );
        assert!(buffer.try_extract().unwrap().is_none());
    }
    #[test]
    fn test_2() {
        let mut buffer = Extractor::new();
        assert!(buffer.try_extract().unwrap().is_none());
        buffer.push("--myboundary\r\nContent-Type: text/plain\r\nContent-Length:39\r\nCode=AudioMutation;action=Start;index=0\r\n\r\n");
        assert_eq!(
            &buffer.try_extract().unwrap().unwrap(),
            "Code=AudioMutation;action=Start;index=0"
        );
        assert!(buffer.try_extract().unwrap().is_none());
    }
    #[test]
    fn test_3() {
        let mut buffer = Extractor::new();
        buffer.push("--myboundary\r\nContent-Type: text/plain\r\nContent-Length:39\r\nCode=AudioMutation;action=Start;index=0\r\n\r\n");
        buffer.push("--myboundary\r\nContent-Type: text/plain\r\nContent-Length:38\r\nCode=AudioMutation;action=Stop;index=0\r\n\r\n");
        assert_eq!(
            &buffer.try_extract().unwrap().unwrap(),
            "Code=AudioMutation;action=Start;index=0"
        );
        assert_eq!(
            &buffer.try_extract().unwrap().unwrap(),
            "Code=AudioMutation;action=Stop;index=0",
        );
        assert!(buffer.try_extract().unwrap().is_none());
    }
    #[test]
    fn test_4() {
        let mut buffer = Extractor::new();
        buffer.push("someshittttt--myboundary\r\nContent-Type: text/plain\r\nContent-Length:39\r\n\r\nCode=AudioMutation;action=Start;index=0\r\n\r\nsomeshit2");
        buffer.push("moreshitheeere--myboundary\r\nContent-Type: text/plain\r\nContent-Length:38\r\n\r\nCode=AudioMutation;action=Stop;index=0\r\n\r\nandhere");
        assert_eq!(
            &buffer.try_extract().unwrap().unwrap(),
            "Code=AudioMutation;action=Start;index=0",
        );
        assert_eq!(
            &buffer.try_extract().unwrap().unwrap(),
            "Code=AudioMutation;action=Stop;index=0",
        );
        assert!(buffer.try_extract().unwrap().is_none());
    }

    #[test]
    fn test_5() {
        let mut buffer = Extractor::new();
        buffer.push("--myboundary\r\nContent-Type: text/plain\r\nContent-Length:39\r\n\r\nCode=AudioMutation;action=Start;index=0\r\n\r\n");
        buffer.push("--myboundary\r\nContent-Type: text/plain\r\nContent-Length:40\r\n\r\nCode=AudioMutation;action=Start;index=0\r\n\r\n");
        assert_eq!(
            &buffer.try_extract().unwrap().unwrap(),
            "Code=AudioMutation;action=Start;index=0",
        );
        assert!(buffer.try_extract().is_err());
        assert!(buffer.try_extract().unwrap().is_none());
    }
}
