use crate::scanner::FileMetadata;

pub fn to_json(results: &[FileMetadata], pretty: bool) -> String {
    if pretty {
        serde_json::to_string_pretty(results).unwrap_or_else(|_| "[]".into())
    } else {
        serde_json::to_string(results).unwrap_or_else(|_| "[]".into())
    }
}
