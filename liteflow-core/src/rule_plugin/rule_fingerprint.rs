//! 规则变更指纹工具。

/// 使用 FNV-1a 计算规则文本指纹。
#[must_use]
pub fn fnv_fp(text: &str) -> String {
    let mut hash: u64 = 0xcbf29ce484222325;
    for byte in text.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("{hash:x}")
}
