use matrix_sdk::Client;
use tokio::sync::mpsc;

use crate::app::{Message, Room};

/// Events emitted by the Matrix sync loop.
#[derive(Debug)]
pub enum MatrixEvent {
    /// The full room list has been refreshed.
    RoomListUpdate(Vec<Room>),
    /// A new message arrived in a specific room.
    NewMessage { room_id: String, message: Message },
    /// The sync loop encountered an error.
    SyncError(String),
}

/// Start the background sync loop.
///
/// Returns an unbounded receiver that yields `MatrixEvent` values as rooms
/// and messages are updated. The sync runs in a spawned tokio task and will
/// retry on transient errors after a short delay.
pub fn start_sync(client: Client) -> mpsc::UnboundedReceiver<MatrixEvent> {
    let (tx, rx) = mpsc::unbounded_channel();

    tokio::spawn(async move {
        // Send the initial room list.
        let rooms = get_room_list(&client).await;
        let _ = tx.send(MatrixEvent::RoomListUpdate(rooms));

        // Continuous sync loop.
        let settings = matrix_sdk::config::SyncSettings::default();
        loop {
            match client.sync_once(settings.clone()).await {
                Ok(_response) => {
                    let rooms = get_room_list(&client).await;
                    let _ = tx.send(MatrixEvent::RoomListUpdate(rooms));
                }
                Err(e) => {
                    let _ = tx.send(MatrixEvent::SyncError(e.to_string()));
                    tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                }
            }
        }
    });

    rx
}

/// Build a list of Room structs from the client's joined rooms.
async fn get_room_list(client: &Client) -> Vec<Room> {
    let mut rooms = Vec::new();
    for room in client.joined_rooms() {
        let name = room
            .cached_display_name()
            .map(|n| n.to_string())
            .unwrap_or_else(|| "Unknown".to_string());

        let unread_count = room
            .unread_notification_counts()
            .notification_count
            .try_into()
            .unwrap_or(0u32);

        rooms.push(Room {
            id: room.room_id().to_string(),
            name,
            unread_count,
            last_activity: None,
        });
    }
    rooms
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_matrix_event_variants() {
        // Verify all MatrixEvent variants can be constructed.
        let _update = MatrixEvent::RoomListUpdate(vec![]);
        let _msg = MatrixEvent::NewMessage {
            room_id: "!test:example.com".to_string(),
            message: Message {
                sender: "@alice:example.com".to_string(),
                body: "hello".to_string(),
                timestamp: chrono::Utc::now(),
                is_own: false,
            },
        };
        let _err = MatrixEvent::SyncError("timeout".to_string());
    }

    #[test]
    fn test_room_conversion_fields() {
        // Verify that our Room struct has the expected fields for sync updates.
        let room = Room {
            id: "!room:example.com".to_string(),
            name: "Test Room".to_string(),
            unread_count: 5,
            last_activity: None,
        };
        assert_eq!(room.id, "!room:example.com");
        assert_eq!(room.name, "Test Room");
        assert_eq!(room.unread_count, 5);
        assert!(room.last_activity.is_none());
    }
}
