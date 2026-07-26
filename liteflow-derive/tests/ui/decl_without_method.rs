use liteflow_derive::liteflow_cmp_define;

struct EmptyDecl;

#[liteflow_cmp_define("empty")]
impl EmptyDecl {
    async fn helper(&self, _ctx: &liteflow_core::CmpContext) {}
}

fn main() {}
