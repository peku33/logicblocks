use anyhow::{Context, Error};
use once_cell::sync::Lazy;
use regex::{Regex, RegexBuilder};
use std::{collections::VecDeque, str};
use xmltree::Element;

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

    pub fn try_extract(&mut self) -> Result<Option<Element>, Error> {
        static PATTERN: Lazy<Regex> = Lazy::new(|| {
            RegexBuilder::new("--boundary\r\nContent-Type: application/xml; charset=\"UTF-8\"\r\nContent-Length: (\\d+)\r\n\r\n")
                .dot_matches_new_line(true)
                .build()
                .unwrap()
        });

        let buffer = self.buffer.make_contiguous();
        let buffer = unsafe { str::from_utf8_unchecked(buffer) }; // SAFETY: buffer accepts &str only

        let capture = match PATTERN.captures(buffer) {
            Some(capture) => capture,
            None => return Ok(None),
        };

        let header_match = capture.get(0).unwrap();
        if header_match.start() != 0 {
            log::debug!("boundary not started on the beginning. noise?");
        }
        let header_end = header_match.end();

        let content_length = match capture
            .get(1)
            .unwrap()
            .as_str()
            .parse::<usize>()
            .context("content_length parse")
        {
            Ok(content_length) => content_length,
            Err(error) => {
                self.buffer.drain(0..header_end);
                return Err(error);
            }
        };

        // do we have whole message in the buffeR?
        if content_length - 1 > buffer.len() - header_end {
            return Ok(None);
        }

        let element =
            match Element::parse(&buffer.as_bytes()[header_end..header_end + content_length])
                .context("parse")
            {
                Ok(element) => element,
                Err(error) => {
                    self.buffer.drain(0..header_end + content_length);
                    return Err(error);
                }
            };

        self.buffer.drain(0..header_end + content_length);
        Ok(Some(element))
    }
}
#[cfg(test)]
mod tests_extractor {
    use super::Extractor;

