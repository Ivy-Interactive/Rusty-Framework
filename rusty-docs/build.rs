use std::fs;
use std::path::Path;

fn main() {
    let docs_dir = Path::new("docs");
    let out_dir = Path::new("src/generated");

    println!("cargo:rerun-if-changed=docs");

    // Clean generated directory
    if out_dir.exists() {
        fs::remove_dir_all(out_dir).expect("failed to clean generated dir");
    }
    fs::create_dir_all(out_dir).expect("failed to create generated dir");

    let mut sections: Vec<Section> = Vec::new();

    // Walk top-level directories in docs/
    let mut entries: Vec<_> = fs::read_dir(docs_dir)
        .expect("failed to read docs dir")
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().map(|t| t.is_dir()).unwrap_or(false))
        .collect();
    entries.sort_by_key(|e| e.file_name());

    for section_entry in &entries {
        let section_dir = section_entry.path();
        let dir_name = section_entry.file_name().to_string_lossy().to_string();
        let (order, clean_name) = parse_prefix(&dir_name);
        let module_name = clean_name.to_lowercase();

        let mut pages = Vec::new();

        let page_entries: Vec<_> = fs::read_dir(&section_dir)
            .expect("failed to read section dir")
            .filter_map(|e| e.ok())
            .filter(|e| {
                let name = e.file_name().to_string_lossy().to_string();
                name.ends_with(".md") && name != "_index.md"
            })
            .collect();

        for page_entry in &page_entries {
            let file_stem = page_entry.file_name().to_string_lossy().to_string();
            let file_stem = file_stem.trim_end_matches(".md").to_string();
            let (page_order, page_clean) = parse_prefix(&file_stem);

            pages.push(Page {
                order: page_order,
                module_name: page_clean.to_lowercase(),
                display_name: to_title_case(&page_clean),
                file_stem,
            });
        }

        // Author order comes from the numeric filename prefix, not the alphabet.
        pages.sort_by(|a, b| (a.order, &a.module_name).cmp(&(b.order, &b.module_name)));

        // Read _index.md for section title
        let index_path = section_dir.join("_index.md");
        let section_title = if index_path.exists() {
            let content = fs::read_to_string(&index_path).unwrap_or_default();
            extract_title(&content).unwrap_or_else(|| to_title_case(&clean_name))
        } else {
            to_title_case(&clean_name)
        };

        sections.push(Section {
            order,
            dir_name,
            module_name,
            display_name: section_title,
            pages,
        });
    }

    // Author order comes from the numeric directory prefix, not the alphabet.
    sections.sort_by(|a, b| (a.order, &a.module_name).cmp(&(b.order, &b.module_name)));

    // Generate a module file per section
    for section in &sections {
        let section_dir = out_dir.join(&section.module_name);
        fs::create_dir_all(&section_dir).expect("failed to create section dir");

        let mut page_modules = Vec::new();
        for page in &section.pages {
            generate_page_module(&section_dir, section, page);
            page_modules.push(page.module_name.clone());
        }

        // rustfmt sorts `pub mod` declarations alphabetically, so emit them that way.
        // Page *ordering* for the sidebar comes from the registry in mod.rs, not from here.
        page_modules.sort();
        let section_mod: String = page_modules
            .iter()
            .map(|m| format!("pub mod {};\n", m))
            .collect();

        fs::write(section_dir.join("mod.rs"), section_mod).expect("failed to write section mod.rs");
    }

    // Generate top-level mod.rs with page registry
    let mut mod_rs = String::new();

    // Alphabetical, to match how rustfmt sorts `pub mod` declarations.
    let mut section_modules: Vec<&str> = sections.iter().map(|s| s.module_name.as_str()).collect();
    section_modules.sort_unstable();
    for module_name in &section_modules {
        mod_rs.push_str(&format!("pub mod {};\n", module_name));
    }

    mod_rs.push_str("\nuse rusty::prelude::*;\n\n");
    mod_rs.push_str("#[allow(dead_code)]\n");
    mod_rs.push_str("pub struct DocPage {\n");
    mod_rs.push_str("    pub section: &'static str,\n");
    mod_rs.push_str("    pub title: &'static str,\n");
    mod_rs.push_str("    pub id: &'static str,\n");
    mod_rs.push_str("    pub view_factory: fn() -> Box<dyn View>,\n");
    mod_rs.push_str("}\n\n");

    mod_rs.push_str("pub fn all_pages() -> Vec<DocPage> {\n");
    mod_rs.push_str("    vec![\n");

    for section in &sections {
        for page in &section.pages {
            let struct_name = to_pascal_case(&page.module_name);
            // One field per line with a trailing comma — the shape rustfmt produces
            // for a struct literal this wide.
            mod_rs.push_str(&format!(
                r#"        DocPage {{
            section: "{section}",
            title: "{title}",
            id: "{section_module}_{page_module}",
            view_factory: || Box::new({section_module}::{page_module}::{struct_name}Page),
        }},
"#,
                section = section.display_name,
                title = page.display_name,
                section_module = section.module_name,
                page_module = page.module_name,
                struct_name = struct_name,
            ));
        }
    }

    mod_rs.push_str("    ]\n");
    mod_rs.push_str("}\n");

    fs::write(out_dir.join("mod.rs"), mod_rs).expect("failed to write generated/mod.rs");
}

