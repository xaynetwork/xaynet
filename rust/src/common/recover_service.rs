use std::path::Path;
use tarpc::context::{current as rpc_context, Context};

use async_trait::async_trait;

#[async_trait]
pub trait SyncRequest {
    async fn reset(&mut self, ctx: Context);
}

#[derive(Clone, Copy)]
pub enum ExitState {
    Clean,
    Dirty(u32),
}

#[async_trait]
pub trait RecoverService {
    fn get_exit_state(&self) -> ExitState;
    async fn sync<C: SyncRequest + Send>(&self, rpc_client: &mut C);
    async fn on_clean_exit(self);
}

pub struct FileRecoverService {
    exit_file_path: String,
    dirty: bool,
    exit_state: ExitState,
}

impl FileRecoverService {
    pub async fn init<S: Into<String>>(
        exit_file_path: S,
        dirty: bool,
    ) -> Result<Self, Box<dyn std::error::Error + 'static>> {
        let exit_file_path = exit_file_path.into();
        let exit_state = Self::read_exit_state(&exit_file_path.clone()).await?;

        Ok(Self {
            exit_file_path: exit_file_path,
            dirty,
            exit_state,
        })
    }

    async fn read_exit_state(
        exit_file_path: &str,
    ) -> Result<ExitState, Box<dyn std::error::Error + 'static>> {
        let exit_file_path = Path::new(exit_file_path);
        if exit_file_path.exists() {
            // the coordinator exited dirty
            let contents = tokio::fs::read(exit_file_path).await?;
            let last_round_number = String::from_utf8_lossy(&contents);
            if last_round_number.is_empty() {
                // the coordinator exited on round zero
                return Ok(ExitState::Dirty(0));
            }

            Ok(ExitState::Dirty(last_round_number.parse()?))
        } else {
            // the coordinator exited clean
            tokio::fs::File::create(exit_file_path).await?;
            Ok(ExitState::Clean)
        }
    }

    async fn remove_exit_file(&self) -> Result<(), std::io::Error> {
        tokio::fs::remove_file(&self.exit_file_path).await
    }
}

#[async_trait]
impl RecoverService for FileRecoverService {
    fn get_exit_state(&self) -> ExitState {
        self.exit_state
    }

    async fn sync<C: SyncRequest + Send>(&self, rpc_client: &mut C) {
        match self.get_exit_state() {
            ExitState::Dirty(r) => {
                warn!("Dirty exit on round {}", r);
                if self.dirty {
                    info!("Skip sync with aggregator");
                    self.remove_exit_file()
                        .await
                        .expect("Cannot remove exit_state file:");
                } else {
                    info!("Try to sync with aggregator...");
                    rpc_client.reset(rpc_context()).await;
                    info!("Success");
                }
            }
            ExitState::Clean => info!("Clean start"),
        }
    }

    async fn on_clean_exit(self) {
        debug!("Mark clean exit");
        self.remove_exit_file()
            .await
            .expect("Cannot remove exit_state file:");
    }
}
