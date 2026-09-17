//! Generated test material. Everything is built in-process and deterministic
//! for a run id, so a fact planted in a 3 MB document cannot come from an
//! earlier run or from the model's training data.

use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};

/// A planted fact: the sentence that goes into a document, the question a
/// person would ask, and the token the answer must contain (normalised).
#[derive(Debug, Clone)]
pub struct Fact {
    pub sentence: String,
    pub question: String,
    pub expect: String,
}

/// Facts unique to `run`: codes carry the run id so they never collide.
pub fn facts(run: &str) -> Vec<Fact> {
    let run = run.to_uppercase();
    vec![
        Fact {
            sentence: format!(
                "The Nordlicht warehouse access code for the Hamburg site is ZETA-{run}."
            ),
            question:
                "What is the Nordlicht warehouse access code for the Hamburg site? Answer from my project files."
                    .into(),
            expect: format!("ZETA{run}"),
        },
        Fact {
            sentence: format!(
                "The Aurora rollout is led by project manager Malin Sørensen; her internal badge number is BADGE-{run}-77."
            ),
            question: "What is the internal badge number of the Aurora rollout project manager? Use my project files."
                .into(),
            expect: format!("BADGE{run}77"),
        },
        Fact {
            sentence: format!(
                "The Q3 2026 revenue of the East region was 36,990 EUR, booked under cost centre CC-{run}-EAST."
            ),
            question: "Under which cost centre was the Q3 2026 East region revenue booked? Use my project files."
                .into(),
            expect: format!("CC{run}EAST"),
        },
        Fact {
            sentence: format!(
                "The backup generator in the Lübeck plant is model GEN-{run}-K9 and is serviced every 400 hours."
            ),
            question: "Which model is the backup generator in the Lübeck plant? Use my project files.".into(),
            expect: format!("GEN{run}K9"),
        },
        Fact {
            sentence: format!(
                "The customer satisfaction survey of August 2026 returned a Net Promoter Score of 61 and the survey reference is NPS-{run}-AUG."
            ),
            question: "What is the survey reference of the August 2026 customer satisfaction survey? Use my project files.".into(),
            expect: format!("NPS{run}AUG"),
        },
    ]
}

/// Deterministic filler prose (LCG over a word list) so a big file compresses
/// like text and embeds like text — not like a repeated pattern.
struct Words {
    state: u64,
}

impl Words {
    const LIST: &'static [&'static str] = &[
        "region",
        "quarter",
        "revenue",
        "units",
        "warehouse",
        "shipment",
        "forecast",
        "meeting",
        "customer",
        "invoice",
        "delivery",
        "supplier",
        "contract",
        "review",
        "budget",
        "target",
        "growth",
        "margin",
        "pipeline",
        "campaign",
        "product",
        "launch",
        "feedback",
        "support",
        "ticket",
        "incident",
        "maintenance",
        "schedule",
        "audit",
        "compliance",
        "training",
        "onboarding",
        "roadmap",
        "milestone",
        "release",
        "backlog",
        "sprint",
        "retrospective",
        "stakeholder",
        "approval",
        "policy",
        "procedure",
        "checklist",
        "inventory",
        "logistics",
        "Hamburg",
        "Bremen",
        "Kiel",
        "Lübeck",
        "Rostock",
        "Flensburg",
        "Oldenburg",
        "Osnabrück",
    ];

    fn new(seed: u64) -> Self {
        Self {
            state: seed
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407),
        }
    }

    fn next(&mut self) -> &'static str {
        self.state = self
            .state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        Self::LIST[((self.state >> 33) as usize) % Self::LIST.len()]
    }

    fn sentence(&mut self, words: usize) -> String {
        let mut s = String::new();
        for i in 0..words {
            let w = self.next();
            if i == 0 {
                let mut c = w.chars();
                if let Some(f) = c.next() {
                    s.push_str(&f.to_uppercase().to_string());
                    s.push_str(c.as_str());
                }
            } else {
                s.push(' ');
                s.push_str(w);
            }
        }
        s.push('.');
        s
    }
}

