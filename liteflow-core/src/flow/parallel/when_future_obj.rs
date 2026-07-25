use std::sync::{Arc, Mutex, MutexGuard};

use anyhow::Context;
use futures::future::BoxFuture;
use futures::{Future, FutureExt};

use crate::exception::data_not_found_exception::DataNotFoundException;
use crate::flow::error::FlowResult;
use crate::flow::parallel::default_context_bean::DefaultContextBean;
use crate::flow::parallel::parallel_supplier::ParallelSupplier;
use crate::slot::{DataBus, Slot};

pub struct WhenFutureObj {
    pub slot_key: usize,
    pub name: String,
    pub(crate) inner: Arc<Mutex<Inner>>,
}

impl WhenFutureObj {
    pub fn new(supplier: ParallelSupplier) -> Self {
        let inner = supplier.inner;
        Self {
            slot_key: supplier.slot_key,
            name: supplier.name,
            inner,
        }
    }
}

impl Clone for WhenFutureObj {
    fn clone(&self) -> Self {
        Self {
            slot_key: self.slot_key,
            name: self.name.clone(),
            inner: self.inner.clone(),
        }
    }
}

impl Future for WhenFutureObj {
    type Output = FlowResult<WhenFutureObj>;

    fn poll(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        let mut guard = self.lock();

        // 先检查 future 是否已经完成，以及是否已经获取过结果
        if guard.future.is_none() {
            return std::task::Poll::Ready(Err(DataNotFoundException::with_err_message(
                format!("Future '{}' was already consumed.", self.name),
            )
            .into()));
        }

        if guard.result.is_some() {
            let result = guard
                .result
                .take()
                .expect("[ERROR] Result should be Some, but found None");
            return std::task::Poll::Ready(Ok(Self {
                slot_key: self.slot_key,
                name: self.name.clone(),
                inner: Arc::new(Mutex::new(Inner {
                    future: None,
                    result: Some(result),
                })),
            }));
        }

        // poll 内部的 future
        let inner_future = guard
            .future
            .as_mut()
            .expect("[ERROR] Future should be Some, but found None");

        match inner_future.poll_unpin(cx) {
            std::task::Poll::Ready(result) => {
                guard.result = Some(result);
                guard.future = None;

                std::task::Poll::Ready(Ok(self.clone()))
            }
            std::task::Poll::Pending => {
                // future 还没完成
                std::task::Poll::Pending
            }
        }
    }
}

pub struct Inner {
    pub future: Option<BoxFuture<'static, FlowResult<()>>>,
    pub result: Option<FlowResult<()>>,
}

impl WhenFutureObj {
    /// 加锁获取内部状态
    fn lock(&self) -> MutexGuard<'_, Inner> {
        self.inner.lock().expect("Failed to lock inner state")
    }

    /// 检查 future 是否已经完成
    pub fn is_completed(&self) -> bool {
        self.lock().future.is_none()
    }

    /// 获取结果（会清除内部状态，防止多次调用）
    pub fn get_result(&self) -> anyhow::Result<Option<FlowResult<()>>> {
        self.lock()
            .result
            .take()
            .context("No result available for future")
    }

    /// 获取结果（使用自定义错误消息）
    pub fn get_result_timed(&self, error_message: &str) -> anyhow::Result<Option<FlowResult<()>>> {
        self.lock().result.take().context(error_message.to_string())
    }

    /// 初始化上下文
    pub fn init_context(&self) -> anyhow::Result<()> {
        let mut slot = DataBus::get_slot(self.slot_key).context("Failed to get slot")?;
        let default_context_bean = Arc::new(DefaultContextBean::new(self.clone()));

        slot.set_default_context_bean(self.name.clone(), default_context_bean);
        Ok(())
    }
}
