// Prevent console window in addition to Slint window in Windows release builds when, e.g., starting the app via file manager. Ignored on other platforms.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use maud::{html, DOCTYPE};
use rfd::FileDialog;
use std::{
    error::Error,
    fmt, fs,
    path::{Path, PathBuf},
};

slint::include_modules!();

/// Represents a category containing indexed folder names from a directory
#[derive(Debug, Clone)]
pub struct SiteStructure {
    /// The root directory path
    pub root_path: PathBuf,
    /// The root index file if it exists
    pub index_path: Option<PathBuf>,
    /// Names of all immediate subdirectories (folders) in the root, excluding "assets"
    pub categories: Vec<String>,
    /// Associated assets folder path if it exists
    pub assets_path: Option<PathBuf>,
}

impl fmt::Display for SiteStructure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let index_status = match &self.index_path {
            Some(p) => p.display().to_string(),
            None => "not found".to_string(),
        };
        let assets_status = match &self.assets_path {
            Some(p) => p.display().to_string(),
            None => "not found".to_string(),
        };

        write!(
            f,
            "SiteStructure:\n  Root: {}\n  Index: {}\n  Assets: {}\n  Categories: [{}]",
            self.root_path.display(),
            index_status,
            assets_status,
            self.categories.join(", ")
        )
    }
}

fn parse_site_structure(path: &Path) -> Result<SiteStructure, String> {
    if !path.is_dir() {
        return Err(format!("{} is not a directory", path.display()));
    }

    let entries = fs::read_dir(path).map_err(|e| format!("Failed to read directory: {}", e))?;

    let mut index_path: Option<PathBuf> = None;
    let mut assets_path: Option<PathBuf> = None;
    let mut categories: Vec<String> = Vec::new();

    for entry in entries.filter_map(|e| e.ok()) {
        let entry_path = entry.path();
        let file_name = entry.file_name();
        let name_str = file_name.to_string_lossy();

        if entry_path.is_file() && name_str == "index.html" {
            index_path = Some(entry_path);
        } else if entry_path.is_dir() {
            if name_str == "assets" {
                assets_path = Some(entry_path);
            } else {
                // Skip hidden directories (starting with .)
                if !name_str.starts_with('.') {
                    categories.push(name_str.into_owned());
                }
            }
        }
    }

    Ok(SiteStructure {
        root_path: path.to_path_buf(),
        index_path,
        categories,
        assets_path,
    })
}

fn main() -> Result<(), Box<dyn Error>> {
    let ui = AppWindow::new()?;

    ui.on_parse_directory({
        let ui_handle = ui.as_weak();
        move || {
            let dialog = FileDialog::new().set_title("Select a directory");

            if let Some(path) = dialog.pick_folder() {
                match parse_site_structure(&path) {
                    Ok(structure) => {
                        println!("{}", structure);

                        let project_name = structure
                            .root_path
                            .file_name()
                            .and_then(|name| name.to_str())
                            .map(|s| s.to_string())
                            .unwrap_or_else(|| structure.root_path.display().to_string());

                        if let Some(ui) = ui_handle.upgrade() {
                            ui.set_selected_project(project_name.into());
                            ui.set_show_selected_project(true);
                        }

                        // Check if index.html is missing and prompt to create
                        if structure.index_path.is_none() {
                            if let Some(ui) = ui_handle.upgrade() {
                                ui.set_show_create_index_prompt(true);
                            }
                        }
                    }
                    Err(e) => eprintln!("Error: {}", e),
                }
            }
        }
    });

    ui.on_create_index_file({
        let ui_handle = ui.as_weak();
        move || {
            println!("Creating index.html...");
            if let Some(ui) = ui_handle.upgrade() {
                ui.set_show_create_index_prompt(false);
            }
        }
    });

    ui.on_cancel_create_index({
        let ui_handle = ui.as_weak();
        move || {
            if let Some(ui) = ui_handle.upgrade() {
                ui.set_show_create_index_prompt(false);
            }
        }
    });

    ui.on_generate_page({
        let ui_handle = ui.as_weak();
        move || {
            if let Some(ui) = ui_handle.upgrade() {
                let title = ui.get_blog_title().to_string();
                let content = ui.get_blog_content().to_string();
                match blog_to_html(title, content) {
                    Ok(result) => println!("Generated: {}", result),
                    Err(e) => eprintln!("Error: {}", e),
                }
            }
        }
    });

    ui.run()?;

    Ok(())
}

fn blog_to_html(title: String, content: String) -> Result<String, String> {
    let htmldoc = html! {
        (DOCTYPE)
        html lang="en" {
            head {
                meta charset="UTF-8";
                title { (format!("jjp | {}", title)) }
            }
            body {
                h1 { (title) }
                div { (content) }
            }
        }
    };
    println!("{}", htmldoc.into_string());
    Ok("Generated HTML".to_string())
}
