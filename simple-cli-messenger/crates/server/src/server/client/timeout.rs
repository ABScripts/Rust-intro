use std::pin::Pin;
use std::time::Duration;
use tokio::time::{Sleep, sleep};

pub struct Timeout<F>
where
    F: Future<Output = anyhow::Result<()>>,
{
    duration: Duration,
    sleep_fut: Pin<Box<Sleep>>,
    task: Pin<Box<F>>,
}

impl<F> Timeout<F>
where
    F: Future<Output = anyhow::Result<()>>,
{
    pub fn new(duration: Duration, task: F) -> Self {
        Self {
            duration,
            sleep_fut: Box::pin(sleep(duration)),
            task: Box::pin(task),
        }
    }
}

impl<F: Future> Future for Timeout<F>
where
    F: Future<Output = anyhow::Result<()>>,
{
    type Output = anyhow::Result<()>;

    fn poll(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        if self.sleep_fut.as_mut().poll(cx) == std::task::Poll::Ready(()) {
            tracing::debug!(
                "Timeout! Task was inactive for too long: {:?}",
                self.duration
            );
            return std::task::Poll::Ready(Ok(()));
        }

        tracing::debug!("Received update on task, restart timer");
        let duration = self.duration;
        self.sleep_fut
            .as_mut()
            .reset(tokio::time::Instant::now() + duration);

        self.task.as_mut().poll(cx)
    }
}
