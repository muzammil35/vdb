use anyhow::Error;
use fastembed::{
    EmbeddingModel, InitOptionsUserDefined, ModelTrait, QuantizationMode, TextEmbedding,
    TokenizerFiles, UserDefinedEmbeddingModel,
};
use once_cell::sync::OnceCell;
use std::fs;
use std::sync::{Arc, RwLock};
 
use crate::chunk::Chunk;

pub struct Embeddings {
    pub original: Vec<Chunk>,
    pub embedded: Vec<Vec<f32>>,
}

static MODEL_CELL: OnceCell<Arc<RwLock<TextEmbedding>>> = OnceCell::new();

fn initialize_model() -> Result<TextEmbedding, Error> {
    let model_dir = "model";

    let onnx_file = fs::read(format!("{}/model_qint8_arm64.onnx", model_dir))?;
    let tokenizer_file = fs::read(format!("{}/tokenizer.json", model_dir))?;
    let config_file = fs::read(format!("{}/config.json", model_dir))?;
    let special_tokens = fs::read(format!("{}/special_tokens_map.json", model_dir))?;
    let tokenizer_config = fs::read(format!("{}/tokenizer_config.json", model_dir))?;

    let model_data = UserDefinedEmbeddingModel {
        onnx_file,
        tokenizer_files: TokenizerFiles {
            tokenizer_file,
            config_file,
            special_tokens_map_file: special_tokens,
            tokenizer_config_file: tokenizer_config,
        },
        output_key: None,
        pooling: None,
        quantization: QuantizationMode::None,
    };

    TextEmbedding::try_new_from_user_defined(model_data, InitOptionsUserDefined::default())
}

pub fn get_embeddings(original: Vec<Chunk>) -> Result<Embeddings, Error> {
    

    // Initialize model on first call
    let model = MODEL_CELL.get_or_try_init(|| {
        let result = initialize_model();
        result.map(|m| Arc::new(RwLock::new(m)))
    })?;

    // Prepare text data
    let contents: Vec<&str> = original
        .iter()
        .map(|chunk| chunk.content.as_str())
        .collect();

    // Generate embeddings (needs write lock for &mut self)
    let mut model_guard = model.write().unwrap();
    let embedded = model_guard.embed(contents, Some(32 as usize))?;
    drop(model_guard); // Explicit drop for clarity

    Ok(Embeddings { original, embedded })
}

pub fn embed_query(query: &str) -> Result<Vec<f32>, Error> {
    
    let model = MODEL_CELL.get_or_try_init(|| {
        let result = initialize_model();
        result.map(|m| Arc::new(RwLock::new(m)))
    })?;

    // Generate embedding for the single query
    let mut model_guard = model.write().unwrap();
    let embedded = model_guard.embed(vec![query], None)?;
    drop(model_guard);

    // Return the first (and only) embedding
    Ok(embedded.into_iter().next().unwrap())
}

impl Embeddings {
    pub fn get_dim(&self) -> usize {
        let model_info = EmbeddingModel::get_model_info(&EmbeddingModel::AllMiniLML6V2);
        model_info.expect("Model info should always exist").dim
    }
}
