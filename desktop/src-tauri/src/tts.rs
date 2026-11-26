use chrono::Utc;
use futures_util::{SinkExt, StreamExt};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::Path;
use std::sync::{Arc, OnceLock};
use std::time::{Duration, SystemTime};
use tokio::process::Command;
use tokio::sync::Semaphore;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::http::HeaderValue;
use tokio_tungstenite::tungstenite::Message;
use uuid::Uuid;

const TRUSTED_CLIENT_TOKEN: &str = "6A5AA1D4EAFF4E9FB37E23D68491D6F4";
const CHROMIUM_FULL_VERSION: &str = "143.0.3650.75";
const CHROMIUM_MAJOR_VERSION: &str = "143";
const WIN_EPOCH_OFFSET: u64 = 11_644_473_600;

const SHA256_INITIAL: [u32; 8] = [
    0x6A09E667, 0xBB67AE85, 0x3C6EF372, 0xA54FF53A, 0x510E527F, 0x9B05688C, 0x1F83D9AB, 0x5BE0CD19,
];
const SHA256_STATE: [u32; 8] = [
    0xFD9FBEA2, 0x91243CE7, 0xBE499094, 0xCD2E1E0A, 0x6C006F49, 0x04BD35AB, 0xFEB0A7E4, 0x03EFFBD2,
];
const SHA256_SCHEDULE: [u32; 14] = [
    0xEB24BF9E, 0x1558691F, 0xA0C09EE2, 0x0CFA32BA, 0xA622EE8C, 0xD1DD9491, 0xC0DD25A0,
    0xF9E70AB6, 0xF0C31573, 0x7058CE4E, 0x7188E606, 0x00E5AB33, 0xD258B060, 0x72766564,
];
const SHA256_ROUND_CONSTANTS: [u32; 64] = [
    0x428A2F98, 0x71374491, 0xB5C0FBCF, 0xE9B5DBA5, 0x3956C25B, 0x59F111F1, 0x923F82A4, 0xAB1C5ED5,
    0xD807AA98, 0x12835B01, 0x243185BE, 0x550C7DC3, 0x72BE5D74, 0x80DEB1FE, 0x9BDC06A7, 0xC19BF174,
    0xE49B69C1, 0xEFBE4786, 0x0FC19DC6, 0x240CA1CC, 0x2DE92C6F, 0x4A7484AA, 0x5CB0A9DC, 0x76F988DA,
    0x983E5152, 0xA831C66D, 0xB00327C8, 0xBF597FC7, 0xC6E00BF3, 0xD5A79147, 0x06CA6351, 0x14292967,
    0x27B70A85, 0x2E1B2138, 0x4D2C6DFC, 0x53380D13, 0x650A7354, 0x766A0ABB, 0x81C2C92E, 0x92722C85,
    0xA2BFE8A1, 0xA81A664B, 0xC24B8B70, 0xC76C51A3, 0xD192E819, 0xD6990624, 0xF40E3585, 0x106AA070,
    0x19A4C116, 0x1E376C08, 0x2748774C, 0x34B0BCB5, 0x391C0CB3, 0x4ED8AA4A, 0x5B9CCA4F, 0x682E6FF3,
    0x748F82EE, 0x78A5636F, 0x84C87814, 0x8CC70208, 0x90BEFFFA, 0xA4506CEB, 0xBEF9A3F7, 0xC67178F2,
];
const SHA256_ADJUSTMENT: u32 = 0xC9DC06A7;
const SHA256_SIGMA0_ADJUSTMENT: u32 = 0x059C0B80;
const SHA256_SIGMA1_ADJUSTMENT: u32 = 0x000B92C0;

