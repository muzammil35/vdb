
use eframe::egui;
use pdfium_render::prelude::*;
use std::path::PathBuf;

pub fn render() -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([800.0, 600.0]),
        ..Default::default()
    };

    
    eframe::run_native(
        "PDF Viewer",
        options,
        Box::new(|_cc| Ok(Box::new(PdfViewerApp::default()))),
    )
}

struct PdfViewerApp {
    pdf_path: String,
    document: Option<PdfDocument<'static>>,
    current_page: usize,
    total_pages: usize,
    rendered_image: Option<egui::ColorImage>,
    error_message: Option<String>,
}

impl Default for PdfViewerApp {
    fn default() -> Self {
        Self {
            pdf_path: String::new(),
            document: None,
            current_page: 0,
            total_pages: 0,
            rendered_image: None,
            error_message: None,
        }
    }
}

impl PdfViewerApp {
    fn load_pdf(&mut self, pdfium: Pdfium) {
        self.error_message = None;

        
        // Load the PDF
        let document = match pdfium.load_pdf_from_file(&self.pdf_path, None) {
            Ok(doc) => doc,
            Err(e) => {
                self.error_message = Some(format!("Failed to load PDF: {}", e));
                return;
            }
        };
        
        self.total_pages = document.pages().len() as usize;
        self.current_page = 0;
        
        // Store pdfium and document (need unsafe due to lifetime)
        // In production, use a better approach with Rc/Arc
        //self.pdfium = Some(&pdfium);
        self.document = Some(unsafe { std::mem::transmute(document) });
        
        self.render_current_page();
    }
    
    fn render_current_page(&mut self) {
        if let Some(ref document) = self.document {
            if let Ok(page) = document.pages().get(self.current_page as u16) {
                // Render at 2x resolution for better quality
                let render_config = PdfRenderConfig::new()
                    .set_target_width(1600)
                    .rotate_if_landscape(PdfPageRenderRotation::Degrees90, true);
                
                match page.render_with_config(&render_config) {
                    Ok(bitmap) => {
                        // Convert bitmap to egui ColorImage
                        let width = bitmap.width() as usize;
                        let height = bitmap.height() as usize;
                        
                        // Get RGBA bytes
                        let bytes = bitmap.as_bytes();
                        let mut pixels = Vec::with_capacity(width * height);
                        
                        for chunk in bytes.chunks(4) {
                            if chunk.len() == 4 {
                                pixels.push(egui::Color32::from_rgba_unmultiplied(
                                    chunk[0], chunk[1], chunk[2], chunk[3]
                                ));
                            }
                        }
                        
                        self.rendered_image = Some(egui::ColorImage {
                            size: [width, height],
                            source_size: egui::Vec2::new(width as f32, height as f32),
                            pixels,
                        });
                    }
                    Err(e) => {
                        self.error_message = Some(format!("Failed to render page: {}", e));
                    }
                }
            }
        }
    }
    
    fn next_page(&mut self) {
        if self.current_page + 1 < self.total_pages {
            self.current_page += 1;
            self.render_current_page();
        }
    }
    
    fn previous_page(&mut self) {
        if self.current_page > 0 {
            self.current_page -= 1;
            self.render_current_page();
        }
    }
    
    fn go_to_page(&mut self, page: usize) {
        if page < self.total_pages {
            self.current_page = page;
            self.render_current_page();
        }
    }
}

impl eframe::App for PdfViewerApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label("PDF Path:");
                ui.text_edit_singleline(&mut self.pdf_path);
                
                if ui.button("Load PDF").clicked() {
                    let pdfium = Pdfium::default();
                    self.load_pdf(pdfium);
                }
            });
            
            if let Some(ref error) = self.error_message {
                ui.colored_label(egui::Color32::RED, error);
            }
        });
        
        egui::TopBottomPanel::bottom("bottom_panel").show(ctx, |ui| {
            if self.document.is_some() {
                ui.horizontal(|ui| {
                    if ui.button("⬅ Previous").clicked() {
                        self.previous_page();
                    }
                    
                    ui.label(format!("Page {} of {}", self.current_page + 1, self.total_pages));
                    
                    if ui.button("Next ➡").clicked() {
                        self.next_page();
                    }
                    
                    ui.separator();
                    
                    ui.label("Go to page:");
                    let mut page_input = (self.current_page + 1).to_string();
                    if ui.text_edit_singleline(&mut page_input).lost_focus() 
                        && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                        if let Ok(page_num) = page_input.parse::<usize>() {
                            if page_num > 0 {
                                self.go_to_page(page_num - 1);
                            }
                        }
                    }
                });
            }
        });
        
        egui::CentralPanel::default().show(ctx, |ui| {
            if let Some(ref image) = self.rendered_image {
                egui::ScrollArea::both().show(ui, |ui| {
                    let texture = ui.ctx().load_texture(
                        "pdf_page",
                        image.clone(),
                        Default::default()
                    );
                    
                    ui.image(&texture);
                });
            } else if self.document.is_none() {
                ui.centered_and_justified(|ui| {
                    ui.label("Load a PDF to get started");
                });
            }
        });
    }
}