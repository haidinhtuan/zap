use matrix_sdk::Client;

use crate::app::Room;
use crate::error::ZapResult;

/// Fetch details for a specific room by its ID.
///
/// Returns `None` if the room is not found among the user's joined rooms.
pub async fn get_room_details(client: &Client, room_id: &str) -> ZapResult<Option<Room>> {
    use matrix_sdk::ruma::RoomId;

    let room_id = RoomId::parse(room_id)
        .map_err(|e| crate::error::ZapError::Matrix(e.to_string()))?;

    let Some(room) = client.get_room(&room_id) else {
        return Ok(None);
    };

    let name = room
        .cached_display_name()
        .map(|n| n.to_string())
        .unwrap_or_else(|| "Unknown".to_string());

    let unread_count = room
        .unread_notification_counts()
        .notification_count
        .try_into()
        .unwrap_or(0u32);

    Ok(Some(Room {
        id: room.room_id().to_string(),
        name,
        unread_count,
        last_activity: None,
    }))
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_rooms_module_compiles() {
        // Room detail fetching requires a live Matrix session.
        // This test verifies the module compiles correctly.
        assert!(true);
    }
}