const CS64_COEFFICIENTS: [(u32, u32); 14] = [
    (0xE82E91B1, 0xB32700D0),
    (0x44DE24C6, 0x88DFF51D),
    (0x3BEB65BC, 0x29514038),
    (0xC89C5933, 0xB328CBDF),
    (0x8D1EC211, 0xD405B4D5),
    (0x314EF5D5, 0x26E55F3C),
    (0xE92491B1, 0x607801C6),
    (0xF0A5B9D5, 0x82B2C1FB),
    (0x06064CF3, 0xFA51A3DA),
    (0xDA79FA60, 0x207CC464),
    (0xBED20560, 0x3C3C74F3),
    (0x89F9C9F8, 0xD2D14B61),
    (0x35FA38C6, 0xBB04A2C1),
    (0xCB49E3C6, 0xC5626167),
];
const CS64_FACTOR: u32 = 0x82893933;
const CS64_ADJUSTMENT0: u32 = 0xF2F90B1A;
const CS64_ADJUSTMENT1: u32 = 0x0B1A0000;

#[inline]
fn sigma0(v: u32) -> u32 {
    v.rotate_right(7) ^ v.rotate_right(18) ^ (v >> 3)
}

#[inline]
fn sigma1(v: u32) -> u32 {
    v.rotate_right(17) ^ v.rotate_right(19) ^ (v >> 10)
}

#[inline]
fn big_sigma0(v: u32) -> u32 {
    v.rotate_right(2) ^ v.rotate_right(13) ^ v.rotate_right(22)
}

#[inline]
fn big_sigma1(v: u32) -> u32 {
    v.rotate_right(6) ^ v.rotate_right(11) ^ v.rotate_right(25)
}

fn sha256_rounds(mut s: [u32; 8], words: &[u32; 64], first_round: usize) -> [u32; 8] {
    for i in first_round..64 {
        let k = if first_round == 14 && i == 14 {
            SHA256_ADJUSTMENT
        } else {
            SHA256_ROUND_CONSTANTS[i]
        };
        let ch = s[6] ^ (s[4] & (s[5] ^ s[6]));
        let maj = (s[0] & s[1]) | (s[2] & (s[0] | s[1]));
        let t1 = s[7]
            .wrapping_add(big_sigma1(s[4]))
            .wrapping_add(ch)
            .wrapping_add(k)
            .wrapping_add(words[i]);
        let t2 = big_sigma0(s[0]).wrapping_add(maj);
        s = [
            t1.wrapping_add(t2),
            s[0],
            s[1],
            s[2],
            s[3].wrapping_add(t1),
            s[4],
            s[5],
            s[6],
        ];
    }
    s
}

fn expand_schedule(words: &mut [u32; 64], first_word: usize) {
    for i in first_word..64 {
        words[i] = sigma1(words[i - 2])
            .wrapping_add(words[i - 7])
            .wrapping_add(sigma0(words[i - 15]))
            .wrapping_add(words[i - 16]);
    }
}

fn sha256_init(prefix7: &[u8]) -> [u32; 8] {
    let mut words = [0u32; 64];
    words[14] = u32::from_be_bytes([0, prefix7[0], prefix7[1], prefix7[2]]);
    words[15] = u32::from_be_bytes([prefix7[3], prefix7[4], prefix7[5], prefix7[6]]);
    for i in 16..30 {
        let mut val = SHA256_SCHEDULE[i - 16];
        if i - 2 >= 14 {
            let s1 = sigma1(words[i - 2]);
            val = val.wrapping_add(if i == 16 { s1 ^ SHA256_SIGMA1_ADJUSTMENT } else { s1 });
        }
        if i - 7 >= 14 {
            val = val.wrapping_add(words[i - 7]);
        }
        if i - 15 >= 14 {
            let s0 = sigma0(words[i - 15]);
            val = val.wrapping_add(if i == 29 { s0 ^ SHA256_SIGMA0_ADJUSTMENT } else { s0 });
        }
        words[i] = val;
    }
    words[30] = sigma1(words[28])
        .wrapping_add(words[23])
        .wrapping_add(sigma0(words[15]))
        .wrapping_add(words[14])
        .wrapping_add(SHA256_ADJUSTMENT)
        .wrapping_sub(SHA256_ROUND_CONSTANTS[14]);
    expand_schedule(&mut words, 31);
    let working = sha256_rounds(SHA256_STATE, &words, 14);
    let mut out = [0u32; 8];
    for i in 0..8 {
        out[i] = SHA256_INITIAL[i].wrapping_add(working[i]);
    }
    out
}

