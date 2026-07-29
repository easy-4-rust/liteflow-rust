use liteflow_core::{CmpContext, LiteflowError};
use liteflow_derive::liteflow_cmp_define;
use serde_json::Value;

struct MissingMainCmp;

#[liteflow_cmp_define("fallback")]
impl MissingMainCmp {
    #[liteflow_method(value = "on_success", node_id = "missingMain")]
    async fn success(&self, _ctx: &CmpContext) -> Result<Value, LiteflowError> {
        Ok(Value::Null)
    }
}

fn main() {}
