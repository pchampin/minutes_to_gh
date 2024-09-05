use anyhow::Result;
use futures::TryStreamExt;

use crate::args::EngineArgs;

pub async fn command(token: String, args: EngineArgs) -> Result<()> {
    let engine = crate::engine::Engine::new(token, args).await?;
    engine.run().try_for_each_concurrent(None, noop).await?;
    Ok(())
}

async fn noop<T>(_: T) -> Result<()> {
    Ok(())
}
