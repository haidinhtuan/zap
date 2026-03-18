use matrix_sdk::ruma::events::room::message::{
    MessageType, OriginalSyncRoomMessageEvent, SyncRoomMessageEvent,
};
use matrix_sdk::Client;
use std::time::Duration;
use tokio::sync::mpsc;

use crate::app::{Message, Room};

/// Events emitted by the Matrix sync loop.
#[derive(Debug)]
pub enum MatrixEvent {
    /// The full room list has been refreshed.
    RoomListUpdate(Vec<Room>),
    /// A new message arrived in a specific room.
    NewMessage { room_id: String, message: Message },
    /// An existing message was edited.
    MessageEdited {
        room_id: String,
        event_id: String,
        new_body: String,
    },
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

    // Register handler for incoming messages.
    let msg_tx = tx.clone();
    let own_user_id = client.user_id().map(|id| id.to_owned());
    client.add_event_handler(
        move |event: SyncRoomMessageEvent, room: matrix_sdk::Room| {
            let tx = msg_tx.clone();
            let own_uid = own_user_id.clone();
            async move {
                if let SyncRoomMessageEvent::Original(OriginalSyncRoomMessageEvent {
                    content,
                    sender,
                    origin_server_ts,
                    event_id,
                    ..
                }) = event
                {
                    // Detect replacement (edit) relation before extracting body.
                    if let Some(matrix_sdk::ruma::events::room::message::Relation::Replacement(replacement)) = &content.relates_to {
                        let original_event_id = replacement.event_id.to_string();
                        let new_body = match &replacement.new_content.msgtype {
                            MessageType::Text(text) => text.body.clone(),
                            MessageType::Notice(notice) => notice.body.clone(),
                            MessageType::Emote(emote) => format!("* {}", emote.body),
                            _ => return,
                        };
                        let _ = tx.send(MatrixEvent::MessageEdited {
                            room_id: room.room_id().to_string(),
                            event_id: original_event_id,
                            new_body,
                        });
                        return;
                    }

                    let body = match content.msgtype {
                        MessageType::Text(text) => text.body,
                        MessageType::Notice(notice) => notice.body,
                        MessageType::Emote(emote) => format!("* {}", emote.body),
                        _ => return,
                    };

                    let millis: i64 =
                        i64::from(origin_server_ts.as_secs()) * 1000;
                    let timestamp =
                        chrono::DateTime::from_timestamp_millis(millis)
                            .unwrap_or_else(chrono::Utc::now);

                    // Resolve display name from room membership.
                    let display_name = room
                        .get_member_no_sync(&sender)
                        .await
                        .ok()
                        .flatten()
                        .and_then(|m| m.display_name().map(|n| n.to_string()))
                        .unwrap_or_else(|| sender.localpart().to_string());

                    // Check for reply.
                    let reply_to = content
                        .relates_to
                        .as_ref()
                        .and_then(|r| {
                            if let matrix_sdk::ruma::events::room::message::Relation::Reply { in_reply_to } = r {
                                Some(in_reply_to.event_id.to_string())
                            } else {
                                None
                            }
                        });

                    // Strip the fallback reply prefix from body if present.
                    let clean_body = if body.starts_with("> ") {
                        body.lines()
                            .skip_while(|l| l.starts_with("> ") || l.is_empty())
                            .collect::<Vec<_>>()
                            .join("\n")
                    } else {
                        body
                    };

                    let is_own = own_uid.as_ref() == Some(&sender);

                    let _ = tx.send(MatrixEvent::NewMessage {
                        room_id: room.room_id().to_string(),
                        message: Message {
                            event_id: Some(event_id.to_string()),
                            sender: display_name,
                            body: clean_body,
                            timestamp,
                            is_own,
                            reply_to,
                        },
                    });
                }
            }
        },
    );

    tokio::spawn(async move {
        // Auto-join any invited rooms before first sync.
        auto_join_invites(&client).await;

        // Send the initial room list.
        let rooms = get_room_list(&client).await;
        let _ = tx.send(MatrixEvent::RoomListUpdate(rooms));

        // Continuous sync loop.
        // Set server-side long-poll timeout for sync.
        let settings = matrix_sdk::config::SyncSettings::default()
            .timeout(Duration::from_secs(30));
        let mut sync_token: Option<String> = None;

        loop {
            let s = if let Some(ref token) = sync_token {
                settings.clone().token(token.clone())
            } else {
                settings.clone()
            };

            match client.sync_once(s).await {
                Ok(response) => {
                    sync_token = Some(response.next_batch);

                    // Auto-join any new invites.
                    auto_join_invites(&client).await;

                    // Timeout the room list fetch so a hanging HTTP request
                    // doesn't block the entire sync loop.
                    let rooms = match tokio::time::timeout(
                        Duration::from_secs(30),
                        get_room_list(&client),
                    ).await {
                        Ok(rooms) => rooms,
                        Err(_) => {
                            tracing::warn!("get_room_list timed out, using empty list");
                            vec![]
                        }
                    };
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

/// Auto-join all rooms the user has been invited to.
async fn auto_join_invites(client: &Client) {
    for room in client.invited_rooms() {
        let room_id = room.room_id().to_owned();
        tracing::info!("Auto-joining invited room: {}", room_id);
        if let Err(e) = room.join().await {
            tracing::warn!("Failed to join room {}: {}", room_id, e);
        }
    }
}

/// Build a list of Room structs from the client's joined rooms, sorted by
/// most recent activity (newest first).
pub async fn get_room_list(client: &Client) -> Vec<Room> {
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

        // Get the timestamp of the latest message in this room by fetching 1 message.
        let last_activity = {
            let mut opts = matrix_sdk::room::MessagesOptions::backward();
            opts.limit = 1u32.into();
            room.messages(opts).await.ok().and_then(|resp| {
                resp.chunk.first().and_then(|ev| {
                    ev.raw().deserialize().ok().map(|e: matrix_sdk::ruma::events::AnySyncTimelineEvent| {
                        let secs = i64::from(e.origin_server_ts().as_secs());
                        chrono::DateTime::from_timestamp(secs, 0)
                            .unwrap_or_else(chrono::Utc::now)
                    })
                })
            })
        };

        // Use m.direct flag OR member count heuristic (bridges often don't set m.direct).
        // Bridged DMs may have extra members (multiple bots, service accounts),
        // so we use a threshold of 5 to catch these cases.
        let member_count = room.joined_members_count();
        let is_direct = room.is_direct().await.unwrap_or(false)
            || member_count <= 5;
        tracing::debug!("Room '{}': members={}, is_direct={}", name, member_count, is_direct);

        rooms.push(Room {
            id: room.room_id().to_string(),
            name,
            unread_count,
            last_activity,
            is_direct,
        });
    }

    // Sort by last_activity descending (most recent first).
    rooms.sort_by(|a, b| b.last_activity.cmp(&a.last_activity));

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
                event_id: None,
                sender: "@alice:example.com".to_string(),
                body: "hello".to_string(),
                timestamp: chrono::Utc::now(),
                is_own: false,
                reply_to: None,
            },
        };
        let _edited = MatrixEvent::MessageEdited {
            room_id: "!test:example.com".to_string(),
            event_id: "$ev1".to_string(),
            new_body: "edited hello".to_string(),
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
            is_direct: false,
        };
        assert_eq!(room.id, "!room:example.com");
        assert_eq!(room.name, "Test Room");
        assert_eq!(room.unread_count, 5);
        assert!(room.last_activity.is_none());
    }
}
