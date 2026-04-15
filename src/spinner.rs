use crate::output;
use anyhow::Result;
use indicatif::{ProgressBar, ProgressStyle};
use std::future::Future;

pub trait SpinnerSuccess {
    fn print_success(&self);
}

impl SpinnerSuccess for String {
    fn print_success(&self) {
        output::success(self);
    }
}

pub async fn step_spinner<Fut, T>(title: &str, fut: Fut) -> Result<T>
where
    Fut: Future<Output = Result<T>>,
    T: SpinnerSuccess,
{
    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::with_template("{spinner:.cyan} {msg}")
            .unwrap()
            .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"]),
    );
    pb.set_message(title.to_string());
    pb.enable_steady_tick(std::time::Duration::from_millis(80));
    match fut.await {
        Ok(t) => {
            pb.finish_and_clear();
            t.print_success();
            Ok(t)
        }
        Err(e) => {
            pb.finish_and_clear();
            Err(e)
        }
    }
}
