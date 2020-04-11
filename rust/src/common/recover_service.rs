use async_trait::async_trait;
use std::path::Path;

#[derive(Clone, Copy)]
pub enum ExitState {
    Clean,
    Dirty(u32),
}

#[async_trait]
pub trait RecoverService {
    fn get_exit_state(&self) -> ExitState;
    async fn on_clean_exit(self);
}
#[derive(Clone)]
pub struct FileRecoverService {
    failure_lock_path: String,
    exit_state: ExitState,
}

impl FileRecoverService {
    pub async fn init<S: Into<String>>(
        failure_lock_path: S,
        dirty: bool,
    ) -> Result<Self, Box<dyn std::error::Error + 'static>> {
        let failure_lock_path = failure_lock_path.into();
        let exit_state = Self::read_exit_state(&failure_lock_path[..], dirty).await?;

        Ok(Self {
            failure_lock_path: failure_lock_path,
            exit_state,
        })
    }

    async fn read_exit_state(
        failure_lock_path: &str,
        dirty: bool,
    ) -> Result<ExitState, Box<dyn std::error::Error + 'static>> {
        let failure_lock_path = Path::new(failure_lock_path);
        if failure_lock_path.exists() {
            // the coordinator exited dirty
            if dirty {
                debug!("Ignored dirty state.");
                return Ok(ExitState::Clean);
            }

            let contents = tokio::fs::read(failure_lock_path).await?;
            let last_round_number = String::from_utf8_lossy(&contents);
            if last_round_number.is_empty() {
                // the coordinator exited on round zero
                return Ok(ExitState::Dirty(0));
            }

            Ok(ExitState::Dirty(last_round_number.parse()?))
        } else {
            // the coordinator exited clean
            tokio::fs::File::create(failure_lock_path).await?;
            Ok(ExitState::Clean)
        }
    }

    async fn remove_exit_file(&self) -> Result<(), std::io::Error> {
        tokio::fs::remove_file(&self.failure_lock_path).await
    }
}

#[async_trait]
impl RecoverService for FileRecoverService {
    fn get_exit_state(&self) -> ExitState {
        self.exit_state
    }

    async fn on_clean_exit(self) {
        debug!("Mark clean exit");
        self.remove_exit_file()
            .await
            .expect("Cannot remove exit_state file:");
    }
}
