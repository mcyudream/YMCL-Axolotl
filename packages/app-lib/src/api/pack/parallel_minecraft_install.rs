use crate::install::InstallProgressReporter;
use tokio_util::sync::CancellationToken;

/// Owns a Minecraft core installation that runs alongside pack content work.
/// Dropping the guard cancels the task so an importer cannot leave a core
/// download running after its content installation fails.
pub(crate) struct ParallelMinecraftInstall {
    cancel: CancellationToken,
    task: Option<tokio::task::JoinHandle<crate::Result<()>>>,
    reporter: InstallProgressReporter,
}

impl ParallelMinecraftInstall {
    pub(crate) fn start(
        instance_id: String,
        reporter: InstallProgressReporter,
    ) -> Self {
        let cancel = CancellationToken::new();
        let task_cancel = cancel.clone();
        let parallel_reporter = reporter.with_parallel_output();
        let failure_reporter = parallel_reporter.clone();
        let task = tokio::spawn(async move {
            tokio::select! {
                _ = task_cancel.cancelled() => {
                    tracing::debug!(
                        instance_id = %instance_id,
                        "Parallel Minecraft install aborted before completion"
                    );
                    Ok(())
                }
                result = crate::launcher::install_minecraft_for_instance_id_with_reporter(
                    &instance_id,
                    false,
                    Some(parallel_reporter),
                    crate::launcher::InstanceCompletionPolicy::DeferToInstallJob,
                ) => result,
            }
        });

        Self {
            cancel,
            task: Some(task),
            reporter: failure_reporter,
        }
    }

    /// Cancels the core install and waits until the task has stopped.
    pub(crate) async fn abort(mut self) {
        self.cancel.cancel();
        if let Some(task) = self.task.take() {
            let _ = task.await;
        }
    }

    /// Waits for the core install to finish without cancelling it.
    pub(crate) async fn join(mut self) -> crate::Result<()> {
        if let Some(task) = self.task.take() {
            let result = task.await?;
            if result.is_err()
                && let Err(error) =
                    self.reporter.promote_parallel_failure().await
            {
                tracing::warn!(%error, "Failed to restore parallel Minecraft failure context");
            }
            result?;
        }
        Ok(())
    }
}

impl Drop for ParallelMinecraftInstall {
    fn drop(&mut self) {
        self.cancel.cancel();
    }
}
