use crate::extract::Page;
use rayon::prelude::*;
use regex::Regex;
use text_splitter::TextSplitter;
use unicode_segmentation::UnicodeSegmentation;
use lopdf::Document;

#[derive(Debug, Clone)]
pub struct Chunk {
    pub content: String,
    pub page: u16,
}

 pub fn extract_and_chunk(pdf_path: &str) -> Result<Vec<Chunk>, Box<dyn std::error::Error>> {
    // Load the PDF
    let doc = Document::load(pdf_path)?;
    
    // Extract text from all pages
    let mut full_text = String::new();
    let pages = doc.get_pages();

    let mut chunks: Vec<Chunk> = Vec::new();

    // Create splitter - overlap is passed to chunks() method, not in constructor
    let splitter = TextSplitter::new(500); // chunk size
    
    for &page_num in pages.keys() {
        if let Ok(text) = doc.extract_text(&[page_num]) {
            let chunk_texts: Vec<_> = splitter.chunks(&text).collect();

            for chunk in chunk_texts {
                chunks.push(Chunk {
                    content: chunk.to_string(),
                    page: (page_num) as u16,
                });
            }
        }
    }

    println!("Created {} chunks", chunks.len());
    for (i, chunk) in chunks.iter().enumerate() {
        println!("\n--- Chunk {} ---\n{}", i + 1, chunk.content);
    }
    
    Ok(chunks)
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

pub fn chunk_per_page(pages: &[Page]) -> Vec<Chunk> {
    let mut return_chunks: Vec<Chunk> = Vec::new();
    for page in pages {
        let chunks = smart_chunk_text(&page.content, 2000, 200, true);
        for chunk in chunks {
            if is_garbage_sentence(&chunk) {
                continue;
            }
            return_chunks.push(Chunk {
                content: (chunk),
                page: (page.page_num),
            });
        }
    }
    return_chunks
}

pub fn remove_section_headers(text: &str) -> String {
    let lines: Vec<&str> = text.lines().collect();
    let mut cleaned_lines = Vec::new();

    // Regex patterns for common section headers
    let section_number_regex = Regex::new(
        r"^[\s]*(\d+\.)+\d*[\s]*[A-Z]", // Matches "3.1 Introduction" or "3.1.2 Details"
    )
    .unwrap();

    let chapter_regex =
        Regex::new(r"^[\s]*(Chapter|Section|Part|Appendix)[\s]+(\d+|[A-Z])").unwrap();

    let simple_header_regex = Regex::new(
        r"^[\s]*\d+\.[\s]*[A-Z][a-z]+", // Matches "3. Introduction"
    )
    .unwrap();

    for line in lines {
        let trimmed = line.trim();

        // Skip empty lines
        if trimmed.is_empty() {
            continue;
        }

        // Check if line is a section header
        let is_header = section_number_regex.is_match(trimmed)
            || chapter_regex.is_match(trimmed)
            || simple_header_regex.is_match(trimmed)
            || is_likely_header(trimmed);

        if !is_header {
            cleaned_lines.push(line);
        }
    }

    cleaned_lines.join("\n")
}

// Additional heuristic check for headers
fn is_likely_header(line: &str) -> bool {
    let trimmed = line.trim();

    // Headers are usually short
    if trimmed.len() > 100 {
        return false;
    }

    // Check for common patterns
    let starts_with_number = trimmed.chars().next().map_or(false, |c| c.is_numeric());
    let has_colon = trimmed.contains(':');
    let word_count = trimmed.split_whitespace().count();

    // Common header patterns:
    // - Starts with number and has few words
    // - All caps (like "INTRODUCTION")
    // - Title case with few words
    if starts_with_number && word_count <= 6 {
        return true;
    }

    if trimmed
        .chars()
        .all(|c| c.is_uppercase() || c.is_whitespace() || c.is_numeric())
        && word_count <= 5
    {
        return true;
    }

    // "3.1: Introduction to Machine Learning" pattern
    if starts_with_number && has_colon && word_count <= 8 {
        return true;
    }

    false
}

fn split_into_sentences(text: &str) -> Vec<String> {
    text.unicode_sentences().map(|s| s.to_string()).collect()
}

/// Clean PDF text for chunking / embeddings.
pub fn clean_pdf_text_robust(text: &str, remove_headers: bool) -> String {
    let mut cleaned = text.to_string();

    // 1️⃣ Optional: remove section headers
    if remove_headers {
        cleaned = remove_section_headers(&cleaned);
    }

    // 2️⃣ Remove TOC / leader lines like ". . . 415 . . . 422"
    let toc_leader_regex = Regex::new(r"(?m)^[\s\d]*([.]\s*){5,}[\s\d]*$").unwrap();
    cleaned = toc_leader_regex.replace_all(&cleaned, "").to_string();

    // 3️⃣ Remove lines that are mostly non-letters
    cleaned = cleaned
        .lines()
        .filter(|line| {
            let letters = line.chars().filter(|c| c.is_alphabetic()).count();
            let total = line.chars().count();
            total == 0 || letters * 4 >= total // keep line if ≥25% letters
        })
        .collect::<Vec<_>>()
        .join("\n");

    // 4️⃣ Fix hyphenated line breaks ("rejec-\nted" → "rejected")
    let hyphen_linebreak_regex = Regex::new(r"(?m)-\n").unwrap();
    cleaned = hyphen_linebreak_regex.replace_all(&cleaned, "").to_string();

    // 5️⃣ Join lines with space (avoid word merges)
    let mut fixed_text = String::new();
    for line in cleaned.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if !fixed_text.is_empty() && !fixed_text.ends_with(|c: char| ".!?".contains(c)) {
            fixed_text.push(' ');
        }
        fixed_text.push_str(line);
    }
    cleaned = fixed_text;

    // 6️⃣ Remove control characters except newline/tab
    cleaned = cleaned
        .chars()
        .filter(|c| !c.is_control() || *c == '\n' || *c == '\t')
        .collect();

    // 7️⃣ Normalize whitespace
    let whitespace_regex = Regex::new(r"\s+").unwrap();
    cleaned = whitespace_regex.replace_all(&cleaned, " ").to_string();

    // 8️⃣ Replace common PDF ligatures
    cleaned = cleaned
        .replace("ﬁ", "fi")
        .replace("ﬂ", "fl")
        .replace("ﬀ", "ff")
        .replace("ﬃ", "ffi")
        .replace("ﬄ", "ffl")
        .replace("œ", "oe")
        .replace("æ", "ae")
        .replace("\u{FEFF}", "")
        .replace("\u{200B}", "")
        .replace("\u{00A0}", " ");

    // 9️⃣ Remove repeated punctuation artifacts
    let punct_regex = Regex::new(r"([.,!?;:]){3,}").unwrap();
    cleaned = punct_regex.replace_all(&cleaned, "$1").to_string();

    cleaned.trim().to_string()
}