/// A Markdown document of roughly `bytes` bytes with `facts` planted at even
/// intervals, each in its own short section so a chunker keeps it whole.
pub fn big_markdown(title: &str, bytes: usize, facts: &[Fact], seed: u64) -> String {
    let mut w = Words::new(seed);
    let mut out = format!("# {title}\n\nInternal working notes, generated corpus for the desktop use-case runner.\n\n");
    let mut section = 1usize;
    let mut next_fact = 0usize;
    let fact_every = if facts.is_empty() {
        usize::MAX
    } else {
        (bytes / (facts.len() + 1)).max(1)
    };
    let mut since_fact = 0usize;
    while out.len() < bytes {
        let before = out.len();
        out.push_str(&format!(
            "\n## Section {section}: {}\n\n",
            w.sentence(3).trim_end_matches('.')
        ));
        for _ in 0..6 {
            out.push_str(&w.sentence(14));
            out.push(' ');
            out.push_str(&w.sentence(11));
            out.push_str("\n\n");
        }
        since_fact += out.len() - before;
        if next_fact < facts.len() && since_fact >= fact_every {
            out.push_str(&format!(
                "\n## Key fact {}\n\n{}\n\n",
                next_fact + 1,
                facts[next_fact].sentence
            ));
            next_fact += 1;
            since_fact = 0;
        }
        section += 1;
    }
    // Any fact not yet planted goes at the end so every fact is present.
    while next_fact < facts.len() {
        out.push_str(&format!(
            "\n## Key fact {}\n\n{}\n\n",
            next_fact + 1,
            facts[next_fact].sentence
        ));
        next_fact += 1;
    }
    out
}

fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// A minimal but valid `.docx` (one paragraph per line). Built with the
/// `zip` crate the core already depends on.
pub fn minimal_docx(lines: &[String]) -> Result<Vec<u8>, String> {
    let mut buf = std::io::Cursor::new(Vec::new());
    {
        let mut zip = zip::ZipWriter::new(&mut buf);
        let opts = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated);
        let mut put = |name: &str, body: &str| -> Result<(), String> {
            zip.start_file(name, opts).map_err(|e| e.to_string())?;
            zip.write_all(body.as_bytes()).map_err(|e| e.to_string())
        };
        put(
            "[Content_Types].xml",
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
<Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
<Default Extension="xml" ContentType="application/xml"/>
<Override PartName="/word/document.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/>
</Types>"#,
        )?;
        put(
            "_rels/.rels",
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
<Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="word/document.xml"/>
</Relationships>"#,
        )?;
        let mut body = String::from(
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:body>"#,
        );
        for line in lines {
            body.push_str(&format!(
                "<w:p><w:r><w:t xml:space=\"preserve\">{}</w:t></w:r></w:p>",
                xml_escape(line)
            ));
        }
        body.push_str("<w:sectPr/></w:body></w:document>");
        put("word/document.xml", &body)?;
        zip.finish().map_err(|e| e.to_string())?;
    }
    Ok(buf.into_inner())
}

/// A minimal PDF 1.4 with one text page (Helvetica, WinAnsi). Enough for
/// every extractor; not pretty.
pub fn minimal_pdf(lines: &[String]) -> Vec<u8> {
    let mut content = String::from("BT\n/F1 11 Tf\n50 780 Td\n13 TL\n");
    for line in lines {
        let ascii: String = line
            .chars()
            .map(|c| {
                if c.is_ascii() && c != '(' && c != ')' && c != '\\' {
                    c
                } else {
                    ' '
                }
            })
            .collect();
        content.push_str(&format!("({ascii}) Tj T*\n"));
    }
    content.push_str("ET\n");
    let objects = [
        "<< /Type /Catalog /Pages 2 0 R >>".to_string(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_string(),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 595 842] /Contents 4 0 R /Resources << /Font << /F1 5 0 R >> >> >>".to_string(),
        format!("<< /Length {} >>\nstream\n{}endstream", content.len(), content),
        "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding >>".to_string(),
    ];
    let mut pdf = String::from("%PDF-1.4\n");
    let mut offsets = Vec::new();
    for (i, obj) in objects.iter().enumerate() {
        offsets.push(pdf.len());
        pdf.push_str(&format!("{} 0 obj\n{}\nendobj\n", i + 1, obj));
    }
    let xref = pdf.len();
    pdf.push_str(&format!(
        "xref\n0 {}\n0000000000 65535 f \n",
        objects.len() + 1
    ));
    for off in offsets {
        pdf.push_str(&format!("{off:010} 00000 n \n"));
    }
    pdf.push_str(&format!(
        "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{}\n%%EOF\n",
        objects.len() + 1,
        xref
    ));
    pdf.into_bytes()
}

