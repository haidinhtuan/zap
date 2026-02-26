use matrix_sdk::ruma::api::client::room::create_room;
use matrix_sdk::ruma::assign;
use matrix_sdk::Client;

use crate::app::UserSearchResult;

/// Search the Matrix user directory for users matching the given term.
///
/// Returns up to 10 results. If the search term is too short or the server
/// returns an error, an empty list is returned.
pub async fn search_users(client: &Client, search_term: &str) -> Vec<UserSearchResult> {
    match client.search_users(search_term, 10).await {
        Ok(response) => response
            .results
            .into_iter()
            .map(|user| UserSearchResult {
                user_id: user.user_id.to_string(),
                display_name: user.display_name,
            })
            .collect(),
        Err(e) => {
            tracing::warn!("User directory search failed: {}", e);
            Vec::new()
        }
    }
}

/// Find an existing room with the given user.
///
/// Checks all joined rooms for one that contains the target user, skipping
/// dead rooms ("Empty Room ..."). Prefers non-encrypted rooms (bridged) over
/// encrypted ones, then smaller rooms over larger ones.
pub async fn find_existing_dm(client: &Client, user_id_str: &str) -> Option<String> {
    use matrix_sdk::ruma::UserId;

    let target_user = UserId::parse(user_id_str).ok()?;

    // Collect candidate rooms: only DM-sized rooms (<=3 members: self + other + bridge bot).
    let mut candidates: Vec<(String, bool, u64)> = Vec::new();

    for room in client.joined_rooms() {
        let count = room.joined_members_count();
        // Only consider small rooms (likely DMs). Skip groups.
        if count > 3 {
            continue;
        }

        // Skip dead/empty rooms left over from failed DM creation.
        let name = room
            .cached_display_name()
            .map(|n| n.to_string())
            .unwrap_or_default();
        if name.starts_with("Empty Room") {
            continue;
        }

        if let Ok(Some(_member)) = room.get_member_no_sync(&target_user).await {
            let encrypted = room.encryption_state().is_encrypted();
            candidates.push((room.room_id().to_string(), encrypted, count));
        }
    }

    // Sort: non-encrypted first, then by smallest member count.
    // This prefers bridged rooms (unencrypted) over Matrix-native DMs.
    candidates.sort_by_key(|(_id, encrypted, count)| (*encrypted, *count));
    candidates.into_iter().next().map(|(id, _, _)| id)
}

/// Create a new unencrypted DM room with the given user.
///
/// Unlike `client.create_dm()` which always enables encryption (breaking
/// bridges like mautrix-meta), this creates a plain DM room that bridges
/// can relay messages through.
pub async fn create_dm_unencrypted(client: &Client, user_id_str: &str) -> Option<String> {
    use matrix_sdk::ruma::UserId;

    let target_user = UserId::parse(user_id_str).ok()?;

    let request = assign!(create_room::v3::Request::new(), {
        invite: vec![target_user.to_owned()],
        is_direct: true,
        preset: Some(create_room::v3::RoomPreset::TrustedPrivateChat),
    });

    match client.create_room(request).await {
        Ok(room) => Some(room.room_id().to_string()),
        Err(e) => {
            tracing::warn!("Failed to create DM room: {}", e);
            None
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_contacts_module_compiles() {
        // Contact search and DM creation require a live Matrix session.
        // This test verifies the module compiles correctly.
        assert!(true);
    }
}
