
use rayon::prelude::*;
use text_splitter::TextSplitter;
use crate::extract::Page; 

#[derive(Debug, Clone)]
pub struct Chunk {
    pub content: String,
    pub page: u16,
}

pub fn create_chunks(pages: &Vec<String>) -> Vec<Chunk> {
    let max_token_size = 500;

    // Process pages in parallel and collect all chunks
    pages
        .par_iter()  // Fixed: use par_iter() instead of into_par_iter()
        .enumerate()
        .flat_map(|(page_idx, page_content)| {
            chunk_page(page_content, page_idx as u16, max_token_size)
        })
        .collect()
}

pub fn chunk_pages_with_splitter(pages: &[Page], max_chars: usize) -> Vec<Chunk> {
    let splitter = TextSplitter::new(max_chars);

    let mut chunks: Vec<Chunk> = Vec::new();

    for page in pages {
        // The splitter returns an iterator of &str chunks
        let chunk_texts = splitter.chunks(&page.content);

        for chunk_str in chunk_texts {
            chunks.push(Chunk {
                content: chunk_str.to_string(),
                page: page.page_num + 1,
            });
        }
    }

    chunks
}

pub fn chunk_everything(pages: &[Page]) -> Vec<Chunk> {
    let mut chunks = Vec::new();

    let target_size = 200; // characters per chunk
    let min_size = 100;    // minimum before forcing a split

    for page in pages {
        let sentences = page
            .content
            .split(|c| c == '.' || c == '?' || c == '!')
            .filter(|s| !s.trim().is_empty());

        let mut current_chunk = String::new();

        for sentence in sentences {
            let sentence = sentence.trim();

            // If adding this sentence would exceed target size AND we're above minimum
            if current_chunk.len() + sentence.len() > target_size
                && current_chunk.len() >= min_size
            {
                chunks.push(Chunk {
                    content: current_chunk.trim().to_string(),
                    page: page.page_num,
                });
                current_chunk.clear();
            }

            if !current_chunk.is_empty() {
                current_chunk.push(' ');
            }

            current_chunk.push_str(sentence);
            current_chunk.push('.');
        }

        // Push last chunk for this page
        if !current_chunk.is_empty() {
            chunks.push(Chunk {
                content: current_chunk.trim().to_string(),
                page: page.page_num,
            });
        }
    }

    chunks
}


fn chunk_page(content: &String, page_num: u16, max_size: usize) -> Vec<Chunk> {

    let splitter = TextSplitter::new(512);
    let chunks: Vec<Chunk> = splitter.chunks(content)
    .map(|s| Chunk {
        content: s.to_string(),
        page:page_num,
    })
    .collect();
    chunks
}

