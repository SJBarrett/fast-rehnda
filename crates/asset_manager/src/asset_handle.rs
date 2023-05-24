

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct AssetHandle<T> {
    pub handle: u32,
    marker: std::marker::PhantomData<T>,
}

impl <T> AssetHandle<T> {
    pub const fn new(asset_name: &str) -> Self {
        AssetHandle {
            handle: hash_asset_name(asset_name),
            marker: std::marker::PhantomData,
        }
    }

    pub const fn hash_asset_name(asset_name: &str) -> u32 {
        const_fnv1a_hash::fnv1a_hash_str_32(asset_name)
    }
}

pub const fn hash_asset_name(asset_name: &str) -> u32 {
    const_fnv1a_hash::fnv1a_hash_str_32(asset_name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_two_handles_for_same_string_are_the_same() {
        let handle1: AssetHandle<f32> = AssetHandle::new("helloWorld");
        let handle2 = AssetHandle::<f32>::new("helloWorld");
        assert_eq!(handle1.handle, handle2.handle);
    }
}