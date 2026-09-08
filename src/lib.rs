#![forbid(unsafe_code)]

//! The `AsyncAPI` content contract — a technology of `xmip-core-contract`.
//!
//! Two claims, decided 2026-09-07 (ADR-0042): **well-formedness is a given**
//! and **conformance is a given once a contract is named**.
//!
//! Well-formed here is a *sound description*: a JSON object that says which
//! `AsyncAPI` it is (`asyncapi` 2.x or 3.x), names its `info.title` and
//! `info.version`, keeps `channels` as an object, keeps 3.x `operations` as
//! an object whose every entry names a channel by `$ref`, and refers with
//! `$ref` only to what the document itself holds. A description whose
//! operation points at a channel it does not have is the one every generated
//! client breaks on.
//!
//! Conformance is the *channel*: a Location that names this contract with
//! `orders/placed` bound has every description held to defining that
//! channel — by its key in 2.x, by its key or its `address` in 3.x. A
//! description in YAML is the same document in another notation and is the
//! next layer here; the message schemas inside are
//! `xmip-core-contract-json-schema`'s.

use contract::{
    Contract, ContractDescriptor, ContractError, ContractFactory, ContractId, ValidationIssue,
    ValidationResult,
};
use serde_json::Value;
use stream::Stream;

/// The `AsyncAPI` contract, bare or bound to a channel.
pub struct AsyncApi {
    descriptor: ContractDescriptor,
    channel: Option<String>,
}

impl AsyncApi {
    /// A sound description, of any channels.
    #[must_use]
    pub fn new() -> Self {
        Self {
            descriptor: descriptor("asyncapi"),
            channel: None,
        }
    }

    /// A sound description that defines `channel`.
    #[must_use]
    pub fn of(channel: &str) -> Self {
        Self {
            descriptor: descriptor(&format!("asyncapi:{channel}")),
            channel: Some(channel.to_string()),
        }
    }

    /// Whether a channel is bound.
    #[must_use]
    pub fn is_bound(&self) -> bool {
        self.channel.is_some()
    }
}

impl Default for AsyncApi {
    fn default() -> Self {
        Self::new()
    }
}

fn descriptor(id: &str) -> ContractDescriptor {
    ContractDescriptor {
        id: ContractId(id.to_string()),
        version: "1".to_string(),
        representation: "application/vnd.aai.asyncapi+json".to_string(),
    }
}

/// Every departure of `document` from a sound description.
fn soundness(document: &Value) -> Vec<ValidationIssue> {
    let mut issues = Vec::new();
    let Some(object) = document.as_object() else {
        return vec![issue(
            "malformed",
            "the document is not a JSON object",
            None,
        )];
    };
    let version = object.get("asyncapi").and_then(Value::as_str).unwrap_or("");
    if !(version.starts_with("2.") || version.starts_with("3.")) {
        issues.push(issue(
            "structure",
            "neither asyncapi 2.x nor 3.x is declared",
            Some("asyncapi".into()),
        ));
    }
    for field in ["title", "version"] {
        if object
            .get("info")
            .and_then(|info| info.get(field))
            .and_then(Value::as_str)
            .is_none()
        {
            issues.push(issue(
                "structure",
                &format!("info.{field} is missing"),
                Some("info".into()),
            ));
        }
    }
    match object.get("channels") {
        Some(Value::Object(_)) | None if version.starts_with("3.") => {}
        Some(Value::Object(_)) => {}
        Some(_) => issues.push(issue(
            "structure",
            "channels is not an object",
            Some("channels".into()),
        )),
        None => issues.push(issue(
            "structure",
            "channels is missing",
            Some("channels".into()),
        )),
    }
    if version.starts_with("3.")
        && let Some(operations) = object.get("operations")
    {
        match operations.as_object() {
            Some(operations) => {
                for (name, operation) in operations {
                    if operation
                        .get("channel")
                        .and_then(|c| c.get("$ref"))
                        .and_then(Value::as_str)
                        .is_none()
                    {
                        issues.push(issue(
                            "structure",
                            "an operation without a channel $ref",
                            Some(format!("operations.{name}")),
                        ));
                    }
                }
            }
            None => issues.push(issue(
                "structure",
                "operations is not an object",
                Some("operations".into()),
            )),
        }
    }
    references(document, document, "", &mut issues);
    issues
}

