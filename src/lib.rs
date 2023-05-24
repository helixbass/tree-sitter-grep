use walkdir::{DirEntry, WalkDir};

mod macros;
mod treesitter;

use treesitter::{get_query, get_results};

pub fn run() {
    let query_source = r#"
        (field_declaration
          type: (type_identifier) @type
          (#eq? @type "String")
        )
    "#;
    let query = get_query(query_source);
    enumerate_project_files()
        .flat_map(|project_file_dir_entry| get_results(&query, project_file_dir_entry.path(), 0))
        .for_each(|result| {
            println!("{}", result.format());
        });
}

fn enumerate_project_files() -> impl Iterator<Item = DirEntry> {
    WalkDir::new(".")
        .into_iter()
        .filter_map(|entry| entry.ok())
        .filter(|entry| {
            let extension = entry.path().extension();
            if extension.is_none() {
                return false;
            }
            let extension = extension.unwrap();
            "rs" == extension
        })
}
