use std::future::Future;
use std::sync::Arc;

use nekoton::core::keystore::KeyStore;
use tokio::sync::Mutex;
use tokio::task::JoinHandle;

use crate::{ExitCode, GqlTransport, TonWalletSubscription};
use crate::get_runtime;
use crate::global::RUNTIME_;

#[derive(Clone)]
pub struct Context {
    pub wallet_state: Arc<TonWalletSubscription>,
    pub transport: Arc<GqlTransport>,
    pub keystore: Arc<Mutex<KeyStore>>,
    pub manager: Arc<TaskManager>,
}

impl Context {
    pub fn new(
        wallet_state: TonWalletSubscription,
        transport: Arc<GqlTransport>,
        keystore: KeyStore,
        manager: TaskManager,
    ) -> Self {
        Self {
            wallet_state: Arc::new(wallet_state),
            transport,
            keystore: Arc::new(Mutex::new(keystore)),
            manager: Arc::new(manager),
        }
    }

    pub fn spawn<F>(&self, future: F) -> ExitCode
        where
            F: Future + Send + 'static,
            F: Future<Output=()> + Send + 'static {
        let e = get_runtime!().handle();
        let h = e.spawn(future);
        e.block_on(self.manager.track(h));
        ExitCode::Ok
    }
}

#[derive(Clone, Default)]
pub struct TaskManager {
    tasks: Arc<Mutex<Vec<JoinHandle<()>>>>,
}

impl Drop for TaskManager {
    fn drop(&mut self) {
        let e = RUNTIME_.as_ref().expect("Drop can't be called, when no features are spawned").handle();
        let tasks = self.tasks.clone();
        e.block_on(async move {
            let tasks = tasks.lock().await;
            for task in tasks.iter() {
                task.abort();
            }
        });
    }
}

impl TaskManager {
    pub async fn track(&self, task: JoinHandle<()>) {
        self.tasks.lock().await.push(task)
    }
}