fn generate_page_module(section_dir: &Path, section: &Section, page: &Page) {
    let struct_name = to_pascal_case(&page.module_name);

    // Relative path from the generated source file back to the docs markdown.
    // Both components come from the walk, which already saw the real names.
    let md_path = format!("../../../docs/{}/{}.md", section.dir_name, page.file_stem);

    let source = format!(
        r#"use rusty::prelude::*;

pub struct {struct_name}Page;

impl View for {struct_name}Page {{
    fn build(&self, _ctx: &mut BuildContext) -> Element {{
        Layout::vertical()
            .padding(24.0)
            .gap(16.0)
            .child(TextBlock::h1("{title}"))
            .child(TextBlock::markdown(include_str!(
                "{md_path}"
            )))
            .into()
    }}
}}
"#,
        struct_name = struct_name,
        title = page.display_name,
        md_path = md_path,
    );

    fs::write(section_dir.join(format!("{}.rs", page.module_name)), source)
        .expect("failed to write page module");
}

/// A top-level docs directory, e.g. `docs/03_widgets`.
struct Section {
    /// Numeric filename prefix — drives sidebar order.
    order: u32,
    /// Directory name as it appears on disk, prefix included.
    dir_name: String,
    module_name: String,
    display_name: String,
    pages: Vec<Page>,
}

/// A single markdown page within a section, e.g. `docs/03_widgets/01_button.md`.
struct Page {
    /// Numeric filename prefix — drives sidebar order.
    order: u32,
    module_name: String,
    display_name: String,
    /// File name without the `.md` extension, prefix included.
    file_stem: String,
}

fn parse_prefix(name: &str) -> (u32, String) {
    if let Some(pos) = name.find('_') {
        if let Ok(num) = name[..pos].parse::<u32>() {
            return (num, name[pos + 1..].to_string());
        }
    }
    (0, name.to_string())
}

fn to_title_case(s: &str) -> String {
    s.split('_')
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                Some(c) => c.to_uppercase().to_string() + &chars.as_str().to_lowercase(),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn to_pascal_case(s: &str) -> String {
    s.split('_')
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                Some(c) => c.to_uppercase().to_string() + &chars.as_str().to_lowercase(),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join("")
}

fn extract_title(content: &str) -> Option<String> {
    for line in content.lines() {
        let trimmed = line.trim();
        if let Some(title) = trimmed.strip_prefix("# ") {
            return Some(title.trim().to_string());
        }
    }
    None
}
