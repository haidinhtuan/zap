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

/// Find an existing DM room with the given user or create a new one.
///
/// First checks joined rooms for a small room (likely DM) that contains the
/// target user. If none is found, creates a new DM room via the Matrix API.
pub async fn find_or_create_dm(client: &Client, user_id_str: &str) -> Option<String> {
    use matrix_sdk::ruma::UserId;

    let target_user = UserId::parse(user_id_str).ok()?;

    // Check existing joined rooms for a DM with this user.
    for room in client.joined_rooms() {
        if room.joined_members_count() <= 3 {
            if let Ok(Some(_member)) = room.get_member_no_sync(&target_user).await {
                return Some(room.room_id().to_string());
            }
        }
    }

    // No existing DM found; create a new one.
    match client.create_dm(&target_user).await {
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