    #[test]
    fn try_extract_1() {
        let mut extractor = Extractor::new();
        extractor.push("--boundary\r\nContent-Type: application/xml; charset=\"UTF-8\"\r\nContent-Length: 478\r\n\r\n<EventNotificationAlert version=\"1.0\" xmlns=\"http://www.hikvision.com/ver20/XMLSchema\">\r\n<ipAddress>10.0.2.101</ipAddress>\r\n<portNo>80</portNo>\r\n<protocol>HTTP</protocol>\r\n<macAddress>c0:56:e3:68:64:36</macAddress>\r\n<channelID>1</channelID>\r\n<dateTime>2020-11-07T14:40:23-00:00</dateTime>\r\n<activePostCount>0</activePostCount>\r\n<eventType>videoloss</eventType>\r\n<eventState>inactive</eventState>\r\n<eventDescription>videoloss alarm</eventDescription>\r\n</EventNotificationAlert>\r\n");

        assert!(extractor.try_extract().unwrap().is_some());
        assert!(extractor.try_extract().unwrap().is_none());
    }
    #[test]
    fn try_extract_2() {
        let mut extractor = Extractor::new();
        extractor.push(
            "--boundary\r\nContent-Type: application/xml; charset=\"UTF-8\"\r\nContent-Length: 478",
        );

        assert!(extractor.try_extract().unwrap().is_none());

        extractor.push("\r\n\r\n<EventNotificationAlert version=\"1.0\" xmlns=\"http://www.hikvision.com/ver20/XMLSchema\">\r\n<ipAddress>10.0.2.101</ipAddress>\r\n<portNo>80</portNo>\r\n<protocol>HTTP</protocol>\r\n<macAddress>c0:56:e3:68:64:36</macAddress>\r\n<channelID>1</channelID>\r\n<dateTime>2020-11-07T14:40:23-00:00</dateTime>\r\n<activePostCount>0</activePostCount>\r\n<eventType>videoloss</eventType>\r\n<eventState>inactive</eventState>\r\n<eventDescription>videoloss alarm</eventDescription>\r\n</EventNotificationAlert>\r\n");

        assert!(extractor.try_extract().unwrap().is_some());
    }
    #[test]
    fn try_extract_3() {
        let mut extractor = Extractor::new();
        extractor.push("--boundary\r\nContent-Type: application/xml; charset=\"UTF-8\"\r\nContent-Length: 478\r\n\r\n");

        assert!(extractor.try_extract().unwrap().is_none());

        extractor.push("<EventNotificationAlert version=\"1.0\" xmlns=\"http://www.hikvision.com/ver20/XMLSchema\">\r\n<ipAddress>10.0.2.101</ipAddress>\r\n<portNo>80</portNo>\r\n<protocol>HTTP</protocol>\r\n<macAddress>c0:56:e3:68:64:36</macAddress>\r\n<channelID>1</channelID>\r\n<dateTime>2020-11-07T14:40:23-00:00</dateTime>\r\n<activePostCount>0</activePostCount>\r\n<eventType>videoloss</eventType>\r\n<eventState>inactive</eventState>\r\n<eventDescription>videoloss alarm</eventDescription>\r\n</EventNotificationAlert>\r\n");

        assert!(extractor.try_extract().unwrap().is_some());
        assert!(extractor.try_extract().unwrap().is_none());
    }
    #[test]
    fn try_extract_4() {
        let mut extractor = Extractor::new();
        extractor.push("--boundary\r\nContent-Type: application/xml; charset=\"UTF-8\"\r\nContent-Length: 478\r\n\r\n<EventNotificationAlert version=\"1.0\" xmlns=\"http://www.hikvision.com/ver20/XMLSchema\">\r\n<ipAddress>10.0.2.101</ipAddress>\r\n<portNo>80</portNo>\r\n<protocol>HTTP</protocol>\r\n<macAddress>c0:56:e3:68:64:36</macAddress>\r\n<channelID>1</channelID>\r\n<dateTime>2020-11-07T14:40:23-00:00</dateTime>\r\n<activePostCount>0</activePostCount>\r\n<eventType>videoloss</eventType>\r\n<eventState>inactive</eventState>\r\n<eventDescription>videoloss alarm</eventDescription>\r\n</EventNotificationAlert>");

        assert!(extractor.try_extract().unwrap().is_none());

        extractor.push("\r\n");

        assert!(extractor.try_extract().unwrap().is_some());
        assert!(extractor.try_extract().unwrap().is_none());
    }
    #[test]
    fn try_extract_5() {
        let mut extractor = Extractor::new();
        extractor.push("--boundary\r\nContent-Type: application/xml; charset=\"UTF-8\"\r\nContent-Length: 478\r\n\r\n<EventNotificationAlert version=\"1.0\" xmlns=\"http://www.hikvision.com/ver20/XMLSchema\">\r\n<ipAddress>10.0.2.101</ipAddress>\r\n<portNo>80</portNo>\r\n<protocol>HTTP</protocol>\r\n<macAddress>c0:56:e3:68:64:36</macAddress>\r\n<channelID>1</channelID>\r\n<dateTime>2020-11-07T14:40:23-00:00</dateTime>\r\n<activePostCount>0</activePostCount>\r\n<eventType>videoloss</eventType>\r\n<eventState>inactive</eventState>\r\n<eventDescription>videoloss alarm</eventDescription>\r\n</EventNotificationAlert>\r\n");
        extractor.push("--boundary\r\nContent-Type: application/xml; charset=\"UTF-8\"\r\nContent-Length: 478\r\n\r\n<EventNotificationAlert version=\"1.0\" xmlns=\"http://www.hikvision.com/ver20/XMLSchema\">\r\n<ipAddress>10.0.2.101</ipAddress>\r\n<portNo>80</portNo>\r\n<protocol>HTTP</protocol>\r\n<macAddress>c0:56:e3:68:64:36</macAddress>\r\n<channelID>1</channelID>\r\n<dateTime>2020-11-07T14:40:23-00:00</dateTime>\r\n<activePostCount>0</activePostCount>\r\n<eventType>videoloss</eventType>\r\n<eventState>inactive</eventState>\r\n<eventDescription>videoloss alarm</eventDescription>\r\n</EventNotificationAlert>\r\n");

        assert!(extractor.try_extract().unwrap().is_some());
        assert!(extractor.try_extract().unwrap().is_some());
        assert!(extractor.try_extract().unwrap().is_none());
    }
    #[test]
    fn try_extract_6() {
        let mut extractor = Extractor::new();
        extractor.push("--boundary\r\nContent-Type: application/xml; charset=\"UTF-8\"\r\nContent-Length: 478\r\n\r\n<EventNotificationAlert version=\"1.0\" xmlns=\"http://www.hikvision.com/ver20/XMLSchema\">\r\n<ipAddress>10.0.2.101</ipAddress>\r\n<portNo>80</portNo>\r\n<protocol>HTTP</protocol>\r\n<macAddress>c0:56:e3:68:64:36</macAddress>\r\n<channelID>1</channelID>\r\n<dateTime>2020-11-07T14:40:23-00:00</dateTime>\r\n<activePostCount>0</activePostCount>\r\n<eventType>videoloss</eventType>\r\n<eventState>inactive</eventState>\r\n<eventDescription>videoloss alarm</eventDescription>\r\n</EventNotificationAlert>\r\n");
        extractor.push("--boundary\r\nContent-Type: application/xml; charset=\"UTF-8\"\r\nContent-Length: 478\r\n\r\n<EventNotificationAlert version=\"1.0\" xmlns=\"http://www.hikvision.com/ver20/XMLSchema\">\r\n<ipAddress>10.0.2.101</ipAddress>\r\n<portNo>80</portNo>\r\n<protocol>HTTP</protocol>\r\n<macAddress>c0:56:e3:68:64:36</macAddress>\r\n<channelID>1</channelID>\r\n<dateTime>2020-11-07T14:40:23-00:00</dateTime>\r\n<activePostCount>0</activePostCount>\r\n<eventType>videoloss</eventType>\r\n<eventState>inactive</eventState>\r\n<eventDescription>videoloss alarm</eventDescription>\r\n</EventNotificationAlert>");

        assert!(extractor.try_extract().unwrap().is_some());
        assert!(extractor.try_extract().unwrap().is_none());

        extractor.push("\r\n");

        assert!(extractor.try_extract().unwrap().is_some());
        assert!(extractor.try_extract().unwrap().is_none());
    }
}