/// Decode any container `ffmpeg` understands into 16 kHz mono `s16le` PCM,
/// the format the dictation session expects. Direct process, no shell.
pub fn decode_to_pcm16k(bytes: &[u8]) -> Result<Vec<u8>, String> {
    let mut child = Command::new("ffmpeg")
        .args([
            "-loglevel",
            "error",
            "-i",
            "pipe:0",
            "-f",
            "s16le",
            "-acodec",
            "pcm_s16le",
            "-ac",
            "1",
            "-ar",
            "16000",
            "pipe:1",
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("ffmpeg not available: {e}"))?;
    let mut stdin = child.stdin.take().ok_or("ffmpeg stdin")?;
    let input = bytes.to_vec();
    let writer = std::thread::spawn(move || stdin.write_all(&input));
    let out = child.wait_with_output().map_err(|e| e.to_string())?;
    let _ = writer.join();
    if !out.status.success() {
        return Err(format!(
            "ffmpeg failed: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        ));
    }
    Ok(out.stdout)
}

/// Wrap 16 kHz mono `s16le` PCM into a WAV container (44-byte RIFF header),
/// so the same bytes can go through the one-shot upload route.
pub fn wav_from_pcm16k(pcm: &[u8]) -> Vec<u8> {
    let sample_rate: u32 = 16_000;
    let channels: u16 = 1;
    let bits: u16 = 16;
    let byte_rate = sample_rate * u32::from(channels) * u32::from(bits) / 8;
    let block_align = channels * bits / 8;
    let data_len = pcm.len() as u32;
    let mut out = Vec::with_capacity(44 + pcm.len());
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&(36 + data_len).to_le_bytes());
    out.extend_from_slice(b"WAVE");
    out.extend_from_slice(b"fmt ");
    out.extend_from_slice(&16u32.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&channels.to_le_bytes());
    out.extend_from_slice(&sample_rate.to_le_bytes());
    out.extend_from_slice(&byte_rate.to_le_bytes());
    out.extend_from_slice(&block_align.to_le_bytes());
    out.extend_from_slice(&bits.to_le_bytes());
    out.extend_from_slice(b"data");
    out.extend_from_slice(&data_len.to_le_bytes());
    out.extend_from_slice(pcm);
    out
}

/// RMS of 16-bit little-endian PCM in 0..1.
pub fn rms_pcm16(pcm: &[u8]) -> f64 {
    if pcm.len() < 2 {
        return 0.0;
    }
    let mut sum = 0f64;
    let n = pcm.len() / 2;
    for i in 0..n {
        let s = i16::from_le_bytes([pcm[2 * i], pcm[2 * i + 1]]) as f64 / 32768.0;
        sum += s * s;
    }
    (sum / n as f64).sqrt()
}

fn words_of(s: &str) -> Vec<String> {
    s.split(|c: char| !c.is_alphanumeric())
        .filter(|w| !w.is_empty())
        .map(|w| w.to_lowercase())
        .collect()
}

/// Word-level similarity `1 - WER` (LCS-based edit distance over words),
/// clamped to 0..1. Good enough to tell "recognised" from "garbage".
pub fn similarity(reference: &str, hypothesis: &str) -> f64 {
    let r = words_of(reference);
    let h = words_of(hypothesis);
    if r.is_empty() {
        return if h.is_empty() { 1.0 } else { 0.0 };
    }
    let mut prev = vec![0usize; h.len() + 1];
    let mut cur = vec![0usize; h.len() + 1];
    for rw in &r {
        for (j, hw) in h.iter().enumerate() {
            cur[j + 1] = if rw == hw {
                prev[j] + 1
            } else {
                prev[j + 1].max(cur[j])
            };
        }
        std::mem::swap(&mut prev, &mut cur);
        cur[0] = 0;
    }
    let lcs = prev[h.len()];
    let errors = r.len().max(h.len()) - lcs;
    (1.0 - errors as f64 / r.len() as f64).clamp(0.0, 1.0)
}

pub fn write_file(dir: &Path, name: &str, bytes: &[u8]) -> Result<std::path::PathBuf, String> {
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    let path = dir.join(name);
    std::fs::write(&path, bytes).map_err(|e| e.to_string())?;
    Ok(path)
}

/// The paragraph read aloud for the dictation case: everyday note-taking
/// language, ~100 words, no numbers Whisper could misplace.
pub const DICTATION_TEXT: &str = "Good morning. This is a short note for the Aurora project. \
Yesterday we met the team in Hamburg and agreed on three things. First, the warehouse in Bremen \
will ship the new lamps at the end of the month. Second, Malin will send the updated forecast to \
the sales leads before Friday. Third, we want a short review meeting every two weeks until the \
launch. Please remind me to book the room and to invite the supplier from Kiel. That is all for \
now, thank you.";
