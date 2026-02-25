use matrix_sdk::Client;

use crate::error::{ZapError, ZapResult};

/// Send a text message to the specified room.
pub async fn send_message(client: &Client, room_id: &str, body: &str) -> ZapResult<()> {
    use matrix_sdk::ruma::RoomId;
    use matrix_sdk::ruma::events::room::message::RoomMessageEventContent;

    let room_id = RoomId::parse(room_id)
        .map_err(|e| ZapError::Matrix(e.to_string()))?;

    let room = client
        .get_room(&room_id)
        .ok_or_else(|| ZapError::Matrix(format!("Room {} not found", room_id)))?;

    let content = RoomMessageEventContent::text_plain(body);
    room.send(content)
        .await
        .map_err(|e| ZapError::Matrix(e.to_string()))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_messages_module_compiles() {
        // Message sending requires a live Matrix session.
        // This test verifies the module compiles correctly.
        assert!(true);
    }
}
