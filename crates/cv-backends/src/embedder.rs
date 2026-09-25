use cv_engine::Embedder;
use async_trait::async_trait;
use std::path::PathBuf;
use std::sync::Arc;

pub struct NoOpEmbedder;

#[async_trait]
impl Embedder for NoOpEmbedder {
    async fn embed(&self, _text: &str) -> anyhow::Result<Vec<f32>> {
        Ok(vec![0.0; 384])
    }

    fn dim(&self) -> usize {
        384
    }
}

pub struct FastembedEmbedder {
    model: Arc<fastembed::TextEmbedding>,
}

impl FastembedEmbedder {
    /// `cache_dir` is where the ~87 MB model is downloaded to / loaded from.
    /// fastembed's own `InitOptions::default()` falls back to a bare relative
    /// path (`.fastembed_cache`), which resolves against the process CWD — the
    /// caller must resolve an absolute, per-user directory and pass it in here
    /// rather than relying on that default. (fastembed still lets `HF_HOME`
    /// override this at download time if the user has it set — see
    /// `pull_from_hf` in fastembed's `common.rs` — so this is only the
    /// fallback, not the last word.)
    pub fn new(cache_dir: PathBuf) -> anyhow::Result<Self> {
        let mut opts = fastembed::InitOptions::default();
        opts.model_name = fastembed::EmbeddingModel::AllMiniLML6V2;
        opts.show_download_progress = true;
        opts.cache_dir = cache_dir;
        let model = fastembed::TextEmbedding::try_new(opts)?;
        Ok(Self { model: Arc::new(model) })
    }
}

#[async_trait]
impl Embedder for FastembedEmbedder {
    async fn embed(&self, text: &str) -> anyhow::Result<Vec<f32>> {
        let text = text.to_string();
        let model = self.model.clone();
        tokio::task::spawn_blocking(move || {
            let mut results = model.embed(vec![text.as_str()], None)?;
            results.pop().ok_or_else(|| anyhow::anyhow!("empty embedding result"))
        })
        .await?
    }

    fn dim(&self) -> usize {
        384
    }
}
