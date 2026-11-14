//! VIS Nexo's narrow Android handoff: no tokens, file paths or executable arguments.
use serde::Deserialize;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Request {
    schema: u8,
    pub name: String,
    pub owner: String,
    pub instance: String,
    pub server: String,
}

fn parse(raw: &str) -> Option<Request> {
    if raw.len() > 1024 { return None; }
    let value: Request = serde_json::from_str(raw).ok()?;
    if value.owner.len() < 3 || value.owner.len() > 24
        || !value.owner.bytes().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == b'_')
        || value.instance.len() != 32 || !value.instance.bytes().all(|c| c.is_ascii_hexdigit())
        || value.schema != 1 || value.name.is_empty() || value.name.len() > 16
        || !value.name.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'_')
        || value.server.len() > 253
        || !value.server.bytes().all(|c| c.is_ascii_alphanumeric() || b".-:[]".contains(&c))
    { return None; }
    Some(value)
}

#[cfg(target_os = "android")]
pub(crate) fn request() -> Option<Request> {
    use jni::objects::{JObject, JString, JValue};
    let raw = crate::platform::with_context(|env, activity| {
        let intent = env.call_method(activity, "getIntent", "()Landroid/content/Intent;", &[])?.l()?;
        if intent.is_null() { return Ok(None); }
        let key = JObject::from(env.new_string("vis_nexo_launch")?);
        let value = env.call_method(&intent, "getStringExtra", "(Ljava/lang/String;)Ljava/lang/String;", &[JValue::Object(&key)])?.l()?;
        if value.is_null() { return Ok(None); }
        let value = JString::from(value);
        let text: String = env.get_string(&value)?.into();
        Ok(Some(text))
    }).flatten()?;
    parse(&raw)
}

#[cfg(test)]
mod tests {
    use super::parse;
    #[test]
    fn handoff_accepts_identity_and_server_but_no_paths_or_credentials() {
        assert!(parse(r#"{"schema":1,"name":"VisTest","owner":"vistest","instance":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa","server":"mc.example:25565"}"#).is_some());
        for value in [
            r#"{"schema":1,"owner":"vistest","instance":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa","name":"../owner","server":""}"#,
            r#"{"schema":1,"owner":"vistest","instance":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa","name":"VisTest","server":"https://evil.test/path"}"#,
            r#"{"schema":1,"owner":"vistest","instance":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa","name":"VisTest","server":"","token":"secret"}"#,
            r#"{"schema":2,"owner":"vistest","instance":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa","name":"VisTest","server":""}"#,
            r#"{"schema":1,"owner":"vistest","instance":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa","name":"VisTest","server":"","game_dir":"/data/other"}"#
        ] { assert!(parse(value).is_none()); }
    }
}
