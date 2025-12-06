use std::fs;
use std::path::Path;
use fastembed::TextEmbedding;
use rayon::prelude::*;


pub fn extract_text(filenames: Vec<&str>) {
    filenames.par_iter()
    .for_each(|file| {
        let bytes = std::fs::read(file).unwrap();
        let mut text = pdf_extract::extract_text_from_mem(&bytes).unwrap();

        //dbg!(&text);

        let cleaned = text.as_mut_str()
            .split('\n')              // Split on \n (discards the \n)
            .map(|s| s.trim())        // Trim each piece
            .filter(|s| !s.is_empty()) // Remove empties
            .collect::<Vec<_>>()
            .join(" "); 

        dbg!(cleaned);
    });
    
}

