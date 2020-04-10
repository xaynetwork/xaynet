use std::path::Path;

pub enum ExitState {
    Clean,
    Dirty(u32),
}

pub async fn get_last_exit_state(
    path: &str,
) -> Result<ExitState, Box<dyn std::error::Error + 'static>> {
    let path = Path::new(path);

    if path.exists() {
        // the coordinator exited dirty
        let contents = tokio::fs::read(path).await?;
        let last_round_number = String::from_utf8_lossy(&contents);
        if last_round_number.is_empty() {
            // the coordinator exited on round zero
            return Ok(ExitState::Dirty(0));
        }

        Ok(ExitState::Dirty(last_round_number.parse()?))
    } else {
        // the coordinator exited clean
        tokio::fs::File::create(path).await?;
        Ok(ExitState::Clean)
    }
}

pub async fn mark_clean_exit(path: &str) {
    let path = Path::new(path);
    tokio::fs::remove_file(path)
        .await
        .expect("Cannot remove exit_state file:");
}
