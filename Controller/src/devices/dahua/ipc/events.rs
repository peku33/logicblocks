use bytes::Bytes;
use failure::{err_msg, format_err, Error};
use futures::task::{Context, Poll};
use futures::Stream;
use lazy_static::lazy_static;
use regex::{Regex, RegexBuilder};
use serde::Serialize;
use serde_json::value::Value as JsonValue;
use std::collections::HashSet;
use std::pin::Pin;

// Event variant type
#[derive(Clone, Hash, PartialEq, Eq, Debug, Serialize)]
#[serde(tag = "type")]
pub enum EventSource {
    AudioMutation,
    CrossLineDetection { rule_id: u64, direction: String },
    CrossRegionDetection { rule_id: u64, direction: String },
    SceneChange,
    VideoBlind,
    VideoMotion { region: String },
}
impl EventSource {
    fn from_code_data(
        code: &str,
        data: Option<JsonValue>,
    ) -> Result<Self, Error> {
        match code {
            "AudioMutation" => Ok(Self::AudioMutation),
            "CrossLineDetection" => {
                let (rule_id, direction) = Self::extract_ivs_rule_id_direction(data)?;
                Ok(Self::CrossLineDetection { rule_id, direction })
            }
            "CrossRegionDetection" => {
                let (rule_id, direction) = Self::extract_ivs_rule_id_direction(data)?;
                Ok(Self::CrossRegionDetection { rule_id, direction })
            }
            "SceneChange" => Ok(Self::SceneChange),
            "VideoBlind" => Ok(Self::VideoBlind),
            "VideoMotion" => {
                let data_object = data
                    .as_ref()
                    .ok_or_else(|| err_msg("missing data for event"))?
                    .as_object()
                    .ok_or_else(|| err_msg("data for event is not object"))?;

                let regions_array = data_object
                    .get("RegionName")
                    .ok_or_else(|| err_msg("missing RegionName"))?
                    .as_array()
                    .ok_or_else(|| err_msg("RegionName is not array"))?;

                if regions_array.len() != 1 {
                    return Err(err_msg("regions array size must be 1"));
                }
                let region = regions_array
                    .get(0)
                    .unwrap()
                    .as_str()
                    .ok_or_else(|| err_msg("region must be string"))?
                    .to_owned();

                Ok(EventSource::VideoMotion { region })
            }
            _ => Err(format_err!("unrecognized event: {}", code)),
        }
    }

    fn extract_ivs_rule_id_direction(data: Option<JsonValue>) -> Result<(u64, String), Error> {
        let data_object = data
            .as_ref()
            .ok_or_else(|| err_msg("missing data for event"))?
            .as_object()
            .ok_or_else(|| err_msg("data for event is not object"))?;

        let rule_id = data_object
            .get("RuleId")
            .ok_or_else(|| err_msg("missing RuleId"))?
            .as_u64()
            .ok_or_else(|| err_msg("RuleId is not int"))?;

        let direction = data_object
            .get("Direction")
            .ok_or_else(|| err_msg("missing Direction"))?
            .as_str()
            .ok_or_else(|| err_msg("Direction is not int"))?
            .to_owned();

        Ok((rule_id, direction))
    }
}

// Event direction (On / Off)
#[derive(Copy, Clone, Hash, PartialEq, Eq, Debug)]
pub enum EventTransitionDirection {
    START,
    STOP,
}
impl EventTransitionDirection {
    fn from_str(direction: &str) -> Result<Self, Error> {
        match direction {
            "Start" => Ok(EventTransitionDirection::START),
            "Stop" => Ok(EventTransitionDirection::STOP),
            _ => Err(format_err!("unrecognized direction: {}", direction)),
        }
    }
}

