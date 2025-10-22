use steel_macros::PacketWrite;

#[derive(PacketWrite)]
struct TestPacket {
    #[write_as(as = "var_int")]
    id: i32,
    #[write_as(as = "string", bound = 255)]
    name: String,
    #[write_as(as = "i32")]
    value: i32,
}

#[test]
fn test_packet_write_derive() {
    let packet = TestPacket {
        id: 42,
        name: "test".to_string(),
        value: 100,
    };

    // This test just ensures the macro compiles and the struct can be created
    // In a real test, you'd want to test the generated write_packet method
    let _ = packet;
}