/// Every `$ref` under `value` that begins with `#` and does not land.
fn references(root: &Value, value: &Value, path: &str, issues: &mut Vec<ValidationIssue>) {
    match value {
        Value::Object(object) => {
            if let Some(Value::String(target)) = object.get("$ref")
                && let Some(pointer) = target.strip_prefix('#')
                && root.pointer(pointer).is_none()
            {
                issues.push(issue(
                    "reference",
                    &format!("$ref {target} does not land"),
                    Some(path.trim_start_matches('.').to_string()),
                ));
            }
            for (key, child) in object {
                references(root, child, &format!("{path}.{key}"), issues);
            }
        }
        Value::Array(items) => {
            for (i, child) in items.iter().enumerate() {
                references(root, child, &format!("{path}[{i}]"), issues);
            }
        }
        _ => {}
    }
}

/// Whether `document` defines `channel`, by key or by 3.x address.
fn defines(document: &Value, channel: &str) -> bool {
    document
        .get("channels")
        .and_then(Value::as_object)
        .is_some_and(|channels| {
            channels.contains_key(channel)
                || channels
                    .values()
                    .any(|c| c.get("address").and_then(Value::as_str) == Some(channel))
        })
}

impl Contract for AsyncApi {
    fn descriptor(&self) -> &ContractDescriptor {
        &self.descriptor
    }

    fn identify(&self, stream: &Stream) -> Result<bool, ContractError> {
        if stream.media_type().is_some_and(|m| {
            let base = m.split(';').next().unwrap_or("").trim();
            base.eq_ignore_ascii_case("application/vnd.aai.asyncapi+json")
                || base.eq_ignore_ascii_case("application/vnd.aai.asyncapi")
        }) {
            return Ok(true);
        }
        let Ok(text) = std::str::from_utf8(stream.bytes()) else {
            return Ok(false);
        };
        Ok(text.contains("\"asyncapi\""))
    }

    fn validate(&self, stream: &Stream) -> Result<ValidationResult, ContractError> {
        let document: Value = match serde_json::from_slice(stream.bytes()) {
            Ok(document) => document,
            Err(error) => {
                return Ok(result(vec![issue(
                    "malformed",
                    &format!("not JSON: {error}"),
                    None,
                )]));
            }
        };
        let mut issues = soundness(&document);
        if let Some(channel) = &self.channel
            && !defines(&document, channel)
        {
            issues.push(issue(
                "channel",
                &format!("does not define channel {channel}"),
                Some("channels".into()),
            ));
        }
        Ok(result(issues))
    }
}

fn issue(code: &str, message: &str, path: Option<String>) -> ValidationIssue {
    ValidationIssue {
        code: code.to_string(),
        message: message.to_string(),
        path,
    }
}

fn result(issues: Vec<ValidationIssue>) -> ValidationResult {
    ValidationResult {
        valid: issues.is_empty(),
        issues,
    }
}

/// Loads the contract a Location names: an empty reference is the bare
/// contract, anything else a channel, `orders/placed`.
pub struct AsyncApiFactory;