// Transition = Event + Direction
#[derive(Clone, Hash, PartialEq, Eq, Debug)]
pub struct EventTransition {
    source: EventSource,
    direction: EventTransitionDirection,
}
impl EventTransition {
    fn from_item(item: &str) -> Result<Self, Error> {
        lazy_static! {
            static ref EVENT_TRANSITION_DETAILS_REGEX: Regex =
                RegexBuilder::new(r"^Code=(\w+);action=(\w+);index=0(;data=(.+))?$")
                    .dot_matches_new_line(true)
                    .build()
                    .unwrap();
        }

        let captures = EVENT_TRANSITION_DETAILS_REGEX
            .captures(item)
            .ok_or_else(|| err_msg("event item does not match required pattern"))?;

        let code = captures.get(1).unwrap().as_str();
        let direction = captures.get(2).unwrap().as_str();
        let data: Option<JsonValue> = match captures.get(4) {
            Some(data) => Some(serde_json::from_str(data.as_str())?),
            None => None,
        };

        let source = EventSource::from_code_data(code, data)?;
        let direction = EventTransitionDirection::from_str(direction)?;

        Ok(Self { source, direction })
    }
}

// Collection, tracks on/off events
pub struct EventsTracker {
    active: HashSet<EventSource>,
}
impl EventsTracker {
    pub fn new() -> Self {
        Self {
            active: HashSet::new(),
        }
    }

    pub fn clear(&mut self) {
        self.active.clear();
    }

