//! Talking to a remote AG-UI endpoint: the `RunAgentInput` request, the SSE stream, and a forgiving parser.
//!
//! The client never follows a redirect, so the endpoint's bearer token can't be forwarded elsewhere.

use crate::agui::events;
use anyhow::{Context, Result, anyhow, bail};
use axum::body::Bytes;
use serde_json::{Value, json};
use std::sync::LazyLock;

/// A frame larger than this between events means the endpoint isn't speaking SSE.
const MAX_FRAME: usize = 2 * 1024 * 1024;

/// What a frame the endpoint used to say it's done looks like.
const DONE: &str = "DONE";

fn client() -> Result<&'static reqwest::Client> {
    static CLIENT: LazyLock<std::result::Result<reqwest::Client, reqwest::Error>> = LazyLock::new(|| {
        reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .connect_timeout(std::time::Duration::from_secs(30))
            .build()
    });
    CLIENT.as_ref().map_err(|_| anyhow!("Could not initialize the AG-UI client"))
}

/// The request a run sends: the conversation, this run's id, and the shared state.
pub fn run_input(thread_id: &str, run_id: &str, messages: &[Value], state: &Value) -> Value {
    json!({
        "threadId": thread_id,
        "runId": run_id,
        "messages": messages,
        "tools": [],
        "context": [],
        "state": state,
        "forwardedProps": {},
    })
}

/// A remote run's event stream.
pub struct Run {
    response: reqwest::Response,
    parser: Parser,
}

impl Run {
    /// Posts `input` and waits for the endpoint's first event.
    pub async fn start(endpoint: &reqwest::Url, token: Option<&str>, input: &Value) -> Result<Self> {
        let mut request =
            client()?.post(endpoint.clone()).header(reqwest::header::ACCEPT, "text/event-stream").json(input);
        if let Some(token) = token {
            request = request.bearer_auth(token);
        }
        let response = request.send().await.with_context(|| format!("reaching the AG-UI endpoint {endpoint}"))?;
        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            bail!("The AG-UI endpoint answered {status}: {}", events::payload(body.trim()));
        }
        Ok(Self { response, parser: Parser::new() })
    }

    /// The next event, or `None` when the endpoint ended the stream.
    pub async fn next(&mut self) -> Result<Option<Value>> {
        loop {
            if let Some(value) = self.parser.take()? {
                return Ok((value["type"] != DONE).then_some(value));
            }
            match self.response.chunk().await.context("reading the AG-UI stream")? {
                Some(bytes) => self.parser.push(&bytes),
                None => return Ok(self.parser.take()?.filter(|value| value["type"] != DONE)),
            }
        }
    }
}

/// Server-Sent Events as they arrive in arbitrary chunks: `data:` lines joined, comments ignored.
pub struct Parser {
    buffer: Vec<u8>,
}

impl Parser {
    pub fn new() -> Self {
        Self { buffer: Vec::new() }
    }

    pub fn push(&mut self, bytes: &Bytes) {
        self.buffer.extend_from_slice(bytes);
    }

    /// The next complete frame, if one has arrived.
    pub fn take(&mut self) -> Result<Option<Value>> {
        let Some((at, separator)) = self.separator() else {
            if self.buffer.len() > MAX_FRAME {
                bail!("The AG-UI endpoint sent a frame larger than {MAX_FRAME} bytes");
            }
            return Ok(None);
        };
        let frame = self.buffer.drain(..at + separator).collect::<Vec<_>>();
        let frame = String::from_utf8_lossy(&frame[..at]);
        let mut data = Vec::new();
        for line in frame.split('\n') {
            let line = line.strip_suffix('\r').unwrap_or(line);
            let Some(value) = line.strip_prefix("data:") else { continue };
            data.push(value.strip_prefix(' ').unwrap_or(value));
        }
        let data = data.join("\n");
        if data.trim().is_empty() {
            return Ok(None);
        }
        if data.trim() == "[DONE]" {
            return Ok(Some(json!({"type": DONE})));
        }
        serde_json::from_str(&data)
            .map(Some)
            .with_context(|| format!("The AG-UI endpoint sent a frame that isn't JSON: {}", events::payload(&data)))
    }

