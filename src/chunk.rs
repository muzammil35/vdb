use rayon::prelude::*;


#[derive(Debug, Clone)]
pub struct Chunk {
    pub content: String,
    page: u16,
}

pub fn create_chunks(pages: &Vec<String>) -> Vec<Chunk> {
    let max_token_size = 500;

    // Process pages in parallel and collect all chunks
    pages
        .into_par_iter()
        .enumerate()
        .flat_map(|(page_idx, page_content)| {
            let cleaned = clean_text(&page_content);
            chunk_page(cleaned, page_idx as u16, max_token_size)
        })
        .collect()
}

fn chunk_page(content: String, page_num: u16, max_size: usize) -> Vec<Chunk> {
    // Split on natural breaks (double newlines, indicating paragraphs/sections)
    let sections: Vec<&str> = content
        .split("\n\n")
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .collect();

    let mut chunks = Vec::new();
    let mut current_chunk = String::new();
    let mut current_tokens = 0;

    for section in sections {
        let section_tokens = estimate_tokens(section);

        // If section alone exceeds max_size, split it further
        if section_tokens > max_size {
            // Save current chunk if it has content
            if !current_chunk.is_empty() {
                chunks.push(Chunk {
                    content: current_chunk.trim().to_string(),
                    page: page_num,
                });
                current_chunk.clear();
                current_tokens = 0;
            }

            // Split large section by sentences
            let sub_chunks = split_large_section(section, max_size);
            for sub in sub_chunks {
                chunks.push(Chunk {
                    content: sub,
                    page: page_num,
                });
            }
            continue;
        }

        // If adding this section would exceed max_size, save current chunk
        if current_tokens + section_tokens > max_size && !current_chunk.is_empty() {
            chunks.push(Chunk {
                content: current_chunk.trim().to_string(),
                page: page_num,
            });
            current_chunk.clear();
            current_tokens = 0;
        }

        // Add section to current chunk with natural break preserved
        if !current_chunk.is_empty() {
            current_chunk.push_str("\n\n");
        }
        current_chunk.push_str(section);
        current_tokens += section_tokens;
    }

    // Save final chunk
    if !current_chunk.is_empty() {
        chunks.push(Chunk {
            content: current_chunk.trim().to_string(),
            page: page_num,
        });
    }

    chunks
}

// Handle sections that are too large by splitting on sentence boundaries
fn split_large_section(section: &str, max_size: usize) -> Vec<String> {
    let sentences: Vec<&str> = section
        .split(|c| c == '.' || c == '!' || c == '?')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .collect();

    let mut result = Vec::new();
    let mut current = String::new();
    let mut tokens = 0;

    for (i, sentence) in sentences.iter().enumerate() {
        let sentence_tokens = estimate_tokens(sentence);
        let with_punct = format!("{}.", sentence); // Re-add punctuation

        if tokens + sentence_tokens > max_size && !current.is_empty() {
            result.push(current.trim().to_string());
            current.clear();
            tokens = 0;
        }

        if !current.is_empty() {
            current.push(' ');
        }
        current.push_str(&with_punct);
        tokens += sentence_tokens;
    }

    if !current.is_empty() {
        result.push(current.trim().to_string());
    }

    result
}

fn clean_text(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    let mut last_was_space = true;

    for c in text.chars() {
        if c.is_whitespace() {
            if !last_was_space {
                result.push(' ');
                last_was_space = true;
            }
        } else if c.is_ascii() {
            result.push(c);
            last_was_space = false;
        }
        // Skip non-ASCII characters entirely (or handle differently)
    }

    result.trim().to_string()
}

// Simple token estimation (roughly 1 token per 4 characters)
fn estimate_tokens(text: &str) -> usize {
    (text.len() as f32 / 4.0).ceil() as usize
}