    pub fn consume_event_transition(
        &mut self,
        event_transition: EventTransition,
    ) {
        match event_transition.direction {
            EventTransitionDirection::START => {
                if let Some(event_transition_source) = self.active.replace(event_transition.source)
                {
                    log::warn!("duplicated active event: {:?}", event_transition_source);
                }
            }
            EventTransitionDirection::STOP => {
                if !self.active.remove(&event_transition.source) {
                    log::warn!("missing active event: {:?}", event_transition.source);
                }
            }
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = &EventSource> {
        self.active.iter()
    }
}

// Gives access to event stream
pub struct EventStreamBuilder {
    host: http::uri::Authority,
    login: String,
    password: String,

    client: hyper::Client<hyper::client::HttpConnector>,
}
impl EventStreamBuilder {
    pub fn new(
        host: http::uri::Authority,
        login: String,
        password: String,
    ) -> Self {
        EventStreamBuilder {
            host,
            login,
            password,

            client: hyper::Client::new(),
        }
    }

    fn uri_build(&self) -> http::uri::Uri {
        http::uri::Builder::new()
            .scheme(http::uri::Scheme::HTTP)
            .authority(self.host.clone())
            .path_and_query(
                "\
                 /cgi-bin/eventManager.cgi?action=attach&codes=[\
                 AudioMutation,\
                 CrossLineDetection,\
                 CrossRegionDetection,\
                 SceneChange,\
                 VideoBlind,\
                 VideoMotion\
                 ]\
                 ",
            )
            .build()
            .unwrap()
    }
    fn request_build(
        &self,
        authorization_header: Option<String>,
    ) -> hyper::Request<hyper::Body> {
        let mut request = hyper::Request::builder()
            .method(http::Method::GET)
            .uri(self.uri_build())
            .body(hyper::Body::empty())
            .unwrap();

        if let Some(authorization_header) = authorization_header {
            request.headers_mut().append(
                http::header::AUTHORIZATION,
                http::header::HeaderValue::from_str(&authorization_header).unwrap(),
            );
        }

        request
    }

    async fn request(&self) -> Result<(hyper::Body, String), Error> {
        fn extract_boundary(response: &hyper::Response<hyper::Body>) -> Result<String, Error> {
            let content_type = response
                .headers()
                .get(http::header::CONTENT_TYPE)
                .ok_or_else(|| err_msg("missing CONTENT_TYPE"))?
                .to_str()?;

            lazy_static! {
                static ref BOUNDARY_FROM_CONTEXT_TYPE: Regex =
                    Regex::new(r"^multipart/x-mixed-replace; boundary=(\w+)$").unwrap();
            }
            let captures = BOUNDARY_FROM_CONTEXT_TYPE
                .captures(content_type)
                .ok_or_else(|| err_msg("unable to extract boundary from CONTENT_TYPE"))?;
            let boundary = captures.get(1).unwrap().as_str();
            if boundary.is_empty() {
                return Err(err_msg("returned boundary is empty"));
            }

            Ok(boundary.to_owned())
        }
        fn extract_result(
            response: hyper::Response<hyper::Body>
        ) -> Result<(hyper::Body, String), Error> {
            let boundary = extract_boundary(&response)?;
            Ok((response.into_body(), boundary))
        }

        // Sometimes cameras passes the traffic based on last api calls without authorization, I called this "implicit auth"
        // For most cases it asks for digest auth, I called the second stage (authed) "explicit auth"
        let implicit_request = self.request_build(None);
        let implicit_request_uri_path_and_query =
            implicit_request.uri().path_and_query().unwrap().clone();
        let implicit_response = self.client.request(implicit_request).await?;
        let implicit_status = implicit_response.status();
        if http::StatusCode::OK == implicit_status {
            return Ok(extract_result(implicit_response)?);
        }
        if http::StatusCode::UNAUTHORIZED != implicit_status {
            return Err(format_err!(
                "no-auth request failed with status: {}",
                implicit_status
            ));
        }

        // http::StatusCode::UNAUTHORIZED here
        let www_authenticate = implicit_response
            .headers()
            .get(http::header::WWW_AUTHENTICATE)
            .ok_or_else(|| err_msg("missing WWW_AUTHENTICATE header for UNAUTHORIZED"))?
            .to_str()?;

        let authorization = digest_auth::parse(www_authenticate)?
            .respond(&digest_auth::AuthContext::new(
                &self.login,
                &self.password,
                implicit_request_uri_path_and_query.as_str(),
            ))?
            .to_header_string();

        let explicit_request = self.request_build(Some(authorization));
        let explicit_response = self.client.request(explicit_request).await?;
        let explicit_status = explicit_response.status();
        if http::StatusCode::OK != explicit_status {
            return Err(format_err!(
                "authed request failed with status: {}",
                explicit_status
            ));
        }

        Ok(extract_result(explicit_response)?)
    }

    pub async fn get_event_stream(&self) -> Result<EventStream, Error> {
        let (body, boundary) = self.request().await?;
        Ok(EventStream::new(body, boundary))
    }
}

// Main event stream, yielding event items
#[derive(Debug)]
pub struct EventStream {
    body: hyper::Body,
    x_mixed_replace_buffer: super::x_mixed_replace::Buffer,
}
impl EventStream {
    fn new(
        body: hyper::Body,
        boundary: String,
    ) -> Self {
        EventStream {
            body,
            x_mixed_replace_buffer: super::x_mixed_replace::Buffer::new(boundary),
        }
    }
    fn x_mixed_replace_buffer_yield_one(&mut self) -> Option<EventTransition> {
        while let Some(item) = self.x_mixed_replace_buffer.try_extract_frame() {
            match EventTransition::from_item(&item) {
                Ok(item) => return Some(item),
                Err(error) => log::error!("error during frame extraction: {}", error),
            }
        }
        None
    }
    fn x_mixed_replace_buffer_append(
        &mut self,
        item: Result<Bytes, hyper::error::Error>,
    ) {
        let item: Result<(), Error> = try {
            let item = item?;
            let item = String::from_utf8(item.to_vec())?;
            self.x_mixed_replace_buffer.append(&item);
        };
        if let Err(error) = item {
            log::error!("error during frame appending: {}", error);
        }
    }
}
impl Stream for EventStream {
    type Item = EventTransition;

    fn poll_next(
        self: Pin<&mut Self>,
        cx: &mut Context,
    ) -> Poll<Option<Self::Item>> {
        let self_ = self.get_mut();

        if let Some(item) = self_.x_mixed_replace_buffer_yield_one() {
            return Poll::Ready(Some(item));
        }

        if let Poll::Ready(item) = Pin::new(&mut self_.body).poll_next(cx) {
            if let Some(item) = item {
                self_.x_mixed_replace_buffer_append(item);
                if let Some(item) = self_.x_mixed_replace_buffer_yield_one() {
                    Poll::Ready(Some(item))
                } else {
                    cx.waker().wake_by_ref();
                    Poll::Pending
                }
            } else {
                Poll::Ready(None)
            }
        } else {
            Poll::Pending
        }
    }
}
