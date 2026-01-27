use pdf_oxide::PdfDocument;
use rayon::prelude::*;


#[derive(Debug)]

pub struct Page {
    pub content: String,
    pub page_num: u16
}
pub struct File {
    filename: String,
    pages: Vec<Page>,
}

pub fn extract_text(file: &str) -> File {
    let page_count = PdfDocument::open(file).unwrap().page_count().unwrap();

    // Calculate optimal chunk size based on available threads
    let num_threads = rayon::current_num_threads();
    let chunk_size = (page_count / num_threads).max(1);

    // Process pages in parallel chunks
    let pages: Vec<Page> = (0..page_count)
        .collect::<Vec<_>>()
        .par_chunks(chunk_size)
        .flat_map(|chunk| {
            // Open document once per chunk
            let mut doc = PdfDocument::open(file).unwrap();
            chunk
                .iter()
                .filter_map(|&page_num| {
                    doc.extract_text(page_num)
                        .ok()
                        .map(|text| Page {
                            content: text,
                            page_num: page_num as u16,
                        })
                })
                .collect::<Vec<Page>>()
                
        })
        .collect();

    File {
        filename: (*file).to_string(),
        pages: pages,
    }
}

impl File {
    pub fn get_pages(&self) -> &Vec<Page> {
        &self.pages
    }
}
