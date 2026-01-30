// Prevent console window in addition to Slint window in Windows release builds when, e.g., starting the app via file manager. Ignored on other platforms.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use html5ever::tendril::TendrilSink;
use html5ever::{parse_document, serialize};
use markup5ever_rcdom::{Handle, NodeData, RcDom, SerializableHandle};
use rfd::FileDialog;
use slint::{ModelRc, SharedString, VecModel};
use std::cell::RefCell;
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

    // Shared state to store the current project root path
    let current_project_root: std::rc::Rc<RefCell<Option<PathBuf>>> =
        std::rc::Rc::new(RefCell::new(None));

    ui.on_parse_directory({
        let ui_handle = ui.as_weak();
        let project_root = current_project_root.clone();
        move || {
            let dialog = FileDialog::new().set_title("Select a directory");

            if let Some(path) = dialog.pick_folder() {
                match parse_site_structure(&path) {
                    Ok(structure) => {
                        println!("{}", structure);

                        // Store the project root path
                        *project_root.borrow_mut() = Some(structure.root_path.clone());

                        let project_name = structure
                            .root_path
                            .file_name()
                            .and_then(|name| name.to_str())
                            .map(|s| s.to_string())
                            .unwrap_or_else(|| structure.root_path.display().to_string());

                        // Convert categories to Slint model
                        let categories: Vec<SharedString> = structure
                            .categories
                            .iter()
                            .map(|s| SharedString::from(s.as_str()))
                            .collect();
                        let categories_model = ModelRc::new(VecModel::from(categories));

                        if let Some(ui) = ui_handle.upgrade() {
                            ui.set_selected_project(project_name.into());
                            ui.set_show_selected_project(true);
                            ui.set_categories(categories_model);
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
        let project_root = current_project_root.clone();
        move || {
            if let Some(ui) = ui_handle.upgrade() {
                let title = ui.get_blog_title().to_string();
                let content = ui.get_blog_content().to_string();

                let root = project_root.borrow();
                if let Some(ref root_path) = *root {
                    let template_path = root_path.join("default.html");
                    match blog_to_html(&template_path, title, content) {
                        Ok(result) => println!("Generated HTML ({} bytes)", result.len()),
                        Err(e) => eprintln!("Error: {}", e),
                    }
                } else {
                    eprintln!("Error: No project selected");
                }
            }
        }
    });

    ui.on_create_category(|| {
        // TODO: pop up textbox to create category
        println!("Create category clicked");
    });

    ui.run()?;

    Ok(())
}

fn load_template(template_path: &Path) -> Result<RcDom, String> {
    let html_content =
        fs::read_to_string(template_path).map_err(|e| format!("Failed to read template: {}", e))?;

    let dom = parse_document(RcDom::default(), Default::default())
        .from_utf8()
        .read_from(&mut html_content.as_bytes())
        .map_err(|e| format!("Failed to parse template: {}", e))?;

    Ok(dom)
}

fn replace_placeholders(handle: &Handle, title: &str, content: &str) {
    let node = handle;

    if let NodeData::Text { contents } = &node.data {
        let text = contents.borrow().to_string();
        if text.contains("{{title}}") || text.contains("{{content}}") {
            let new_text = text
                .replace("{{title}}", title)
                .replace("{{content}}", content);
            *contents.borrow_mut() = new_text.into();
        }
    }

    for child in node.children.borrow().iter() {
        replace_placeholders(child, title, content);
    }
}

fn serialize_dom(dom: &RcDom) -> Result<String, String> {
    let document: SerializableHandle = dom.document.clone().into();
    let mut bytes = Vec::new();

    serialize(&mut bytes, &document, Default::default())
        .map_err(|e| format!("Failed to serialize HTML: {}", e))?;

    String::from_utf8(bytes).map_err(|e| format!("Failed to convert to UTF-8: {}", e))
}

fn blog_to_html(template_path: &Path, title: String, content: String) -> Result<String, String> {
    let dom = load_template(template_path)?;
    replace_placeholders(&dom.document, &title, &content);
    let html_output = serialize_dom(&dom)?;
    println!("{}", html_output);
    Ok(html_output)
}
