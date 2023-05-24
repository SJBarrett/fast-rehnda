use std::env;
use std::path::Path;
use ahash::AHashMap;
use crate::asset_data_loader::{ResourceReference, ResourceReferenceMapper};
use crate::asset_handle::AssetHandle;

#[derive(Debug, Eq, PartialEq)]
pub enum AssetState<T> {
    NotLoaded,
    Loading,
    Loaded(T),
    Unloading,
}

pub struct AssetServer<T: Asset<T>> {
    assets: AHashMap<u32, AssetState<T>>,
    resource_mapper: ResourceReferenceMapper,
}

impl <T: Asset<T>> Default for AssetServer<T> {
    fn default() -> Self {
        AssetServer::new()
    }
}

impl <AssetType: Asset<AssetType>> AssetServer<AssetType> {
    pub fn new() -> Self {
        AssetServer {
            assets: AHashMap::new(),
            resource_mapper: ResourceReferenceMapper::new(env::current_dir().unwrap().as_path()),
        }
    }

    pub fn new_initial_capacity_and_assert_dir(capacity: usize, asset_dir: &Path) -> Self {
        AssetServer {
            assets: AHashMap::with_capacity(capacity),
            resource_mapper: ResourceReferenceMapper::new(asset_dir),
        }
    }

    pub fn get_or_load_asset(&mut self, asset_handle: &AssetHandle<AssetType>) -> &AssetState<AssetType> {
        let resource_reference = self.resource_mapper.get_resource_reference(asset_handle)
            .expect("No file found for asset");
        match resource_reference {
            ResourceReference::File(file_path) => {
                let asset = AssetType::load_asset_from_file(file_path);
                self.assets.insert(asset_handle.handle, AssetState::Loaded(asset));
            }
        }
        self.assets.get(&asset_handle.handle).unwrap()
    }
}

pub trait Asset<T> {
    fn load_asset_from_file(asset_data: &Path) -> T;
}

#[cfg(test)]
mod tests {
    use std::fs::File;
    use std::io::{BufRead, BufReader};
    use super::*;

    #[derive(Debug, Eq, PartialEq)]
    struct TestStruct {
        value: u32,
    }

    impl Asset<TestStruct> for TestStruct {
        fn load_asset_from_file(asset_data: &Path) -> Self {
            let file = File::open(asset_data).unwrap();
            let reader = BufReader::new(file);

            let value = reader.lines().next().unwrap().unwrap().parse::<u32>().unwrap();
            Self {
                value
            }
        }
    }

    #[test]
    fn test_loading_asset() {
        let mut server = AssetServer::<TestStruct>::new_initial_capacity_and_assert_dir(2, Path::new("test_resources"));
        let asset = server.get_or_load_asset(&AssetHandle::<TestStruct>::new("12345u32.txt"));
        assert_eq!(asset, &AssetState::Loaded(TestStruct { value: 12345 }));
    }
}