use steel_macros::{ReadFrom, WriteTo};

#[derive(Clone, Debug, WriteTo, ReadFrom)]
pub struct KnownPack {
    #[write_as(as = "string")]
    #[read_as(as = "string")]
    pub namespace: String,
    #[write_as(as = "string")]
    #[read_as(as = "string")]
    pub id: String,
    #[write_as(as = "string")]
    #[read_as(as = "string")]
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
