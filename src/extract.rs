use pdf_oxide::PdfDocument;
use rayon::prelude::*;


#[derive(Debug)]
pub struct File {
    filename: String,
    pages: Vec<String>,
}

pub fn extract_text(file: &str) -> File {
    let page_count = PdfDocument::open(file).unwrap().page_count().unwrap();

    // Calculate optimal chunk size based on available threads
    let num_threads = rayon::current_num_threads();
    let chunk_size = (page_count / num_threads).max(1);

    // Process pages in parallel chunks
    let page_texts: Vec<String> = (0..page_count)
        .collect::<Vec<_>>()
        .par_chunks(chunk_size)
        .flat_map(|chunk| {
            // Open document once per chunk
            let mut doc = PdfDocument::open(file).unwrap();
            chunk
                .iter()
                .filter_map(|&page_num| doc.extract_text(page_num).ok())
                .collect::<Vec<_>>()
        })
        .collect();

    File {
        filename: (*file).to_string(),
        pages: page_texts,
    }
}

impl File {
    pub fn get_pages(&self) -> &Vec<String> {
        &self.pages
    }
}
