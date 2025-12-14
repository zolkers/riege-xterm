mod core;

use crate::core::repl_new::Terminal;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let mut terminal = Terminal::new();
    terminal.run().await?;
    Ok(())
}
