//! 对应 Java: com.yomahub.liteflow.util.SerialsUtil

use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use chrono::Local;
use rand::RngCore;

const ALPHABET_32: &[u8; 32] = b"BR6UMEHCGQA83SJY75W9FTD2PZNKVXL4";
const ALPHABET_24: &[u8; 24] = b"BRUMEHCGQASJYWFTDPZNKVXL";
const DIVISOR_8: u128 = 99_999_995;
const DIVISOR_12: u128 = 950_000_000_485;
static SERIAL_INT: AtomicUsize = AtomicUsize::new(1);

/// LiteFlow 序列号与短标识生成工具。
pub struct SerialsUtil;

impl SerialsUtil {
    /// 生成“14 位时间 + 3 位随机数 + 3 位循环序号”。
    #[must_use]
    pub fn gen_serial_no() -> String {
        let timestamp = Local::now().format("%Y%m%d%H%M%S");
        let random = rand::random::<u16>() % 999;
        format!("{timestamp}{random:03}{}", Self::next_serial())
    }

    /// 返回 001..999 循环序号。
    #[must_use]
    pub fn next_serial() -> String {
        let serial = SERIAL_INT
            .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |current| {
                Some(if current >= 999 { 1 } else { current + 1 })
            })
            .unwrap_or(1);
        format!("{serial:03}")
    }

    /// 由种子五次幂取模生成 12 位数字。
    #[must_use]
    pub fn random_num12(seed: u64) -> String {
        format!("{:012}", modular_pow5(seed as u128, DIVISOR_12))
    }

    /// 由种子五次幂取模生成 8 位数字。
    #[must_use]
    pub fn random_num8(seed: u64) -> String {
        format!("{:08}", modular_pow5(seed as u128, DIVISOR_8))
    }

    /// 十进制字符串转自定义 32 进制，不使用易混淆的 0/O/1/I。
    pub fn from10_to32(number: &str, size: usize) -> Result<String, std::num::ParseIntError> {
        encode(number.parse()?, size, ALPHABET_32, b'2')
    }

    /// 十进制字符串转自定义 24 进制。
    pub fn from10_to24(number: &str, size: usize) -> Result<String, std::num::ParseIntError> {
        encode(number.parse()?, size, ALPHABET_24, b'B')
    }

    /// 生成无连字符的 32 位十六进制 UUID。
    #[must_use]
    pub fn get_uuid() -> String {
        let mut bytes = [0_u8; 16];
        rand::thread_rng().fill_bytes(&mut bytes);
        bytes[6] = (bytes[6] & 0x0f) | 0x40;
        bytes[8] = (bytes[8] & 0x3f) | 0x80;
        bytes.iter().map(|byte| format!("{byte:02x}")).collect()
    }

    /// 生成 6 位短 UUID。
    #[must_use]
    pub fn generate_short_uuid() -> String {
        Self::from10_to24(&Self::random_num8(nanos()), 6).expect("内部数字格式有效")
    }

    /// 生成 8 位文件 UUID。
    #[must_use]
    pub fn generate_file_uuid() -> String {
        Self::from10_to32(&Self::random_num12(nanos()), 8).expect("内部数字格式有效")
    }

    /// 生成 16 位令牌。
    #[must_use]
    pub fn gen_token() -> String {
        let millis = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
        format!(
            "{}{}",
            Self::from10_to32(&Self::random_num12(millis), 8).expect("内部数字格式有效"),
            Self::generate_file_uuid()
        )
    }
}

fn modular_pow5(seed: u128, modulus: u128) -> u128 {
    let base = seed % modulus;
    (0..4).fold(base, |result, _| (result * base) % modulus)
}

fn encode(
    mut number: u64,
    size: usize,
    alphabet: &[u8],
    padding: u8,
) -> Result<String, std::num::ParseIntError> {
    let mut encoded = Vec::new();
    while number != 0 {
        encoded.push(alphabet[(number % alphabet.len() as u64) as usize]);
        number /= alphabet.len() as u64;
    }
    encoded.resize(size.max(encoded.len()), padding);
    encoded.reverse();
    Ok(String::from_utf8(encoded).expect("字母表仅包含 ASCII"))
}

fn nanos() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos() as u64
}
