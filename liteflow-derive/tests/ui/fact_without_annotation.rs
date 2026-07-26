#![allow(unused_imports)]

use std::sync::Arc;

use liteflow_core::{CmpContext, LiteflowError};
use liteflow_derive::liteflow_cmp_define;

struct InvalidFact;

#[liteflow_cmp_define("invalidFact")]
impl InvalidFact {
    #[liteflow_method("process")]
    async fn process(
        &self,
        _ctx: &CmpContext,
        missing_annotation: Arc<String>,
    ) -> Result<serde_json::Value, LiteflowError> {
        Ok(serde_json::json!(missing_annotation.as_str()))
    }
}

fn main() {}
