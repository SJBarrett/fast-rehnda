use ahash::AHashMap;
use std::path::{Path, PathBuf};
use crate::asset_handle::{AssetHandle, hash_asset_name};

#[derive(Debug, PartialEq, Eq)]
pub enum ResourceReference {
    File(PathBuf),
}

pub struct ResourceReferenceMapper {
    handle_resources: AHashMap<u32, ResourceReference>,
}

impl ResourceReferenceMapper {
    pub fn new(asset_directory: &Path) -> Self {
        let mut handle_resources = AHashMap::new();
        let asset_directory = asset_directory.to_str().unwrap();
        println!("Searching dir: {}", asset_directory);
        let search_pattern = format!("{}/**/*.txt", asset_directory);
        let search_options = glob::MatchOptions {
            case_sensitive: false,
            require_literal_separator: false,
            require_literal_leading_dot: false,
        };
        for path in glob::glob_with(&search_pattern, search_options).unwrap().flatten() {
            use path_slash::PathExt as _;
            let asset_name = path.strip_prefix(asset_directory).unwrap();
            let asset_name_normalized = asset_name.to_slash().unwrap();

            let asset_handle = hash_asset_name(&asset_name_normalized);
            handle_resources.insert(asset_handle, ResourceReference::File(path));
        }
        ResourceReferenceMapper {
            handle_resources,
        }
    }

    pub fn get_resource_reference<T>(&self, asset_handle: &AssetHandle<T>) -> Option<&ResourceReference>{
        self.handle_resources.get(&asset_handle.handle)
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;
    use super::*;


    #[test]
    fn test_loading_asset() {
        let server = ResourceReferenceMapper::new(Path::new("C:/repos/fast-rehnda/crates/asset_manager/test_resources"));

        let test_file_1_handle = AssetHandle::<u32>::new("test_res1.txt");
        assert!(server.handle_resources.contains_key(&test_file_1_handle.handle));
        let file_1 = server.get_resource_reference(&test_file_1_handle).unwrap();
        assert_eq!(file_1, &ResourceReference::File(Path::new("test_res1.txt").to_path_buf()));

        let test_file_2_handle = AssetHandle::<u32>::new("subdirectory/sub_file1.txt");
        assert!(server.handle_resources.contains_key(&test_file_2_handle.handle));
        let file_2 = server.get_resource_reference(&test_file_2_handle).unwrap();
        assert_eq!(file_2, &ResourceReference::File(Path::new("subdirectory/sub_file1.txt").to_path_buf()));
    }
}