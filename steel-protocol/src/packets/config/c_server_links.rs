use steel_macros::{ClientPacket, WriteTo};
use steel_registry::packets::config::C_SERVER_LINKS;
// use steel_utils::text::TextComponent;

#[derive(ClientPacket, WriteTo, Clone, Debug)]
#[packet_id(Config = C_SERVER_LINKS)]
pub struct CServerLinks {
    pub links: Vec<Link>,
}

impl CServerLinks {
    pub fn new() -> CServerLinks {
        Self { links: vec![Link::new(ServerLinksType::BugReport, "test".into()), Link::new(ServerLinksType::Website, "huhu".into())] }
    }
}

#[derive(WriteTo, Clone, Copy, Debug)]
#[write(as = VarInt)]
pub enum ServerLinksType{
    BugReport = 0,
    CommunityGuidelines = 1,
    Support = 2,
    Status = 3,
    Feedback = 4,
    Community = 5,
    Website = 6,
    Forums = 7,
    News = 8,
    Announcements = 9
}

#[derive(WriteTo, Clone, Debug)]
pub struct Link {
    pub is_built_in: bool,
    pub label: ServerLinksType,
    #[write(as = Prefixed(VarInt), bound = 40)]
    pub url: String,
}

impl Link {
    pub fn new(label: ServerLinksType, url: String) -> Self {
        Self {
            is_built_in: true,
            label,
            url,
        }
    }
}
//
// #[derive(WriteTo, Clone, Debug)]
// #[write(as = VarInt)]
// pub enum Label {
//     BuiltIn(),
//     TextComponent(Box<TextComponent>),
// }

// impl Serialize for Label {
//     fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
//         match self {
//             Label::BuiltIn(link_type) => link_type.serialize(serializer),
//             Label::TextComponent(component) => component.serialize(serializer),
//         }
//     }
// }