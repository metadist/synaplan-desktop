//! Detect when a chat turn is asking to *create* a file (image, sound,
//! video, document) rather than just talk about one.

/// What the user is asking the project to create.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GenerationKind {
    Image,
    Audio,
    Video,
    Document,
}

impl GenerationKind {
    pub fn as_str(self) -> &'static str {
        match self {
            GenerationKind::Image => "image",
            GenerationKind::Audio => "audio",
            GenerationKind::Video => "video",
            GenerationKind::Document => "document",
        }
    }

    pub fn parse(raw: &str) -> Option<Self> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "image" => Some(Self::Image),
            "audio" | "sound" | "speech" => Some(Self::Audio),
            "video" => Some(Self::Video),
            "document" | "doc" => Some(Self::Document),
            _ => None,
        }
    }
}

/// Classify a user chat turn. Slash commands win; otherwise a create-verb plus
/// a media/document noun (DE/EN/ES/FR/TR). Ordinary questions stay `None`.
pub fn classify(text: &str) -> Option<GenerationKind> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return None;
    }
    let lower = trimmed.to_lowercase();

    if looks_like_slash(&lower, "pic") || looks_like_slash(&lower, "img") {
        return Some(GenerationKind::Image);
    }
    if looks_like_slash(&lower, "tts") || looks_like_slash(&lower, "speak") {
        return Some(GenerationKind::Audio);
    }
    if looks_like_slash(&lower, "vid") {
        return Some(GenerationKind::Video);
    }
    if looks_like_slash(&lower, "doc") {
        return Some(GenerationKind::Document);
    }

    if is_image_request(&lower) {
        return Some(GenerationKind::Image);
    }
    if is_audio_request(&lower) {
        return Some(GenerationKind::Audio);
    }
    if is_video_request(&lower) {
        return Some(GenerationKind::Video);
    }
    if is_document_request(&lower) {
        return Some(GenerationKind::Document);
    }
    None
}

fn looks_like_slash(lower: &str, cmd: &str) -> bool {
    lower == format!("/{cmd}")
        || lower.starts_with(&format!("/{cmd} "))
        || lower.starts_with(&format!("/{cmd}\n"))
}

fn is_image_request(lower: &str) -> bool {
    contains_any(
        lower,
        &[
            "ein echtes bild",
            "ein bild",
            "echte bild",
            "generate an image",
            "create an image",
            "create a picture",
            "draw a",
            "draw me",
            "paint a",
            "make an image",
            "make a picture",
            "make a photo",
            "genera una imagen",
            "crée une image",
            "cree une image",
            "görsel oluştur",
            "resim oluştur",
            "zeichne eine",
            "zeichne ein",
            "zeichne mir",
            "male eine",
            "male ein",
            "male mir",
        ],
    ) || (has_create_verb(lower)
        && contains_any(
            lower,
            &[
                "bild",
                "image",
                "picture",
                "photo",
                "foto",
                "zeichnung",
                "imagen",
                "görsel",
                "resim",
            ],
        ))
}

fn is_audio_request(lower: &str) -> bool {
    lower.starts_with("sprich:")
        || lower.starts_with("sprich ")
        || contains_any(
            lower,
            &[
                "text to speech",
                "text-to-speech",
                "lies vor",
                "vorlesen",
                "sprachausgabe",
                "generate audio",
                "create audio",
                "make a sound",
                "erzeuge ein audio",
                "erstelle ein audio",
                "genera un audio",
                "crée un audio",
                "ses oluştur",
            ],
        )
        || (has_create_verb(lower) && has_token(lower, &["audio", "tts", "mp3", "wav", "sound"]))
}

fn is_video_request(lower: &str) -> bool {
    contains_any(
        lower,
        &[
            "generate a video",
            "create a video",
            "make a video",
            "erzeuge ein video",
            "erstelle ein video",
            "genera un vídeo",
            "genera un video",
            "crée une vidéo",
            "video oluştur",
        ],
    ) || (has_create_verb(lower) && has_token(lower, &["video", "film", "clip"]))
}

fn is_document_request(lower: &str) -> bool {
    if is_how_question(lower) {
        return false;
    }
    contains_any(
        lower,
        &[
            "generate a document",
            "create a document",
            "write a document",
            "write a report",
            "erstelle ein dokument",
            "erzeuge ein dokument",
            "schreib ein dokument",
            "genera un documento",
            "crée un document",
            "belge oluştur",
        ],
    ) || (has_create_verb(lower)
        && contains_any(
            lower,
            &[
                "ein dokument",
                "das dokument",
                "a document",
                "the document",
                "docx",
                "xlsx",
                "pptx",
                ".pdf",
                "eine präsentation",
                "a presentation",
                "eine tabelle",
                "a spreadsheet",
            ],
        ))
}

fn is_how_question(lower: &str) -> bool {
    lower.starts_with("how ")
        || lower.starts_with("wie ")
        || lower.starts_with("cómo ")
        || lower.starts_with("como ")
        || lower.starts_with("comment ")
        || lower.starts_with("nasıl ")
}

fn has_create_verb(lower: &str) -> bool {
    contains_any(
        lower,
        &[
            "generate", "create", "make", "draw", "paint", "write", "erzeuge", "erstelle",
            "zeichne", "male", "schreib", "zeige", "zeig", "genera", "crée", "cree", "oluştur",
            "yaz",
        ],
    )
}

fn contains_any(haystack: &str, needles: &[&str]) -> bool {
    needles.iter().any(|n| haystack.contains(n))
}

fn has_token(haystack: &str, tokens: &[&str]) -> bool {
    haystack
        .split(|c: char| !c.is_alphanumeric())
        .any(|word| tokens.contains(&word))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cat_picture_is_image() {
        assert_eq!(
            classify("ein echtes bild einer katze"),
            Some(GenerationKind::Image)
        );
        assert_eq!(
            classify("Generate an image of a cat"),
            Some(GenerationKind::Image)
        );
        assert_eq!(classify("/pic a red balloon"), Some(GenerationKind::Image));
        assert_eq!(classify("zeichne eine katze"), Some(GenerationKind::Image));
        assert_eq!(classify("male mir einen hund"), Some(GenerationKind::Image));
    }

    #[test]
    fn ordinary_questions_are_not_generation() {
        assert_eq!(classify("und wer ist jetzt mats?"), None);
        assert_eq!(classify("What is in the picture I uploaded?"), None);
        assert_eq!(classify("How do I write a report?"), None);
    }

    #[test]
    fn audio_and_document_intents() {
        assert_eq!(
            classify("Sprich: Guten Morgen"),
            Some(GenerationKind::Audio)
        );
        assert_eq!(
            classify("Erstelle ein Audio von diesem Text"),
            Some(GenerationKind::Audio)
        );
        assert_eq!(
            classify("Erstelle ein Dokument über Mats"),
            Some(GenerationKind::Document)
        );
        assert_eq!(
            classify("Create a video of waves"),
            Some(GenerationKind::Video)
        );
    }
}
