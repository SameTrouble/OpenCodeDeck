use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum QrKind {
    Ascii,
    Url,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct WechatQrEvent {
    pub kind: QrKind,
    pub data: String,
}

pub struct StdoutParser {
    ascii_buffer: Vec<String>,
    collecting_ascii: bool,
}

impl StdoutParser {
    pub fn new() -> Self {
        Self { ascii_buffer: Vec::new(), collecting_ascii: false }
    }

    pub fn feed_line(&mut self, line: &str) -> Option<WechatQrEvent> {
        let trimmed = line.trim_end();
        if let Some(url) = extract_wechat_url(trimmed) {
            self.collecting_ascii = false;
            self.ascii_buffer.clear();
            return Some(WechatQrEvent { kind: QrKind::Url, data: url.to_string() });
        }

        if is_ascii_qr_line(trimmed) {
            if !self.collecting_ascii {
                self.collecting_ascii = true;
                self.ascii_buffer.clear();
            }
            self.ascii_buffer.push(trimmed.to_string());
            if self.ascii_buffer.len() >= 21 {
                let data = self.ascii_buffer.join("\n");
                self.collecting_ascii = false;
                self.ascii_buffer.clear();
                return Some(WechatQrEvent { kind: QrKind::Ascii, data });
            }
            return None;
        }

        if self.collecting_ascii && self.ascii_buffer.len() >= 10 {
            let data = self.ascii_buffer.join("\n");
            self.collecting_ascii = false;
            self.ascii_buffer.clear();
            return Some(WechatQrEvent { kind: QrKind::Ascii, data });
        }

        None
    }
}

fn is_ascii_qr_line(line: &str) -> bool {
    let block_chars = ['█', '▀', '▄', '▌', '▐', '■', '□'];
    let block_count = line.chars().filter(|c| block_chars.contains(c)).count();
    block_count >= 10
}

fn extract_wechat_url(line: &str) -> Option<&str> {
    let markers = ["login.weixin.qq.com", "wx.qq.com", "login.wechat.com"];
    for marker in &markers {
        if let Some(pos) = line.find(marker) {
            let start = line[..pos].rfind("https://")
                .or_else(|| line[..pos].rfind("http://"))
                .unwrap_or(0);
            let rest = &line[start..];
            let end = rest.find(char::is_whitespace).unwrap_or(rest.len());
            return Some(&rest[..end]);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_wechat_url() {
        let mut p = StdoutParser::new();
        let ev = p.feed_line("请扫码登录: https://login.weixin.qq.com/qrcode/abc123 ");
        assert_eq!(ev, Some(WechatQrEvent {
            kind: QrKind::Url,
            data: "https://login.weixin.qq.com/qrcode/abc123".to_string(),
        }));
    }

    #[test]
    fn detects_ascii_qr_block() {
        let mut p = StdoutParser::new();
        let mut last_ev = None;
        for _ in 0..21 {
            let line: String = std::iter::repeat('█').take(20).collect();
            last_ev = p.feed_line(&line);
        }
        assert!(matches!(last_ev, Some(WechatQrEvent { kind: QrKind::Ascii, .. })));
    }

    #[test]
    fn ignores_normal_lines() {
        let mut p = StdoutParser::new();
        assert_eq!(p.feed_line("[INFO] server starting on port 4096"), None);
    }

    #[test]
    fn url_detection_resets_ascii_collection() {
        let mut p = StdoutParser::new();
        let block_line: String = std::iter::repeat('█').take(20).collect();
        let _ = p.feed_line(&block_line);
        let ev = p.feed_line("https://login.weixin.qq.com/qrcode/xyz");
        assert_eq!(ev.unwrap().kind, QrKind::Url);
    }
}
