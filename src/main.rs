// Prevent console window in addition to Slint window in Windows release builds when, e.g., starting the app via file manager. Ignored on other platforms.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::{error::Error, fs::File, io::Write, path::{self, Path, PathBuf}};

slint::include_modules!();

fn main() -> Result<(), Box<dyn Error>> {
    let ui = AppWindow::new()?;
    let main_dialogue_box = ErrorWindow::new()?;

    ui.on_generate_page({
        let ui_handle = ui.as_weak();
        let dialogue_handle = main_dialogue_box.as_weak();
        move || {
            let ui = ui_handle.unwrap();
            let dialogue_window = dialogue_handle.unwrap();

            let title: String = (&ui).get_blog_title().to_string();
            let content: String = (&ui).get_blog_content().to_string();

            match blog_to_html(title, content) {
                Ok(p) => {
                    (&dialogue_window).set_error_window_content(format!("Wrote site file to {p}").into());
                },
                Err(e) => {
                    (&dialogue_window).set_error_window_content(e.into());
                },
            }
            let _ = (&dialogue_window).show();
        }
    });

    main_dialogue_box.on_ok_clicked({
        let handle = main_dialogue_box.as_weak();
        move || {
            let window = handle.unwrap();
            let _ = window.hide();
        }
    });

    ui.run()?;

    Ok(())
}

fn blog_to_html(title: String, content: String) -> Result<String, String> {
    let htmldoc: String = String::from(
r#"<!DOCTYPE html>
<html lang="en">
    <head>
        <meta charset="UTF-8" />
        <title>james "james" pham</title>
        <meta name="viewport" content="width=device-width,initial-scale=1" />
        <meta name="description" content="" />
        <link rel="stylesheet" type="text/css" href="barebones.css" />
        <link rel="icon" href="favicon.png">
        <link rel="preconnect" href="https://fonts.googleapis.com">
        <link rel="preconnect" href="https://fonts.gstatic.com" crossorigin>
        <link href="https://fonts.googleapis.com/css2?family=Lexend:wght@100..900&display=swap" rel="stylesheet">
        <div class="grid-container full">
            <nav class="navbar" id="navbar">
                <ul class="navbar-list">
                <li class="navbar-item"><a class="navbar-link" href="index.html">Home</a></li>
                <li class="navbar-item"><a class="navbar-link" href="tech.html">Technical</a></li>
                <li class="navbar-item"><a class="navbar-link" href="anime.html">Anime</a></li>
                <li class="navbar-item"><a class="navbar-link" href="life.html">Life</a></li>
                <li class="navbar-item right"><a class="navbar-link right" href="resume.html">Resume</a></li>
                <li class="navbar-item right"><a class="navbar-link right" href="contact.html">Contact</a></li>
                </ul>
            </nav>
        </div>
    </head>
    <body>
        <div class="grid-container full full-left">
        "#
    );
    let htmlfoot: String = String::from(
        r#"
        </div>
    </body>
</html>
        "#
    );

    let mut doodoo: String = String::from("<div class=\"data-entry\">\r\n");

    doodoo.push_str(&(format!("\t\t\t<h2>{title}</h2>\r\n")));
    doodoo.push_str(&(format!("\t\t\t<h5>{title}</h5>\r\n")));
    for line in content.lines() {
        if !line.is_empty() {
            doodoo.push_str(&(format!("\t\t\t<p>{line}</p>\r\n")))
        }
    }

    let path: PathBuf = Path::new("/Users/jpham/equinox/equinox/output.html").to_path_buf();
    let mut file = match File::create(&path) {
        Ok(f) => f,
        Err(_) => return Err("Failed to create file".to_string()),
    };

    match file.write_all(format!("{htmldoc}{doodoo}{htmlfoot}").as_bytes()) {
        Ok(_) => Ok(path.into_os_string().into_string().unwrap()),
        Err(_) => Err("Failed to compose html".to_string())
    }
}