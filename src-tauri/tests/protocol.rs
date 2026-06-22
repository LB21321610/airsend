use app_lib::protocol::ControlMessage;

#[test]
fn chunk_messages_include_file_index_to_avoid_multi_file_collisions() {
    let request = ControlMessage::ChunkRequest {
        transfer_id: "transfer-1".into(),
        file_index: 7,
        chunk_index: 0,
    };
    let complete = ControlMessage::ChunkComplete {
        transfer_id: "transfer-1".into(),
        file_index: 7,
        chunk_index: 0,
        blake3: "abc123".into(),
    };

    let request_json = serde_json::to_value(request).expect("request should serialize");
    let complete_json = serde_json::to_value(complete).expect("complete should serialize");

    assert_eq!(request_json["fileIndex"], 7);
    assert_eq!(complete_json["fileIndex"], 7);
}
