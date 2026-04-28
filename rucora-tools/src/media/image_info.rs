//! 图片信息工具
//!
//! 读取图片文件的元数据信息，包括格式、尺寸、文件大小等

use async_trait::async_trait;
use rucora_core::{
    error::ToolError,
    tool::{Tool, ToolCategory},
};
use serde_json::{Value, json};
use std::path::Path;

/// 最大文件大小（5MB）
const MAX_IMAGE_BYTES: u64 = 5 * 1024 * 1024;

/// 图片信息工具
///
/// 读取图片文件的元数据，支持 PNG、JPEG、GIF、WebP、BMP 格式
pub struct ImageInfoTool;

impl ImageInfoTool {
    /// 创建新的图片信息工具
    pub fn new() -> Self {
        Self
    }

    /// 从文件头检测图片格式
    fn detect_format(bytes: &[u8]) -> &'static str {
        if bytes.len() < 4 {
            return "unknown";
        }

        if bytes.starts_with(b"\x89PNG") {
            "png"
        } else if bytes.starts_with(b"\xFF\xD8\xFF") {
            "jpeg"
        } else if bytes.starts_with(b"GIF8") {
            "gif"
        } else if bytes.starts_with(b"RIFF") && bytes.len() >= 12 && &bytes[8..12] == b"WEBP" {
            "webp"
        } else if bytes.starts_with(b"BM") {
            "bmp"
        } else {
            "unknown"
        }
    }

    /// 从图片头提取尺寸
    fn extract_dimensions(bytes: &[u8], format: &str) -> Option<(u32, u32)> {
        match format {
            "png" => {
                // PNG IHDR chunk: bytes 16-19 = width, 20-23 = height (big-endian)
                if bytes.len() >= 24 {
                    let w = u32::from_be_bytes([bytes[16], bytes[17], bytes[18], bytes[19]]);
                    let h = u32::from_be_bytes([bytes[20], bytes[21], bytes[22], bytes[23]]);
                    Some((w, h))
                } else {
                    None
                }
            }
            "gif" => {
                // GIF: bytes 6-7 = width, 8-9 = height (little-endian)
                if bytes.len() >= 10 {
                    let w = u32::from(u16::from_le_bytes([bytes[6], bytes[7]]));
                    let h = u32::from(u16::from_le_bytes([bytes[8], bytes[9]]));
                    Some((w, h))
                } else {
                    None
                }
            }
            "bmp" => {
                // BMP: bytes 18-21 = width, 22-25 = height (little-endian, signed)
                if bytes.len() >= 26 {
                    let w = u32::from_le_bytes([bytes[18], bytes[19], bytes[20], bytes[21]]);
                    let h_raw = i32::from_le_bytes([bytes[22], bytes[23], bytes[24], bytes[25]]);
                    let h = h_raw.unsigned_abs();
                    Some((w, h))
                } else {
                    None
                }
            }
            "jpeg" => Self::jpeg_dimensions(bytes),
            "webp" => Self::webp_dimensions(bytes),
            _ => None,
        }
    }

    /// 解析 JPEG SOF 标记获取尺寸
    fn jpeg_dimensions(bytes: &[u8]) -> Option<(u32, u32)> {
        let mut i = 2; // 跳过 SOI 标记
        while i + 1 < bytes.len() {
            if bytes[i] != 0xFF {
                return None;
            }
            let marker = bytes[i + 1];
            i += 2;

            // SOF0..SOF3 标记包含尺寸
            if (0xC0..=0xC3).contains(&marker) {
                if i + 7 <= bytes.len() {
                    let h = u32::from(u16::from_be_bytes([bytes[i + 3], bytes[i + 4]]));
                    let w = u32::from(u16::from_be_bytes([bytes[i + 5], bytes[i + 6]]));
                    return Some((w, h));
                }
                return None;
            }

            // 跳过其他标记
            if marker != 0x00 && !(0xD0..=0xD9).contains(&marker) {
                if i + 2 > bytes.len() {
                    return None;
                }
                let len = u16::from_be_bytes([bytes[i], bytes[i + 1]]) as usize;
                i += len;
            }
        }
        None
    }

    /// 解析 WebP VP8 头获取尺寸
    fn webp_dimensions(bytes: &[u8]) -> Option<(u32, u32)> {
        // WebP 文件结构: RIFF....WEBP VP8/VP8L/VP8X
        if bytes.len() < 30 {
            return None;
        }

        // 查找 VP8 或 VP8L chunk
        for i in 12..bytes.len().saturating_sub(4) {
            if &bytes[i..i + 4] == b"VP8 " {
                // VP8 格式: bytes 6-9 包含尺寸信息
                if i + 10 < bytes.len() {
                    let b0 = bytes[i + 6] as u32;
                    let b1 = bytes[i + 7] as u32;
                    let b2 = bytes[i + 8] as u32;
                    let b3 = bytes[i + 9] as u32;
                    let w = (b1 << 8 | b0) & 0x3FFF;
                    let h = (b3 << 8 | b2) & 0x3FFF;
                    return Some((w + 1, h + 1));
                }
            } else if &bytes[i..i + 4] == b"VP8L" {
                // VP8L 格式
                if i + 5 < bytes.len() {
                    let b0 = bytes[i + 5] as u32;
                    let b1 = bytes[i + 6] as u32;
                    let b2 = bytes[i + 7] as u32;
                    let b3 = bytes[i + 8] as u32;
                    let bits = b0 | (b1 << 8) | (b2 << 16) | (b3 << 24);
                    let w = (bits & 0x3FFF) + 1;
                    let h = ((bits >> 14) & 0x3FFF) + 1;
                    return Some((w, h));
                }
            }
        }
        None
    }
}