fn custom_sha256_digest(data: &[u8]) -> [u8; 32] {
    let length = (57 + data.len()) as u64;
    let mut padded = Vec::with_capacity(data.len() + 72);
    padded.extend_from_slice(&data[7..]);
    padded.push(0x80);
    let pad_zeros = (55usize.wrapping_sub(length as usize)) % 64;
    padded.resize(padded.len() + pad_zeros, 0);
    padded.extend_from_slice(&(length * 8).to_be_bytes());

    let mut state = sha256_init(&data[..7]);
    for chunk in padded.chunks_exact(64) {
        let mut words = [0u32; 64];
        for (i, w) in chunk.chunks_exact(4).enumerate() {
            words[i] = u32::from_be_bytes([w[0], w[1], w[2], w[3]]);
        }
        expand_schedule(&mut words, 16);
        let working = sha256_rounds(state, &words, 0);
        for i in 0..8 {
            state[i] = state[i].wrapping_add(working[i]);
        }
    }
    let mut out = [0u8; 32];
    for (i, w) in state.iter().enumerate() {
        out[i * 4..(i + 1) * 4].copy_from_slice(&w.to_be_bytes());
    }
    out
}

fn cs64_digest(data: &[u8], digest: &[u8; 32]) -> (u32, u32) {
    let key_a = u32::from_le_bytes([digest[0], digest[1], digest[2], digest[3]]) | 1;
    let key_b = u32::from_le_bytes([digest[4], digest[5], digest[6], digest[7]]) | 1;
    let factors = [key_a.wrapping_mul(CS64_FACTOR), key_b.wrapping_mul(CS64_FACTOR)];

    let mut terms: Vec<u32> = CS64_COEFFICIENTS
        .iter()
        .enumerate()
        .map(|(i, (c, _))| c.wrapping_mul(factors[i % 2]))
        .collect();
    let mut offsets: Vec<u32> = CS64_COEFFICIENTS.iter().map(|(_, o)| *o).collect();

    let mut prefixed = Vec::with_capacity(1 + data.len() + 8);
    prefixed.push(0);
    prefixed.extend_from_slice(data);
    while prefixed.len() % 4 != 0 {
        prefixed.push(0);
    }
    let mut words: Vec<u32> = prefixed
        .chunks_exact(4)
        .map(|c| u32::from_le_bytes([c[0], c[1], c[2], c[3]]))
        .collect();
    if words.len() % 2 == 1 {
        words.push(0);
    }

    for (i, word) in words.into_iter().enumerate() {
        let mut term = word.wrapping_mul(if i % 2 == 0 { key_a } else { key_b });
        let mut offset = word
            .rotate_left(16)
            .wrapping_mul(if i % 2 == 0 { 0xEF0569FB } else { 0xC31713DB });
        if i == 0 {
            term = term.wrapping_add(CS64_ADJUSTMENT0.wrapping_mul(factors[0]));
            offset = offset.wrapping_add(CS64_ADJUSTMENT1);
        }
        terms.push(term);
        offsets.push(offset);
    }

    let mut swap_acc: u32 = 0;
    let mut swap_last: u32 = 0;
    let mut state: u32 = 0;
    let mut acc: u32 = 0;

    for i in (0..terms.len()).step_by(2) {
        let x = terms[i];
        let y = terms[i + 1];
        let t = offsets[i].wrapping_add(x).rotate_left(16);
        let q = t.wrapping_mul(0x76F50F34);
        swap_acc = swap_acc.wrapping_add(q.rotate_left(16).wrapping_mul(0xC3EFEA97).wrapping_add(q));

        let t2 = offsets[i + 1].wrapping_add(y).rotate_left(16);
        let q2 = t2.wrapping_mul(0x1881CE3C);
        swap_last = q2.rotate_left(16).wrapping_mul(0x35BD1EC9).wrapping_add(q2);
        swap_acc = swap_acc.wrapping_add(swap_last);

        state = state.wrapping_mul(key_a).wrapping_add(x);
        state = state.rotate_left(16).wrapping_mul(0xCF98B111);
        state = state.rotate_left(16).wrapping_mul(0x87085B9F);
        state = state.rotate_left(16).wrapping_mul(0x12CEB96D);
        state = state.rotate_left(16).wrapping_mul(0x257E1D83);
        acc = acc.wrapping_add(state);

        let mut u = state.wrapping_mul(key_b).wrapping_add(y);
        u = u.rotate_left(16).wrapping_mul(0xA27416F5);
        u = u.rotate_left(16).wrapping_mul(0xD38396FF);
        state = u.rotate_left(16).wrapping_mul(0x7C932B89).rotate_left(16).wrapping_mul(0xBFA49F69);
        acc = acc.wrapping_add(state);
    }

    (swap_last ^ state, swap_acc ^ acc)
}