    /// The first blank line: `\n\n`, `\r\n\r\n`, or a `\n` followed by `\r\n`.
    fn separator(&self) -> Option<(usize, usize)> {
        for (at, window) in self.buffer.windows(4).enumerate() {
            if window == b"\r\n\r\n" {
                return Some((at, 4));
            }
        }
        for (at, window) in self.buffer.windows(3).enumerate() {
            if window == b"\n\r\n" {
                return Some((at, 3));
            }
        }
        self.buffer.windows(2).position(|window| window == b"\n\n").map(|at| (at, 2))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn chunk(text: &str) -> Bytes {
        Bytes::from(text.to_owned())
    }

    #[test]
    fn a_run_input_carries_the_conversation_and_this_run() {
        let input = run_input(
            "thread",
            "run",
            &[json!({"id": "1", "role": "user", "content": "hi"})],
            &json!({"view": "chat"}),
        );
        assert_eq!(input["threadId"], "thread");
        assert_eq!(input["runId"], "run");
        assert_eq!(input["messages"][0]["content"], "hi");
        assert_eq!(input["state"]["view"], "chat");
        assert_eq!(input["tools"], json!([]));
    }

    #[test]
    fn events_split_across_chunks_still_parse() {
        let mut parser = Parser::new();
        parser.push(&chunk("data: {\"type\":\"RUN_STARTED\","));
        assert!(parser.take().unwrap().is_none(), "no blank line yet");
        parser.push(&chunk("\"threadId\":\"t\"}\n\n"));
        let event = parser.take().unwrap().unwrap();
        assert_eq!(event["type"], "RUN_STARTED");
        assert_eq!(event["threadId"], "t");
    }

    #[test]
    fn comments_crlf_and_multiple_data_lines_are_handled() {
        let mut parser = Parser::new();
        parser.push(&chunk(": keep-alive\r\n\r\ndata: {\"type\":\"CUSTOM\",\r\ndata: \"name\":\"x\"}\r\n\r\n"));
        let event = parser.take().unwrap().unwrap();
        assert_eq!(event["type"], "CUSTOM");
        assert_eq!(event["name"], "x");
        assert!(parser.take().unwrap().is_none());
    }

    #[test]
    fn a_done_marker_ends_the_stream() {
        let mut parser = Parser::new();
        parser.push(&chunk("data: [DONE]\n\n"));
        assert_eq!(parser.take().unwrap().unwrap()["type"], DONE);
    }

    #[test]
    fn invalid_json_is_an_error_with_the_frame_in_it() {
        let mut parser = Parser::new();
        parser.push(&chunk("data: not json\n\n"));
        let error = parser.take().unwrap_err().to_string();
        assert!(error.contains("not json"), "{error}");
    }

    #[tokio::test]
    async fn a_remote_endpoint_streams_its_events_to_a_run() {
        use axum::{Router, routing::post};
        let app = Router::new().route(
            "/agui",
            post(|body: String| async move {
                assert!(body.contains("\"threadId\":\"t\""), "the run input reached the endpoint: {body}");
                let frames = futures::stream::iter(vec![
                    Ok::<_, std::convert::Infallible>(Bytes::from("data: {\"type\":\"RUN_STARTED\"}\n\n")),
                    Ok(Bytes::from("data: {\"type\":\"TEXT_MESSAGE_CONTENT\",\"delta\":\"hi\"}\n\n")),
                    Ok(Bytes::from("data: [DONE]\n\n")),
                ]);
                ([("content-type", "text/event-stream")], axum::body::Body::from_stream(frames))
            }),
        );
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let server = tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
        let endpoint = reqwest::Url::parse(&format!("http://{address}/agui")).unwrap();
        let input = run_input("t", "r", &[json!({"id": "1", "role": "user", "content": "hi"})], &json!({}));
        let mut run = Run::start(&endpoint, None, &input).await.unwrap();
        assert_eq!(run.next().await.unwrap().unwrap()["type"], "RUN_STARTED");
        assert_eq!(run.next().await.unwrap().unwrap()["delta"], "hi");
        assert!(run.next().await.unwrap().is_none());
        server.abort();
    }
}
