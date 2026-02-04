use axum::extract::Query;
use clap::{Arg, Parser, arg, command};
use qdrant_client::Qdrant;
use serde::Deserialize;
use std::fs;
use std::io::{self, BufRead, Write};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt};
use axum::{Json, Router, body::Body, http::StatusCode, response::Html, routing::get};
use serde::Serialize;

pub mod chunk;
pub mod embed;
pub mod extract;
pub mod qdrant;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(short, long)]
    file: Option<String>,

    #[arg(short, long)]
    search: Option<String>,
}

// Response structure for the search API
#[derive(Serialize)]
struct SearchResult {
    page: i64,
    text: String,
}

// Query parameter structure for /api/search
#[derive(Deserialize)]
struct SearchQuery {
    q: String,
}

#[tokio::main]
async fn main() {
    let banner = r#"
 ██████╗ ██╗   ██╗███████╗██████╗ ██╗   ██╗
██╔═══██╗██║   ██║██╔════╝██╔══██╗╚██╗ ██╔╝
██║   ██║██║   ██║█████╗  ██████╔╝ ╚████╔╝ 
██║▄▄ ██║██║   ██║██╔══╝  ██╔══██╗  ╚██╔╝  
╚██████╔╝╚██████╔╝███████╗██║  ██║   ██║   
 ╚══▀▀═╝  ╚═════╝ ╚══════╝╚═╝  ╚═╝   ╚═╝   
"#;

    println!("{}", banner);
    println!("Type 'help' for available commands, 'exit' to quit");

    if let Err(e) = run_repl().await {
        eprintln!("Error: {}", e);
    }
}

async fn run_repl() -> Result<(), Box<dyn std::error::Error>> {
    let stdin = io::stdin();
    let mut reader = stdin.lock();

    loop {
        print!("> ");
        io::stdout().flush()?;

        let mut input = String::new();
        reader.read_line(&mut input)?;
        let input = input.trim();

        if input.is_empty() {
            continue;
        }

        // Parse the command
        let parts: Vec<&str> = input.split_whitespace().collect();

        match parts.first().map(|s| *s) {
            Some("exit") | Some("quit") => {
                println!("Goodbye!");
                break;
            }
            Some("help") => {
                print_help();
            }
            Some("file") => {
                if parts.len() < 2 {
                    println!("Usage: file <path>");
                    continue;
                }
                let file_path = parts[1];
                if let Err(e) = process_file(file_path).await {
                    eprintln!("Error processing file: {}", e);
                }
            }
            Some("search") => {
                if parts.len() < 3 {
                    println!("Usage: search <collection_name> <query>");
                    continue;
                }
                let collection_name = parts[1];
                let query = parts[2..].join(" ");
                if let Err(e) = run_search_repl(collection_name, query).await {
                    eprintln!("Error searching: {}", e);
                }
            }
            Some("serve") => {
                if parts.len() < 3 {
                    println!("Usage: serve <file_path> <collection_name>");
                    continue;
                }
                let file_path = parts[1];
                let collection_name = parts[2];
                if let Err(e) = start_server(file_path, collection_name).await {
                    eprintln!("Error starting server: {}", e);
                }
            }
            Some(cmd) => {
                println!(
                    "Unknown command: {}. Type 'help' for available commands.",
                    cmd
                );
            }
            None => {}
        }
    }

    Ok(())
}

fn print_help() {
    println!("Available commands:");
    println!("  file <path>                        - Process and index a file");
    println!("  search <collection> <query>        - Search in a collection");
    println!("  serve <file_path> <collection>     - Start web server with PDF viewer and search API");
    println!("  help                               - Show this help message");
    println!("  exit/quit                          - Exit the program");
}

async fn process_file(file_path: &str) -> Result<(), Box<dyn std::error::Error>> {
    println!("Processing file: {}", file_path);

    let file = extract::extract_pdf_file(file_path);

    let pages = file.get_pages();
    let chunks = chunk::chunk_per_page(pages);
    let embedded_chunks = embed::get_embeddings(chunks)?;
    let client = qdrant::setup_qdrant(&embedded_chunks, file_path).await?;
    let response = qdrant::store_embeddings(&client, file_path, embedded_chunks).await?;

    println!("File processed successfully!");
    dbg!(response);

    Ok(())
}

// REPL version of search (prints to console)
async fn run_search_repl(
    collection_name: &str,
    query: String,
) -> Result<(), Box<dyn std::error::Error>> {
    let query = query.trim();
    if query.is_empty() {
        println!("No query entered.");
        return Ok(());
    }

    let client = Qdrant::from_url("http://localhost:6334").build()?;
    let resp = qdrant::run_query(&client, collection_name, query).await?;

    println!("\nSearch Results:");
    println!("===============");
    for point in resp.result {
        if let Some(text_value) = point.payload.get("text") {
            let page = point.payload.get("page").unwrap();
            if let Some(text) = text_value.as_str() {
                println!("-----");
                println!("{:?}", page);
                println!("{}", text);
            }
        }
    }

    Ok(())
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

// Handler for /api/search endpoint
async fn search_handler(
    Query(params): Query<SearchQuery>,
    axum::extract::State(collection_name): axum::extract::State<String>,
) -> Result<Json<Vec<SearchResult>>, (StatusCode, String)> {
    let query = params.q.trim();
    
    if query.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            "Query parameter 'q' cannot be empty".to_string(),
        ));
    }

    match run_search_api(&collection_name, query.to_string()).await {
        Ok(results) => Ok(Json(results)),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Search failed: {}", e),
        )),
    }
}

async fn start_server(file_path: &str, collection_name: &str) -> Result<(), Box<dyn std::error::Error>> {
    let file_path = file_path.to_string();
    let collection_name = collection_name.to_string();

    // Verify the file exists
    if !std::path::Path::new(&file_path).exists() {
        return Err(format!("File not found: {}", file_path).into());
    }

    // Build the router with state for collection_name
    let app = Router::new()
        .route("/", get(render_pdf))
        .route(
            "/api/pdf",
            get({
                let path = file_path.clone();
                move || serve_pdf(path.clone())
            }),
        )
        .route("/api/search", get(search_handler))
        .with_state(collection_name.clone());

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000").await?;

    println!("Server running on http://127.0.0.1:3000");
    println!("Serving PDF: {}", file_path);
    println!("Search collection: {}", collection_name);
    println!("API endpoints:");
    println!("  - PDF viewer: http://127.0.0.1:3000/");
    println!("  - PDF file: http://127.0.0.1:3000/api/pdf");
    println!("  - Search: http://127.0.0.1:3000/api/search?q=<query>");
    println!("Press Ctrl+C to stop the server");

    axum::serve(listener, app).await?;

    Ok(())
}

async fn serve_pdf(file_path: String) -> Result<(StatusCode, Body), StatusCode> {
    match fs::read(&file_path) {
        Ok(contents) => Ok((StatusCode::OK, Body::from(contents))),
        Err(_) => Err(StatusCode::NOT_FOUND),
    }
}

async fn render_pdf() -> Result<Html<String>, StatusCode> {
    match fs::read_to_string("static/index.html") {
        Ok(contents) => Ok(Html(contents)),
        Err(_) => Err(StatusCode::NOT_FOUND),
    }
}
