use steel_macros::{ReadFrom, WriteTo};

#[derive(Clone, Debug, WriteTo, ReadFrom)]
pub struct KnownPack {
    #[write(as = "string")]
    #[read(as = "string")]
    pub namespace: String,
    #[write(as = "string")]
    #[read(as = "string")]
    pub id: String,
    #[write(as = "string")]
    #[read(as = "string")]
    pub version: String,
}

impl KnownPack {
    pub fn new(namespace: String, id: String, version: String) -> Self {
        Self {
            namespace,
            id,
            version,
        }
    }
}
