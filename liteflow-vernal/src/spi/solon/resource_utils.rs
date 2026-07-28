/// Solon 资源 URL 协议常量集合。
///
/// 用于路径解析器识别 classpath、文件、Jar、War 与常见应用服务器协议。
/// 对应 Java: `com.yomahub.liteflow.spi.solon.ResourceUtils`。
#[derive(Debug, Clone, Copy, Default)]
pub struct ResourceUtils;

impl ResourceUtils {
    /// classpath 单资源前缀。
    pub const CLASSPATH_URL_PREFIX: &'static str = "classpath:";
    /// 本地文件 URL 前缀。
    pub const FILE_URL_PREFIX: &'static str = "file:";
    /// Jar URL 前缀。
    pub const JAR_URL_PREFIX: &'static str = "jar:";
    /// War URL 前缀。
    pub const WAR_URL_PREFIX: &'static str = "war:";
    /// 本地文件协议名。
    pub const URL_PROTOCOL_FILE: &'static str = "file";
    /// Jar 协议名。
    pub const URL_PROTOCOL_JAR: &'static str = "jar";
    /// War 协议名。
    pub const URL_PROTOCOL_WAR: &'static str = "war";
    /// Zip 协议名。
    pub const URL_PROTOCOL_ZIP: &'static str = "zip";
    /// WebSphere Jar 协议名。
    pub const URL_PROTOCOL_WSJAR: &'static str = "wsjar";
    /// JBoss VFS Zip 协议名。
    pub const URL_PROTOCOL_VFSZIP: &'static str = "vfszip";
    /// JBoss VFS 文件协议名。
    pub const URL_PROTOCOL_VFSFILE: &'static str = "vfsfile";
    /// 通用 VFS 协议名。
    pub const URL_PROTOCOL_VFS: &'static str = "vfs";
    /// Jar 文件扩展名。
    pub const JAR_FILE_EXTENSION: &'static str = ".jar";
    /// Jar 内部资源分隔符。
    pub const JAR_URL_SEPARATOR: &'static str = "!/";
    /// War 内部资源分隔符。
    pub const WAR_URL_SEPARATOR: &'static str = "*/";
    /// classpath 全资源前缀。
    pub const CLASSPATH_ALL_URL_PREFIX: &'static str = "classpath*:";
}
