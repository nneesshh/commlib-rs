use std::sync::Arc;

use crate::{NodeState, ServiceHandle, ServiceRs};

use super::http_client::{http_client_get, http_client_post, http_client_update};

/// ServiceHttpClientRs
pub struct ServiceHttpClientRs {
    pub handle: ServiceHandle,
}

impl ServiceHttpClientRs {
    ///
    pub fn new(id: u64) -> Self {
        Self {
            handle: ServiceHandle::new(id, NodeState::Idle),
        }
    }

    ///
    pub fn http_get<F>(self: &Arc<Self>, url: &str, cb: F)
    where
        F: Fn(u32, String) + Send + Sync + 'static,
    {
        // 投递到 srv_http_cli 线程
        let srv_http_cli = self.clone();
        let url = url.to_owned();
        self.run_in_service(Box::new(move || {
            //
            let headers = vec![];
            http_client_get(url, headers, cb, &srv_http_cli);
        }));
    }

    ///
    pub fn http_post<F>(self: &Arc<Self>, url: &str, headers: Vec<String>, data: String, cb: F)
    where
        F: Fn(u32, String) + Send + Sync + 'static,
    {
        // 投递到 srv_http_cli 线程
        let srv_http_cli = self.clone();
        let url = url.to_owned();
        self.run_in_service(Box::new(move || {
            //
            http_client_post(url, data, headers, cb, &srv_http_cli);
        }));
    }
}

impl ServiceRs for ServiceHttpClientRs {
    /// 获取 service nmae
    #[inline(always)]
    fn name(&self) -> &str {
        "service_http_client"
    }

    /// 获取 service 句柄
    #[inline(always)]
    fn get_handle(&self) -> &ServiceHandle {
        &self.handle
    }

    /// 配置 service
    fn conf(&self) {}

    /// update
    #[inline(always)]
    fn update(&self) {
        //
        http_client_update(self);
    }

    /// 在 service 线程中执行回调任务
    #[inline(always)]
    fn run_in_service(&self, cb: Box<dyn FnOnce() + Send>) {
        self.get_handle().run_in_service(cb);
    }

    /// 当前代码是否运行于 service 线程中
    #[inline(always)]
    fn is_in_service_thread(&self) -> bool {
        self.get_handle().is_in_service_thread()
    }

    /// 等待线程结束
    fn join(&self) {
        //
        self.get_handle().join_service();
    }
}
