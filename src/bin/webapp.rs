use axum::{
    Router, body::Bytes, extract::{Json, Multipart, Path, Query, State}, response::{Html, IntoResponse},
    extract::DefaultBodyLimit,
    routing::{get, post}
};
use axum::{http::StatusCode, http::header};
use serde::Deserialize;
use serde::Serialize;
use std::net::SocketAddr;
use tower_http::services::ServeDir;
use std::fs;
use vb::chunk;
use vb::embed;
use vb::extract;
use vb::qdrant;
use std::time::Instant;
use qdrant_client::Qdrant;
use std::sync::Arc;
use tokio::sync::RwLock;
use std::collections::HashMap;
use uuid::Uuid;

// Shared state for ID to filename mapping
type IdToFilenameMap = Arc<RwLock<HashMap<String, String>>>;

#[derive(Serialize)]
struct SearchResult {
    page: i64,
    text: String,
}

#[derive(Serialize)]
struct UploadResponse {
    id: String,
}

// Query parameter structure for /api/search
#[derive(Deserialize)]
struct SearchQuery {
    q: String,
    id: String,
}

#[tokio::main]
async fn main() {
    // Create shared state for ID to filename mapping
    let id_map: IdToFilenameMap = Arc::new(RwLock::new(HashMap::new()));

    // Build our application with routes
    let app = Router::new()
        .route("/", get(index))
        .route("/upload", post(handle_upload))
        .route("/api/search", get(search_handler))
        .nest_service("/static", ServeDir::new("static"))
        .layer(DefaultBodyLimit::max(10 * 1024 * 1024)) // 10MB
        .with_state(id_map);

    // Run the server
    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    println!("Server running on http://{}", addr);
    
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn index() -> Result<Html<String>, StatusCode> {
    match fs::read_to_string("static/webapp/render.html") {
        Ok(contents) => Ok(Html(contents)),
        Err(_) => Err(StatusCode::NOT_FOUND),
    }
}

async fn handle_upload(
    State(id_map): State<IdToFilenameMap>,
    mut multipart: Multipart,
) -> Result<impl IntoResponse, StatusCode> {
    while let Some(field) = multipart.next_field().await.unwrap() {
        let name = field.name().unwrap().to_string();
        let filename = field.file_name().unwrap().to_string();
        let data = field.bytes().await.unwrap();

        if name == "pdf" {
            println!("Received file: {} ({} bytes)", filename, data.len());
            
            // Generate unique ID for this upload
            let id = Uuid::new_v4().to_string();
            
            // Store the mapping
            {
                let mut map = id_map.write().await;
                map.insert(id.clone(), filename.clone());
            }
            
            // Spawn background processing
            let data_clone = data.to_vec();
            let filename_clone = filename.clone();
            tokio::task::spawn(async move {
                let start = Instant::now();
                match process_file(&filename_clone, data_clone.into()).await {
                    Ok(_) => println!("Processing done: {:?}", start.elapsed()),
                    Err(e) => eprintln!("Processing failed: {:?}", e),
                }
            });
            
            // Return the ID as JSON
            return Ok((
                StatusCode::OK,
                Json(UploadResponse { id })
            ));
        }
    }
    Err(StatusCode::BAD_REQUEST)
}

async fn process_file(filename:&str, pdf_data: Bytes) -> Result<(), Box<dyn std::error::Error>> {
    let chunks = chunk::extract_and_chunk(chunk::PdfSource::Bytes(pdf_data.to_vec()))?;
    let embedded_chunks = embed::get_embeddings(chunks)?;
    let client = qdrant::setup_qdrant(&embedded_chunks, filename).await?;
    let response = qdrant::store_embeddings(&client, filename, embedded_chunks).await?;

    println!("File processed successfully!");
    dbg!(response);

    Ok(())
}

// Handler for /api/search endpoint
async fn search_handler(
    State(id_map): State<IdToFilenameMap>,
    Query(params): Query<SearchQuery>,
) -> Result<Json<Vec<SearchResult>>, (StatusCode, String)> {
    let query = params.q.trim();
    let id = params.id.trim();
    
    if query.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            "Query parameter 'q' cannot be empty".to_string(),
        ));
    }
    
    if id.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            "Query parameter 'id' cannot be empty".to_string(),
        ));
    }

    // Get filename from ID
    let collection_name = {
        let map = id_map.read().await;
        map.get(id)
            .ok_or_else(|| (
                StatusCode::NOT_FOUND,
                format!("ID '{}' not found", id),
            ))?
            .clone()
    };

    match run_search_api(&collection_name, query.to_string()).await {
        Ok(results) => Ok(Json(results)),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Search failed: {}", e),
        )),
    }
}

// API version of search (returns JSON)
async fn run_search_api(
    collection_name: &str,
    query: String,
) -> Result<Vec<SearchResult>, Box<dyn std::error::Error>> {
    let query = query.trim();
    if query.is_empty() {
        return Ok(vec![]);
    }

    let client = Qdrant::from_url("http://localhost:6334").build()?;
    let resp = qdrant::run_query(&client, collection_name, &query).await?;

    let mut results = Vec::new();
    
    for point in resp.result {
        if let Some(text_value) = point.payload.get("text") {
            if let Some(page_value) = point.payload.get("page") {
                if let Some(text) = text_value.as_str() {
                    // Extract page number - handle different number types
                    use qdrant_client::qdrant::value::Kind;
                    
                    let page = match &page_value.kind {
                        Some(Kind::DoubleValue(d)) => *d as i64,
                        Some(Kind::IntegerValue(i)) => *i,
                        Some(Kind::StringValue(s)) => s.parse::<i64>().unwrap_or(1),
                        _ => 1,
                    };
                        
                    results.push(SearchResult {
                        page,
                        text: text.to_string(),
                    });
                }
            }
        }
    }

    Ok(results)
}   

