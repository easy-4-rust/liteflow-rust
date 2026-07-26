//! 默认会话 ID 生成器。
//!
//! 对应 Java: `com.yomahub.liteflow.util.ConversationIdGenerator`。

use chrono::Local;
use rand::seq::SliceRandom;

const CODE_ALPHABET: &[u8] = b"123456789ABCDEFGHIJKLMNPQRSTUVWXYZ";

/// 生成 `yyyyMMdd_` 前缀和 12 位 NanoId 风格随机码。
pub struct ConversationIdGenerator;

impl ConversationIdGenerator {
    /// 生成一次业务会话标识。
    ///
    /// 返回值供同一条 chain 内的组件共享。对应 Java:
    /// `ConversationIdGenerator#generate`。
    #[must_use]
    pub fn generate() -> String {
        let date = Local::now().format("%Y%m%d");
        let mut rng = rand::thread_rng();
        let code: String = (0..12)
            .map(|_| {
                *CODE_ALPHABET
                    .choose(&mut rng)
                    .expect("conversation alphabet is not empty") as char
            })
            .collect();
        format!("{date}_{code}")
    }
}
