use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct TextMeta {
    pub lines: usize,
    pub words: usize,
    pub characters: usize,
    pub bytes: usize,
    pub is_ascii: bool,
    pub is_utf8: bool,
}

pub fn analyze(data: Option<&[u8]>) -> Option<TextMeta> {
    let data = match data {
        Some(d) if !d.is_empty() => d,
        _ => return None,
    };

    let bytes = data.len();
    let is_ascii = data.iter().all(|&b| b < 128);
    let is_utf8 = is_ascii || std::str::from_utf8(data).is_ok();

    if !is_utf8 {
        return Some(TextMeta {
            lines: 0,
            words: 0,
            characters: 0,
            bytes,
            is_ascii: false,
            is_utf8: false,
        });
    }

    let text = std::str::from_utf8(data).unwrap_or("");
    let lines = text.lines().count().max(1);
    let words = text.split_whitespace().count();
    let characters = text.chars().count();

    Some(TextMeta {
        lines,
        words,
        characters,
        bytes,
        is_ascii,
        is_utf8,
    })
}
