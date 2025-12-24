use clap::{Arg, Parser, command, arg};
use std::io::{self, Write};

pub mod chunk;
pub mod embed;
pub mod extract;
pub mod render;
pub mod qdrant;

/// Simple program to greet a person
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// file path to be stored
    #[arg(short, long)]
    file: String,
}

fn main_() {
    let home = dirs::home_dir().unwrap();
    println!("User home directory: {}", home.display());
    let _ = render::render();
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
    let result = run().await;
    println!("{:?}", result);
}

async fn run() -> Result<(), Box<dyn std::error::Error>> {
    loop {
        let matches = command!()
            .arg(Arg::new("file").short('f').long("file"))
            .arg(Arg::new("search").short('s').long("search"))
            .try_get_matches_from(std::env::args());

        let matches = match matches {
            Ok(m) => m,
            Err(e) => {
                eprintln!("{}", e);
                prompt_for_next()?;
                continue;
            }
        };

        let file_path = matches.get_one::<String>("file");
        let collection_name = matches.get_one::<String>("search");
            
        // Handle file command
        if let Some(file) = file_path {
            let res = extract::extract_text(file);
            let pages = res.get_pages();
            let chunks = chunk::create_chunks(pages);
            let embedded_chunks = embed::get_embeddings(chunks)?;
            let client = qdrant::setup_qdrant(&embedded_chunks, file).await?;
            let response = qdrant::store_embeddings(&client, file, embedded_chunks).await?;
            dbg!(response);
        }

        // Handle search command
        if let Some(collection) = collection_name {
            println!("search paths: {:?}", &collection);
        }

        prompt_for_next()?;
    }
}

fn prompt_for_next() -> Result<(), Box<dyn std::error::Error>> {
    print!("\nEnter command (or Ctrl+C to exit): ");
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;

    Ok(())
}


