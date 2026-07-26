use liteflow_derive::liteflow_retry;

struct WrongTarget;

#[liteflow_retry(1)]
impl WrongTarget {
    fn helper(&self) {}
}

fn main() {}