impl ContractFactory for AsyncApiFactory {
    fn technology(&self) -> &'static str {
        "asyncapi"
    }

    fn load(&self, reference: &str) -> Result<Box<dyn Contract>, ContractError> {
        let reference = reference.trim();
        if reference.is_empty() {
            return Ok(Box::new(AsyncApi::new()));
        }
        Ok(Box::new(AsyncApi::of(reference)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use xcore::StreamId;

    const V2: &str = r##"{
        "asyncapi": "2.6.0",
        "info": {"title": "Orders", "version": "1"},
        "channels": {
            "orders/placed": {"subscribe": {"message": {"$ref": "#/components/messages/Placed"}}}
        },
        "components": {"messages": {"Placed": {"payload": {"type": "object"}}}}
    }"##;

    const V3: &str = r##"{
        "asyncapi": "3.0.0",
        "info": {"title": "Orders", "version": "1"},
        "channels": {"placed": {"address": "orders/placed"}},
        "operations": {"onPlaced": {"action": "receive", "channel": {"$ref": "#/channels/placed"}}}
    }"##;

    fn stream(text: &str, media_type: Option<&str>) -> Stream {
        Stream::new(
            StreamId::new(1),
            text.as_bytes().to_vec(),
            media_type.map(str::to_string),
        )
    }

    #[test]
    fn a_sound_description_holds_bare_and_bound_in_both_versions() {
        let bare = AsyncApi::new();
        assert!(bare.identify(&stream(V2, None)).expect("identify"));
        assert!(
            bare.identify(&stream("{}", Some("application/vnd.aai.asyncapi+json")))
                .expect("identify")
        );
        assert!(
            !bare
                .identify(&stream("{\"openapi\":1}", None))
                .expect("identify")
        );
        for text in [V2, V3] {
            let result = bare.validate(&stream(text, None)).expect("validate");
            assert!(result.valid, "{:?}", result.issues);
            let bound = AsyncApiFactory.load("orders/placed").expect("load");
            assert!(bound.validate(&stream(text, None)).expect("validate").valid);
        }
        assert!(
            AsyncApi::of("placed")
                .validate(&stream(V3, None))
                .expect("validate")
                .valid
        );
        assert_eq!(
            AsyncApiFactory
                .load("orders/placed")
                .expect("load")
                .descriptor()
                .id
                .0,
            "asyncapi:orders/placed"
        );
        assert!(AsyncApi::of("x").is_bound());
        assert!(
            !AsyncApiFactory
                .load(" ")
                .expect("bare")
                .descriptor()
                .id
                .0
                .contains(':')
        );
    }

    #[test]
    fn departures_are_named_with_their_paths() {
        let broken = V2
            .replace("#/components/messages/Placed", "#/components/messages/Gone")
            .replace("\"asyncapi\": \"2.6.0\"", "\"asyncapi\": \"1.0\"");
        let result = AsyncApi::new()
            .validate(&stream(&broken, None))
            .expect("validate");
        assert_eq!(result.issues.len(), 2, "{:?}", result.issues);
        assert_eq!(
            result.issues[0].message,
            "neither asyncapi 2.x nor 3.x is declared"
        );
        assert_eq!(
            result.issues[1].path.as_deref(),
            Some("channels.orders/placed.subscribe.message")
        );
        let no_channel = V3.replace(r##""channel": {"$ref": "#/channels/placed"}"##, "\"x\": 1");
        let result = AsyncApi::new()
            .validate(&stream(&no_channel, None))
            .expect("validate");
        assert_eq!(
            result.issues[0].message,
            "an operation without a channel $ref"
        );
        assert_eq!(
            result.issues[0].path.as_deref(),
            Some("operations.onPlaced")
        );
        let result = AsyncApi::of("orders/cancelled")
            .validate(&stream(V2, None))
            .expect("validate");
        assert_eq!(result.issues[0].code, "channel");
        assert_eq!(
            result.issues[0].message,
            "does not define channel orders/cancelled"
        );
    }

    #[test]
    fn what_is_not_a_description_does_not_hold() {
        let result = AsyncApi::new()
            .validate(&stream("7", None))
            .expect("validate");
        assert_eq!(result.issues[0].code, "malformed");
        let result = AsyncApi::new()
            .validate(&stream("{", None))
            .expect("validate");
        assert_eq!(result.issues[0].code, "malformed");
        let result = AsyncApi::new()
            .validate(&stream(
                r#"{"asyncapi":"2.0.0","info":{"title":"t","version":"1"}}"#,
                None,
            ))
            .expect("validate");
        assert_eq!(result.issues[0].message, "channels is missing");
        let result = AsyncApi::new()
            .validate(&stream(
                r#"{"asyncapi":"2.0.0","info":{},"channels":[]}"#,
                None,
            ))
            .expect("validate");
        assert_eq!(result.issues.len(), 3);
    }
}