impl Default for ImageInfoTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for ImageInfoTool {
    fn name(&self) -> &str {
        "image_info"
    }

    fn description(&self) -> Option<&str> {
        Some(
            "读取图片文件的元数据信息。支持 PNG、JPEG、GIF、WebP、BMP 格式。 \
             返回文件格式、尺寸（宽x高）、文件大小等信息。",
        )
    }

    fn categories(&self) -> &'static [ToolCategory] {
        &[ToolCategory::File]
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "图片文件路径"
                }
            },
            "required": ["path"]
        })
    }

    async fn call(&self, input: Value) -> Result<Value, ToolError> {
        let path_str = input
            .get("path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::Message("缺少 'path' 参数".to_string()))?;

        let path = Path::new(path_str);

        // 检查文件是否存在
        if !path.exists() {
            return Err(ToolError::Message(format!("文件不存在: {path_str}")));
        }

        if !path.is_file() {
            return Err(ToolError::Message(format!("路径不是文件: {path_str}")));
        }

        // 获取文件元数据
        let metadata = std::fs::metadata(path)
            .map_err(|e| ToolError::Message(format!("无法读取文件: {e}")))?;

        let file_size = metadata.len();

        // 检查文件大小
        if file_size > MAX_IMAGE_BYTES {
            return Err(ToolError::Message(format!(
                "文件过大 ({file_size} > {MAX_IMAGE_BYTES} bytes)"
            )));
        }

        // 读取文件头
        let mut header = vec![0u8; 1024.min(file_size as usize)];
        use std::io::Read;
        let mut file = std::fs::File::open(path)
            .map_err(|e| ToolError::Message(format!("无法打开文件: {e}")))?;
        file.read_exact(&mut header)
            .map_err(|e| ToolError::Message(format!("无法读取文件: {e}")))?;

        // 检测格式
        let format = Self::detect_format(&header);

        // 提取尺寸
        let dimensions = Self::extract_dimensions(&header, format);

        // 构建结果
        let mut result = json!({
            "path": path_str,
            "format": format,
            "file_size": file_size,
            "file_size_human": format_file_size(file_size),
        });

        if let Some((width, height)) = dimensions {
            result["width"] = json!(width);
            result["height"] = json!(height);
            let ratio = width as f64 / height as f64;
            result["aspect_ratio"] = json!(format!("{ratio:.2}"));
        }

        Ok(result)
    }
}

/// 格式化文件大小为人类可读格式
fn format_file_size(size: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB"];
    let mut size = size as f64;
    let mut unit_index = 0;

    while size >= 1024.0 && unit_index < UNITS.len() - 1 {
        size /= 1024.0;
        unit_index += 1;
    }

    format!("{size:.2} {}", UNITS[unit_index])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_file_size() {
        assert_eq!(format_file_size(512), "512.00 B");
        assert_eq!(format_file_size(1024), "1.00 KB");
        assert_eq!(format_file_size(1024 * 1024), "1.00 MB");
    }

    #[test]
    fn test_detect_format() {
        // PNG
        let png = b"\x89PNG\r\n\x1a\n";
        assert_eq!(ImageInfoTool::detect_format(png), "png");

        // JPEG
        let jpeg = b"\xFF\xD8\xFF\xE0";
        assert_eq!(ImageInfoTool::detect_format(jpeg), "jpeg");

        // GIF
        let gif = b"GIF89a";
        assert_eq!(ImageInfoTool::detect_format(gif), "gif");

        // Unknown
        let unknown = b"UNKNOWN";
        assert_eq!(ImageInfoTool::detect_format(unknown), "unknown");
    }
}
