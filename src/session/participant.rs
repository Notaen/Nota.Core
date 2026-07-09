//! TODO: 该文件需重构，当前仅为测试

#[allow(dead_code)]
pub struct Participant {
    pub name: String,
    pub callback: Box<dyn Fn(&str) + Send + Sync>,
}