fn generate_sec_ms_gec_v2() -> String {
    let unix_secs = Utc::now().timestamp().max(0) as u64;
    let mut ticks = unix_secs + WIN_EPOCH_OFFSET;
    ticks -= ticks % 300;
    let ticks_100ns = (ticks as u128) * 10_000_000;
    let input = format!("{ticks_100ns}{TRUSTED_CLIENT_TOKEN}");
    let digest = custom_sha256_digest(input.as_bytes());
    let (w0, w1) = cs64_digest(input.as_bytes(), &digest);
    let mut cs_bytes = [0u8; 8];
    cs_bytes[..4].copy_from_slice(&w0.to_le_bytes());
    cs_bytes[4..].copy_from_slice(&w1.to_le_bytes());
    let cs64: String = cs_bytes.iter().map(|b| format!("{b:02X}")).collect();
    let sha_hex: String = digest.iter().map(|b| format!("{b:02X}")).collect();
    format!("{cs64}-{sha_hex}")
}

fn escape_xml(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

fn js_timestamp() -> String {
    Utc::now()
        .format("%a %b %d %Y %H:%M:%S GMT+0000 (Coordinated Universal Time)")
        .to_string()
}

fn sanitize_rate(rate: Option<&str>) -> String {
    let r = rate.unwrap_or("+0%").trim();
    if r.is_empty() {
        return "+0%".to_string();
    }
    if (r.starts_with('+') || r.starts_with('-')) && r.ends_with('%') {
        if r[1..r.len() - 1].parse::<i32>().is_ok() {
            return r.to_string();
        }
    }
    "+0%".to_string()
}

fn sanitize_voice(voice: Option<&str>) -> String {
    let v = voice.unwrap_or("vi-VN-HoaiMyNeural").trim();
    if v.is_empty()
        || !v
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    {
        return "vi-VN-HoaiMyNeural".to_string();
    }
    v.to_string()
}

fn cache_key(voice: &str, rate: &str, text: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(format!("{voice}|{rate}|{text}").as_bytes());
    let digest = hasher.finalize();
    digest.iter().map(|b| format!("{b:02x}")).collect()
}

async fn synthesize_edge_tts_once(
    clean_text: &str,
    voice_name: &str,
    rate_str: &str,
) -> Result<Vec<u8>, String> {
    let conn_id = Uuid::new_v4().simple().to_string();
    let sec_ms_gec = generate_sec_ms_gec_v2();
    let ws_url = format!(
        "wss://speech.platform.bing.com/consumer/speech/synthesize/readaloud/edge/v1?TrustedClientToken={TRUSTED_CLIENT_TOKEN}&Sec-MS-GEC={sec_ms_gec}&Sec-MS-GEC-Version=2-{CHROMIUM_FULL_VERSION}&ConnectionId={conn_id}"
    );

    let mut req = ws_url
        .into_client_request()
        .map_err(|e| format!("Invalid WS URL: {e}"))?;
    let headers = req.headers_mut();
    headers.insert("Pragma", HeaderValue::from_static("no-cache"));
    headers.insert("Cache-Control", HeaderValue::from_static("no-cache"));
    headers.insert(
        "Origin",
        HeaderValue::from_static("chrome-extension://jdiccldimpdaibmpdkjnbmckianbfold"),
    );
    let ua = format!(
        "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/{CHROMIUM_MAJOR_VERSION}.0.0.0 Safari/537.36 Edg/{CHROMIUM_MAJOR_VERSION}.0.0.0"
    );
    headers.insert(
        "User-Agent",
        HeaderValue::from_str(&ua).map_err(|e| e.to_string())?,
    );
    headers.insert(
        "Accept-Encoding",
        HeaderValue::from_static("gzip, deflate, br, zstd"),
    );
    headers.insert("Accept-Language", HeaderValue::from_static("en-US,en;q=0.9"));

    let (mut ws, _) = tokio_tungstenite::connect_async(req)
        .await
        .map_err(|e| format!("Edge TTS connect failed: {e}"))?;

    let ts = js_timestamp();
    let config_msg = format!(
        "X-Timestamp:{ts}\r\nContent-Type:application/json; charset=utf-8\r\nPath:speech.config\r\n\r\n{{\"context\":{{\"synthesis\":{{\"audio\":{{\"metadataoptions\":{{\"sentenceBoundaryEnabled\":\"false\",\"wordBoundaryEnabled\":\"false\"}},\"outputFormat\":\"audio-24khz-48kbitrate-mono-mp3\"}}}}}}}}"
    );
    ws.send(Message::Text(config_msg.into()))
        .await
        .map_err(|e| format!("Edge TTS config send failed: {e}"))?;

    let req_id = Uuid::new_v4().simple().to_string();
    let escaped = escape_xml(clean_text);
    let ssml = format!(
        "<speak version='1.0' xmlns='http://www.w3.org/2001/10/synthesis' xml:lang='en-US'><voice name='{voice_name}'><prosody pitch='+0Hz' rate='{rate_str}' volume='+0%'>{escaped}</prosody></voice></speak>"
    );
    let ssml_msg = format!(
        "X-RequestId:{req_id}\r\nContent-Type:application/ssml+xml\r\nX-Timestamp:{ts}\r\nPath:ssml\r\n\r\n{ssml}"
    );
    ws.send(Message::Text(ssml_msg.into()))
        .await
        .map_err(|e| format!("Edge TTS SSML send failed: {e}"))?;

    let mut audio_bytes: Vec<u8> = Vec::new();
    while let Some(frame) = ws.next().await {
        match frame {
            Ok(Message::Binary(data)) => {
                if data.len() >= 2 {
                    let header_len = u16::from_be_bytes([data[0], data[1]]) as usize;
                    let audio_start = 2 + header_len;
                    if audio_start < data.len() {
                        audio_bytes.extend_from_slice(&data[audio_start..]);
                    }
                }
            }
            Ok(Message::Text(txt)) => {
                if txt.contains("Path:turn.end") {
                    break;
                }
            }
            Ok(Message::Close(_)) => break,
            Err(e) => return Err(format!("Edge TTS stream error: {e}")),
            _ => {}
        }
    }
    let _ = ws.close(None).await;

    if audio_bytes.is_empty() {
        return Err("Edge TTS returned empty audio".to_string());
    }
    Ok(audio_bytes)
}

static EDGE_TTS_SEM: OnceLock<Arc<Semaphore>> = OnceLock::new();

fn edge_tts_sem() -> Arc<Semaphore> {
    EDGE_TTS_SEM.get_or_init(|| Arc::new(Semaphore::new(2))).clone()
}

pub async fn synthesize_edge_tts(
    data_dir: &Path,
    text: &str,
    voice: Option<&str>,
    rate: Option<&str>,
) -> Result<Vec<u8>, String> {
    let clean_text = text.trim();
    if clean_text.is_empty() {
        return Err("Empty text".to_string());
    }

    let voice_name = sanitize_voice(voice);
    let rate_str = sanitize_rate(rate);

    let cache_dir = data_dir.join("tts_cache");
    let cache_path = cache_dir.join(format!("{}.mp3", cache_key(&voice_name, &rate_str, clean_text)));
    if let Ok(cached) = fs::read(&cache_path) {
        if !cached.is_empty() {
            return Ok(cached);
        }
    }

    let sem = edge_tts_sem();
    let Ok(_permit) = sem.acquire().await else {
        return Err("TTS semaphore closed".to_string());
    };

    // Re-check cache after acquiring semaphore in case another task just populated it
    if let Ok(cached) = fs::read(&cache_path) {
        if !cached.is_empty() {
            return Ok(cached);
        }
    }

    let mut last_err = String::new();
    for attempt in 0..2 {
        match tokio::time::timeout(
            Duration::from_secs(6),
            synthesize_edge_tts_once(clean_text, &voice_name, &rate_str),
        )
        .await
        {
            Ok(Ok(audio_bytes)) => {
                let _ = fs::create_dir_all(&cache_dir);
                let _ = fs::write(&cache_path, &audio_bytes);
                return Ok(audio_bytes);
            }
            Ok(Err(e)) => last_err = e,
            Err(_) => last_err = "Edge TTS request timed out".to_string(),
        }
        if attempt == 0 {
            tokio::time::sleep(Duration::from_millis(120)).await;
        }
    }
    Err(last_err)
}

#[cfg(target_os = "macos")]
async fn synthesize_macos_say_mp3(
    bin_dir: &Path,
    text: &str,
    voice_name: &str,
) -> Result<Vec<u8>, String> {
    let ffmpeg = crate::tools::find_binary("ffmpeg", bin_dir)
        .ok_or_else(|| "ffmpeg not found for macOS say fallback".to_string())?;
    let mac_voice = if voice_name.starts_with("vi-") {
        "Linh"
    } else {
        "Samantha"
    };
    let id = Uuid::new_v4().simple().to_string();
    let tmp_dir = std::env::temp_dir();
    let aiff_path = tmp_dir.join(format!("ytcs_say_{id}.aiff"));
    let mp3_path = tmp_dir.join(format!("ytcs_say_{id}.mp3"));

    let say_ok = Command::new("say")
        .args(["-v", mac_voice, "-o"])
        .arg(&aiff_path)
        .arg("--")
        .arg(text)
        .output()
        .await
        .map(|o| o.status.success())
        .unwrap_or(false);

    if !say_ok || !aiff_path.is_file() {
        let _ = fs::remove_file(&aiff_path);
        return Err("macOS say command failed".to_string());
    }

    let ff_ok = Command::new(&ffmpeg)
        .args(["-y", "-i"])
        .arg(&aiff_path)
        .args(["-ar", "24000", "-ac", "1", "-b:a", "48k"])
        .arg(&mp3_path)
        .env("PATH", crate::tools::build_env_path(bin_dir))
        .output()
        .await
        .map(|o| o.status.success())
        .unwrap_or(false);

    let _ = fs::remove_file(&aiff_path);
    let bytes = if ff_ok { fs::read(&mp3_path).ok() } else { None };
    let _ = fs::remove_file(&mp3_path);

    bytes.filter(|b| !b.is_empty())
        .ok_or_else(|| "Failed to encode macOS say fallback MP3".to_string())
}

#[cfg(not(target_os = "macos"))]
async fn synthesize_macos_say_mp3(
    _bin_dir: &Path,
    _text: &str,
    _voice_name: &str,
) -> Result<Vec<u8>, String> {
    Err("Offline say fallback only available on macOS".to_string())
}

pub async fn synthesize_edge_tts_with_fallback(
    data_dir: &Path,
    bin_dir: &Path,
    text: &str,
    voice: Option<&str>,
    rate: Option<&str>,
) -> Result<Vec<u8>, String> {
    match synthesize_edge_tts(data_dir, text, voice, rate).await {
        Ok(bytes) => Ok(bytes),
        Err(edge_err) => {
            let voice_name = sanitize_voice(voice);
            if let Ok(fallback) = synthesize_macos_say_mp3(bin_dir, text.trim(), &voice_name).await {
                Ok(fallback)
            } else {
                Err(edge_err)
            }
        }
    }
}

pub async fn prepare_edge_tts_batch(
    data_dir: &Path,
    texts: Vec<String>,
    voice: Option<String>,
    rate: Option<String>,
) -> usize {
    let sem = Arc::new(Semaphore::new(2));
    let mut handles = Vec::with_capacity(texts.len());
    for text in texts {
        let trimmed = text.trim().to_string();
        if trimmed.is_empty() {
            continue;
        }
        let permit = sem.clone();
        let dir = data_dir.to_path_buf();
        let v = voice.clone();
        let r = rate.clone();
        handles.push(tokio::spawn(async move {
            let Ok(_guard) = permit.acquire().await else {
                return false;
            };
            synthesize_edge_tts(&dir, &trimmed, v.as_deref(), r.as_deref())
                .await
                .is_ok()
        }));
    }
    let mut ready = 0usize;
    for h in handles {
        if matches!(h.await, Ok(true)) {
            ready += 1;
        }
    }
    ready
}

pub fn cleanup_tts_cache(data_dir: &Path, max_bytes: u64, max_age_secs: u64) {
    let cache_dir = data_dir.join("tts_cache");
    let Ok(entries) = fs::read_dir(&cache_dir) else {
        return;
    };
    let now = SystemTime::now();
    let max_age = Duration::from_secs(max_age_secs);
    let mut files: Vec<(std::path::PathBuf, u64, SystemTime)> = Vec::new();
    let mut total_bytes: u64 = 0;

    for entry in entries.flatten() {
        let path = entry.path();
        let Ok(meta) = entry.metadata() else { continue };
        if !meta.is_file() {
            continue;
        }
        let modified = meta.modified().unwrap_or(SystemTime::UNIX_EPOCH);
        if let Ok(age) = now.duration_since(modified) {
            if age > max_age {
                let _ = fs::remove_file(&path);
                continue;
            }
        }
        let len = meta.len();
        total_bytes = total_bytes.saturating_add(len);
        files.push((path, len, modified));
    }

    if total_bytes > max_bytes {
        files.sort_by_key(|(_, _, modified)| *modified);
        for (path, len, _) in files {
            if total_bytes <= max_bytes {
                break;
            }
            if fs::remove_file(&path).is_ok() {
                total_bytes = total_bytes.saturating_sub(len);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_edge_tts_live() {
        let tmp = std::env::temp_dir().join("yt_cloud_saver_tts_test");
        let bytes = synthesize_edge_tts(
            &tmp,
            "Xin chào, đây là giọng đọc Hoài Mỹ.",
            Some("vi-VN-HoaiMyNeural"),
            Some("+15%"),
        )
        .await
        .expect("Edge TTS should synthesize MP3");
        println!("Synthesized {} bytes", bytes.len());
        assert!(bytes.len() > 1000);
        let _ = fs::remove_dir_all(&tmp);
    }
}
