//! 对应 Java 类：com.yomahub.liteflow.flow.id.DefaultRequestIdGenerator
//!
//! 默认 Id 生成器。Java 实现为 `IdUtil.fastSimpleUUID()`——即无短横线的
//! 32 位十六进制随机 UUID（ThreadLocalRandom）。Rust 侧不引入 uuid/rand
//! 依赖，用 `RandomState`（每实例随机种子）混合进程内原子序号生成 128 位
//! 随机值，输出同为 32 位小写十六进制字符串，唯一性与格式语义对齐。

use std::collections::hash_map::RandomState;
use std::hash::{BuildHasher, Hasher};
use std::sync::atomic::{AtomicU64, Ordering};

use super::request_id_generator::RequestIdGenerator;

static SEQ: AtomicU64 = AtomicU64::new(0);

/// 生成 64 位随机值：RandomState 随机种子混合进程内自增序号
fn rand_u64() -> u64 {
    let mut h = RandomState::new().build_hasher();
    h.write_u64(SEQ.fetch_add(1, Ordering::Relaxed));
    h.finish()
}

/// 对应 IdUtil.fastSimpleUUID()：32 位无短横线十六进制字符串
pub fn fast_simple_uuid() -> String {
    format!("{:016x}{:016x}", rand_u64(), rand_u64())
}

/// 对应 DefaultRequestIdGenerator
#[derive(Default)]
pub struct DefaultRequestIdGenerator;

impl DefaultRequestIdGenerator {
    /// 创建默认 Request ID 生成器。
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    /// 生成 32 位无短横线十六进制唯一 ID。
    ///
    /// # 返回
    /// 与 Java `IdUtil.fastSimpleUUID()` 格式一致的 Request ID。
    ///
    /// 对应 Java: `DefaultRequestIdGenerator#generate`。
    #[must_use]
    pub fn generate(&self) -> String {
        <Self as RequestIdGenerator>::generate(self)
    }
}

impl RequestIdGenerator for DefaultRequestIdGenerator {
    /// 对应 generate()
    fn generate(&self) -> String {
        fast_simple_uuid()
    }
}