pub fn clean_pdf_text_advanced(text: &str, remove_headers: bool) -> String {
    let mut cleaned = text.to_string();

    // Remove headers first if requested
    if remove_headers {
        cleaned = remove_section_headers(&cleaned);
    }

    let toc_leader_regex = Regex::new(r"(?m)^[\s\d]*([.]\s*){5,}[\s\d]*$").unwrap();

    //remove table of contents leader lines
    cleaned = toc_leader_regex.replace_all(&cleaned, "").to_string();

    //remove mostly non-letter lines
    cleaned = cleaned
        .lines()
        .filter(|line| {
            let letters = line.chars().filter(|c| c.is_alphabetic()).count();
            let total = line.chars().count();

            // Keep line if it has enough real text
            total == 0 || letters * 4 >= total
        })
        .collect::<Vec<_>>()
        .join("\n");

    // Remove common PDF artifacts
    cleaned = cleaned
        .chars()
        .filter(|c| !c.is_control() || *c == '\n' || *c == '\t')
        .collect();

    // Normalize whitespace
    let whitespace_regex = Regex::new(r"\s+").unwrap();
    cleaned = whitespace_regex.replace_all(&cleaned, " ").to_string();

    // Remove common PDF ligatures and special chars
    cleaned = cleaned
        .replace("ﬁ", "fi")
        .replace("ﬂ", "fl")
        .replace("ﬀ", "ff")
        .replace("ﬃ", "ffi")
        .replace("ﬄ", "ffl")
        .replace("\u{FEFF}", "")
        .replace("\u{200B}", "")
        .replace("\u{00A0}", " ");

    // Remove repeated punctuation artifacts
    let punct_regex = Regex::new(r"([.,!?;:]){3,}").unwrap();
    cleaned = punct_regex.replace_all(&cleaned, "$1").to_string();

    cleaned.trim().to_string()
}

fn is_garbage_sentence(s: &str) -> bool {
    let letters = s.chars().filter(|c| c.is_alphabetic()).count();
    let digits = s.chars().filter(|c| c.is_numeric()).count();
    let dots = s.matches('.').count();

    dots > 10 && letters < 5 && digits > 0
}

// Updated chunking function
pub fn smart_chunk_text(
    text: &str,
    chunk_size: usize,
    overlap: usize,
    remove_headers: bool,
) -> Vec<String> {
    let cleaned = clean_pdf_text_robust(text, remove_headers);
    let sentences = split_into_sentences(&cleaned);

    let mut chunks = Vec::new();
    let mut current_chunk = String::new();
    let mut sentence_buffer = Vec::new();

    for sentence in sentences {
        // Skip if sentence looks like a standalone header that slipped through
        if is_likely_header(&sentence) {
            continue;
        }

        sentence_buffer.push(sentence.clone());
        current_chunk.push_str(&sentence);
        current_chunk.push(' ');

        if current_chunk.len() >= chunk_size {
            chunks.push(current_chunk.trim().to_string());

            current_chunk = sentence_buffer
                .iter()
                .rev()
                .take(2)
                .rev()
                .cloned()
                .collect::<Vec<_>>()
                .join(" ");

            sentence_buffer.clear();
        }
    }

    if !current_chunk.is_empty() {
        chunks.push(current_chunk.trim().to_string());
    }

    chunks
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chunks_textbook_like_pages() {
        let pages = vec![
            Page {
                page_num: 1,
                content: "Chapter 1: Introduction\n\n\
                Machine learning is a field of computer science that \
                gives computers the ability to learn without being \
                explicitly programmed. This chapter introduces basic \
                terminology and concepts used throughout the book."
                    .repeat(20),
            },
            Page {
                page_num: 2,
                content: "Chapter 2: Linear Models\n\n\
                Linear regression is one of the simplest supervised \
                learning algorithms. Despite its simplicity, it forms \
                the basis for more complex models."
                    .repeat(20),
            },
        ];

        let chunks = chunk_per_page(&pages);

        for chunk in &chunks {
            print!("chunk start");
            println!("{:?}", chunk.content);
            println!("");
        }

        // sanity checks
        assert!(!chunks.is_empty());

        // every chunk should be associated with a valid page
        for chunk in &chunks {
            assert!(chunk.page == 1 || chunk.page == 2);
            assert!(!chunk.content.is_empty());
        }
    }
}
