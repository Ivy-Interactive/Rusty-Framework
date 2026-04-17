use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

fn main() {
    let docs_dir = Path::new("docs");
    let out_dir = Path::new("src/generated");

    println!("cargo:rerun-if-changed=docs");

    // Clean generated directory
    if out_dir.exists() {
        fs::remove_dir_all(out_dir).expect("failed to clean generated dir");
    }
    fs::create_dir_all(out_dir).expect("failed to create generated dir");

    let mut sections: BTreeMap<String, Section> = BTreeMap::new();

    // Walk top-level directories in docs/
    let mut entries: Vec<_> = fs::read_dir(docs_dir)
        .expect("failed to read docs dir")
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().map(|t| t.is_dir()).unwrap_or(false))
        .collect();
    entries.sort_by_key(|e| e.file_name());

    for section_entry in &entries {
        let section_dir = section_entry.path();
        let section_name = section_entry.file_name().to_string_lossy().to_string();
        let (order, clean_name) = parse_prefix(&section_name);
        let module_name = clean_name.to_lowercase();

        let mut pages = Vec::new();

        let mut page_entries: Vec<_> = fs::read_dir(&section_dir)
            .expect("failed to read section dir")
            .filter_map(|e| e.ok())
            .filter(|e| {
                let name = e.file_name().to_string_lossy().to_string();
                name.ends_with(".md") && name != "_index.md"
            })
            .collect();
        page_entries.sort_by_key(|e| e.file_name());

        for page_entry in &page_entries {
            let file_name = page_entry.file_name().to_string_lossy().to_string();
            let (page_order, page_clean) = parse_prefix(file_name.trim_end_matches(".md"));
            let page_module = page_clean.to_lowercase();

            pages.push(Page {
                order: page_order,
                module_name: page_module,
                display_name: to_title_case(&page_clean),
                relative_path: page_entry
                    .path()
                    .strip_prefix(".")
                    .unwrap_or(&page_entry.path())
                    .to_path_buf(),
            });
        }

        // Read _index.md for section title
        let index_path = section_dir.join("_index.md");
        let section_title = if index_path.exists() {
            let content = fs::read_to_string(&index_path).unwrap_or_default();
            extract_title(&content).unwrap_or_else(|| to_title_case(&clean_name))
        } else {
            to_title_case(&clean_name)
        };

        sections.insert(
            module_name.clone(),
            Section {
                order,
                module_name,
                display_name: section_title,
                pages,
            },
        );
    }

    // Generate a module file per section
    for section in sections.values() {
        let section_dir = out_dir.join(&section.module_name);
        fs::create_dir_all(&section_dir).expect("failed to create section dir");

        let mut section_mod = String::new();
        for page in &section.pages {
            generate_page_module(&section_dir, section, page);
            section_mod.push_str(&format!("pub mod {};\n", page.module_name));
        }

        fs::write(section_dir.join("mod.rs"), section_mod).expect("failed to write section mod.rs");
    }

    // Generate top-level mod.rs with page registry
    let mut mod_rs = String::new();

    for section in sections.values() {
        mod_rs.push_str(&format!("pub mod {};\n", section.module_name));
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

    for section in sections.values() {
        for page in &section.pages {
            let struct_name = to_pascal_case(&page.module_name);
            mod_rs.push_str(&format!(
                "        DocPage {{ section: \"{}\", title: \"{}\", id: \"{}_{}\", view_factory: || Box::new({}::{}::{}Page) }},\n",
                section.display_name,
                page.display_name,
                section.module_name,
                page.module_name,
                section.module_name,
                page.module_name,
                struct_name,
            ));
        }
    }

    mod_rs.push_str("    ]\n");
    mod_rs.push_str("}\n");

    fs::write(out_dir.join("mod.rs"), mod_rs).expect("failed to write generated/mod.rs");
}

fn generate_page_module(section_dir: &Path, section: &Section, page: &Page) {
    let struct_name = to_pascal_case(&page.module_name);

    // Compute the relative path from generated source file to the docs markdown
    let md_path = format!(
        "../../../docs/{}/{}.md",
        find_original_dir_name("docs", &section.module_name),
        find_original_file_name(
            &format!(
                "docs/{}",
                find_original_dir_name("docs", &section.module_name)
            ),
            &page.module_name,
        ),
    );

    let source = format!(
        r#"use rusty::prelude::*;

pub struct {struct_name}Page;

impl View for {struct_name}Page {{
    fn build(&self, _ctx: &mut BuildContext) -> Element {{
        Layout::vertical()
            .padding(24.0)
            .gap(16.0)
            .child(TextBlock::h1("{title}"))
            .child(TextBlock::markdown(include_str!("{md_path}")))
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

fn find_original_dir_name(base: &str, module_name: &str) -> String {
    let base_path = Path::new(base);
    if let Ok(entries) = fs::read_dir(base_path) {
        for entry in entries.flatten() {
            if entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                let name = entry.file_name().to_string_lossy().to_string();
                let (_, clean) = parse_prefix(&name);
                if clean.to_lowercase() == module_name {
                    return name;
                }
            }
        }
    }
    module_name.to_string()
}

fn find_original_file_name(dir: &str, module_name: &str) -> String {
    let dir_path = Path::new(dir);
    if let Ok(entries) = fs::read_dir(dir_path) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if name.ends_with(".md") && name != "_index.md" {
                let stem = name.trim_end_matches(".md");
                let (_, clean) = parse_prefix(stem);
                if clean.to_lowercase() == module_name {
                    return name.trim_end_matches(".md").to_string();
                }
            }
        }
    }
    module_name.to_string()
}

struct Section {
    #[allow(dead_code)]
    order: u32,
    module_name: String,
    display_name: String,
    pages: Vec<Page>,
}

struct Page {
    #[allow(dead_code)]
    order: u32,
    module_name: String,
    display_name: String,
    #[allow(dead_code)]
    relative_path: PathBuf,
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
