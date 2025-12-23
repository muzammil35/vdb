// Alternative implementation using OnceCell with interior mutability
use once_cell::sync::OnceCell;
use std::sync::RwLock;

use anyhow::Error;
use fastembed::{TextEmbedding, UserDefinedEmbeddingModel, InitOptionsUserDefined, TokenizerFiles, QuantizationMode, EmbeddingModel};
use fastembed::ModelTrait;
use std::fs;
use std::sync::{Arc, Mutex};
use std::time::Instant;
use once_cell::sync::Lazy;

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
    
    TextEmbedding::try_new_from_user_defined(
        model_data,
        InitOptionsUserDefined::default()
    )
}

pub fn get_embeddings(original: Vec<Chunk>) -> Result<Embeddings, Error> {
    let function_start = Instant::now();
    
    // Initialize model on first call
    let model = MODEL_CELL.get_or_try_init(|| {
        println!("\n🔄 Initializing embedding model (first time only)...");
        let init_start = Instant::now();
        let result = initialize_model();
        let init_time = init_start.elapsed();
        
        match &result {
            Ok(_) => println!("✅ Model initialized in {:?}", init_time),
            Err(e) => println!("❌ Model initialization failed: {}", e),
        }
        
        result.map(|m| Arc::new(RwLock::new(m)))
    })?;
    
    // Prepare text data
    let contents: Vec<&str> = original
        .iter()
        .map(|chunk| chunk.content.as_str())
        .collect();
    
    println!("🚀 Generating embeddings for {} chunks...", contents.len());
    let embed_start = Instant::now();

    // Generate embeddings (needs write lock for &mut self)
    let mut model_guard = model.write().unwrap();
    let embedded = model_guard.embed(contents, Some(32 as usize))?;
    drop(model_guard); // Explicit drop for clarity
    
    let embed_time = embed_start.elapsed();
    let total_time = function_start.elapsed();
    
    println!("✅ Generated embeddings in {:?} (total: {:?})", embed_time, total_time);
    
    Ok(Embeddings { original, embedded })
}

impl Embeddings {
    pub fn get_dim(&self) -> usize {
        let model_info = EmbeddingModel::get_model_info(&EmbeddingModel::AllMiniLML6V2);
        model_info.expect("Model info should always exist").dim
    }
}
