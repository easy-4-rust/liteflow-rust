use liteflow_core::{CmpContext, LiteflowError};
use liteflow_derive::liteflow_cmp_define;
use serde_json::Value;

struct DuplicateMainCmp;

#[liteflow_cmp_define("fallback")]
impl DuplicateMainCmp {
    #[liteflow_method(value = "process", node_id = "duplicate")]
    async fn first(&self, _ctx: &CmpContext) -> Result<Value, LiteflowError> {
        Ok(Value::Null)
    }

    #[liteflow_method(value = "process", node_id = "duplicate")]
    async fn second(&self, _ctx: &CmpContext) -> Result<Value, LiteflowError> {
        Ok(Value::Null)
    }
}

fn main() {}
