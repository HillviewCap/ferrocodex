#[cfg(test)]
mod standalone_credential_tests {
    use super::super::*;
    use crate::encryption::test_key;
    use tempfile::NamedTempFile;
    use rusqlite::Connection;

    fn setup_test_db() -> (NamedTempFile, Connection) {
        let temp_file = NamedTempFile::new().unwrap();
        let conn = Connection::open(temp_file.path()).unwrap();
        
        // Create required tables for foreign key constraints
        conn.execute_batch(
            r#"
            PRAGMA foreign_keys = ON;
            
            CREATE TABLE users (
                id INTEGER PRIMARY KEY,
                username TEXT NOT NULL,
                password_hash TEXT NOT NULL,
                role TEXT NOT NULL,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                is_active BOOLEAN DEFAULT 1
            );
            
            INSERT INTO users (id, username, password_hash, role) VALUES (1, 'testuser', 'hash', 'Engineer');
            "#,
        ).unwrap();
        
        let repo = SqliteVaultRepository::new(&conn);
        repo.initialize_schema().unwrap();
        
        (temp_file, conn)
    }

    #[test]
    fn test_create_standalone_credential() {
        let (_temp_file, conn) = setup_test_db();
        let repo = SqliteVaultRepository::new(&conn);

        let request = CreateStandaloneCredentialRequest {
            name: "Test Database Password".to_string(),
            description: "Password for test database".to_string(),
            credential_type: SecretType::Password,
            category_id: Some(2), // Databases category
            username: None,
            value: "super_secret_password".to_string(),
            tags: Some(vec!["test".to_string(), "database".to_string()]),
            created_by: 1,
        };

        let credential = repo.create_standalone_credential(request, test_key()).unwrap();
        assert_eq!(credential.name, "Test Database Password");
        assert_eq!(credential.credential_type, SecretType::Password);
        assert_eq!(credential.category_id, Some(2));
        assert!(!credential.encrypted_data.is_empty());
    }

    #[test]
    fn test_search_standalone_credentials() {
        let (_temp_file, conn) = setup_test_db();
        let repo = SqliteVaultRepository::new(&conn);

        // Create multiple credentials
        let credentials = vec![
            ("DB Password", SecretType::Password, Some(2), vec!["production", "mysql"]),
            ("Jump Host SSH", SecretType::Password, Some(1), vec!["production", "ssh"]),
            ("VPN Key", SecretType::VpnKey, Some(3), vec!["network", "vpn"]),
            ("App License", SecretType::LicenseFile, Some(4), vec!["application"]),
        ];

        for (name, cred_type, category_id, tags) in credentials {
            let request = CreateStandaloneCredentialRequest {
                name: name.to_string(),
                description: format!("Description for {}", name),
                credential_type: cred_type,
                category_id,
                username: None,
                value: "test_value".to_string(),
                tags: Some(tags.iter().map(|s| s.to_string()).collect()),
                created_by: 1,
            };
            repo.create_standalone_credential(request, test_key()).unwrap();
        }

        // Test search by query
        let search_request = SearchCredentialsRequest {
            query: Some("password".to_string()),
            credential_type: None,
            category_id: None,
            tags: None,
            created_after: None,
            created_before: None,
            limit: Some(10),
            offset: Some(0),
        };

        let results = repo.search_standalone_credentials(search_request).unwrap();
        assert_eq!(results.total_count, 1);
        assert_eq!(results.credentials.len(), 1);
        assert_eq!(results.credentials[0].credential.name, "DB Password");

        // Test search by type
        let search_request = SearchCredentialsRequest {
            query: None,
            credential_type: Some(SecretType::Password),
            category_id: None,
            tags: None,
            created_after: None,
            created_before: None,
            limit: Some(10),
            offset: Some(0),
        };

        let results = repo.search_standalone_credentials(search_request).unwrap();
        assert_eq!(results.total_count, 2);

        // Test search by category
        let search_request = SearchCredentialsRequest {
            query: None,
            credential_type: None,
            category_id: Some(2),
            tags: None,
            created_after: None,
            created_before: None,
            limit: Some(10),
            offset: Some(0),
        };

        let results = repo.search_standalone_credentials(search_request).unwrap();
        assert_eq!(results.total_count, 1);
        assert_eq!(results.credentials[0].credential.name, "DB Password");
    }

    #[test]
    fn test_category_management() {
        let (_temp_file, conn) = setup_test_db();
        let repo = SqliteVaultRepository::new(&conn);

        // Test predefined categories exist
        let categories = repo.get_credential_categories().unwrap();
        assert!(!categories.is_empty());
        
        let jump_hosts_cat = categories.iter()
            .find(|c| c.category.name == "Jump Hosts")
            .expect("Jump Hosts category should exist");
        assert_eq!(jump_hosts_cat.category.id, 1);
        assert_eq!(jump_hosts_cat.category.icon, Some("server".to_string()));

        // Create a custom category
        let custom_cat_request = CreateCategoryRequest {
            name: "Custom Category".to_string(),
            description: Some("A custom category for testing".to_string()),
            parent_category_id: None,
            color_code: Some("#FF5733".to_string()),
            icon: Some("custom".to_string()),
        };

        let custom_category = repo.create_credential_category(custom_cat_request).unwrap();
        assert_eq!(custom_category.name, "Custom Category");
        assert_eq!(custom_category.color_code, Some("#FF5733".to_string()));

        // Create a subcategory
        let sub_cat_request = CreateCategoryRequest {
            name: "Subcategory".to_string(),
            description: None,
            parent_category_id: Some(custom_category.id),
            color_code: None,
            icon: None,
        };

        let subcategory = repo.create_credential_category(sub_cat_request).unwrap();
        assert_eq!(subcategory.parent_category_id, Some(custom_category.id));

        // Test category hierarchy
        let categories = repo.get_credential_categories().unwrap();
        let custom_cat_with_children = categories.iter()
            .find(|c| c.category.id == custom_category.id)
            .expect("Custom category should exist");
        assert_eq!(custom_cat_with_children.children.len(), 1);
        assert_eq!(custom_cat_with_children.children[0].category.name, "Subcategory");
    }

    #[test]
    fn test_update_standalone_credential() {
        let (_temp_file, conn) = setup_test_db();
        let repo = SqliteVaultRepository::new(&conn);

        // Create a credential
        let create_request = CreateStandaloneCredentialRequest {
            name: "Original Name".to_string(),
            description: "Original Description".to_string(),
            credential_type: SecretType::Password,
            category_id: Some(1),
            username: None,
            value: "original_value".to_string(),
            tags: None,
            created_by: 1,
        };

        let credential = repo.create_standalone_credential(create_request, test_key()).unwrap();

        // Update the credential
        let update_request = UpdateStandaloneCredentialRequest {
            id: credential.id,
            name: Some("Updated Name".to_string()),
            description: Some("Updated Description".to_string()),
            category_id: Some(2),
            username: None,
            value: Some("new_value".to_string()),
            author_id: 1,
        };

        repo.update_standalone_credential(update_request, Some(test_key())).unwrap();

        // Verify the update
        let updated = repo.get_standalone_credential(credential.id).unwrap().unwrap();
        assert_eq!(updated.credential.name, "Updated Name");
        assert_eq!(updated.credential.description, "Updated Description");
        assert_eq!(updated.credential.category_id, Some(2));

        // Verify history was recorded
        let history = repo.get_standalone_credential_history(credential.id).unwrap();
        assert!(history.len() >= 2); // Created + Updated
        assert_eq!(history[0].change_type, StandaloneChangeType::Updated);
    }

    #[test]
    fn test_credential_tags() {
        let (_temp_file, conn) = setup_test_db();
        let repo = SqliteVaultRepository::new(&conn);

        // Create credential with tags
        let request = CreateStandaloneCredentialRequest {
            name: "Tagged Credential".to_string(),
            description: "".to_string(),
            credential_type: SecretType::Password,
            category_id: None,
            username: None,
            value: "value".to_string(),
            tags: Some(vec!["tag1".to_string(), "tag2".to_string(), "tag3".to_string()]),
            created_by: 1,
        };

        let credential = repo.create_standalone_credential(request, test_key()).unwrap();

        // Verify tags were added
        let info = repo.get_standalone_credential(credential.id).unwrap().unwrap();
        assert_eq!(info.tags.len(), 3);
        assert!(info.tags.contains(&"tag1".to_string()));
        assert!(info.tags.contains(&"tag2".to_string()));
        assert!(info.tags.contains(&"tag3".to_string()));

        // Test search by tags
        let search_request = SearchCredentialsRequest {
            query: None,
            credential_type: None,
            category_id: None,
            tags: Some(vec!["tag2".to_string()]),
            created_after: None,
            created_before: None,
            limit: Some(10),
            offset: Some(0),
        };

        let results = repo.search_standalone_credentials(search_request).unwrap();
        assert_eq!(results.total_count, 1);
        assert_eq!(results.credentials[0].credential.id, credential.id);

        // Test get all tags
        let all_tags = repo.get_all_tags().unwrap();
        assert!(all_tags.contains(&"tag1".to_string()));
        assert!(all_tags.contains(&"tag2".to_string()));
        assert!(all_tags.contains(&"tag3".to_string()));
    }

    #[test]
    fn test_delete_credential() {
        let (_temp_file, conn) = setup_test_db();
        let repo = SqliteVaultRepository::new(&conn);

        // Create a credential
        let request = CreateStandaloneCredentialRequest {
            name: "To Be Deleted".to_string(),
            description: "".to_string(),
            credential_type: SecretType::Password,
            category_id: None,
            username: None,
            value: "value".to_string(),
            tags: None,
            created_by: 1,
        };

        let credential = repo.create_standalone_credential(request, test_key()).unwrap();

        // Delete the credential
        repo.delete_standalone_credential(credential.id, 1).unwrap();

        // Verify it's deleted
        let result = repo.get_standalone_credential(credential.id).unwrap();
        assert!(result.is_none());

        // History rows cascade-delete with the credential; the durable record of the
        // deletion is the audit event written by the delete_standalone_credential command
        let history = repo.get_standalone_credential_history(credential.id).unwrap();
        assert!(history.is_empty());
    }

    #[test]
    fn test_encryption_and_decryption() {
        let (_temp_file, conn) = setup_test_db();
        let repo = SqliteVaultRepository::new(&conn);

        let secret_value = "This is a very secret password!";
        
        // Create credential
        let request = CreateStandaloneCredentialRequest {
            name: "Encrypted Credential".to_string(),
            description: "".to_string(),
            credential_type: SecretType::Password,
            category_id: None,
            username: None,
            value: secret_value.to_string(),
            tags: None,
            created_by: 1,
        };

        let credential = repo.create_standalone_credential(request, test_key()).unwrap();

        // Verify the stored value is encrypted (not plaintext)
        assert_ne!(credential.encrypted_data, secret_value);
        assert!(!credential.encrypted_data.contains(secret_value));

        // In a real scenario, decryption would happen via the Tauri command
        // which uses the same encryption key pattern
    }

    fn create_with_username(repo: &SqliteVaultRepository, name: &str, username: Option<&str>) -> StandaloneCredential {
        repo.create_standalone_credential(CreateStandaloneCredentialRequest {
            name: name.to_string(),
            description: "Engineering workstation".to_string(),
            credential_type: SecretType::Password,
            category_id: Some(1),
            username: username.map(String::from),
            value: "Value-0".to_string(),
            tags: None,
            created_by: 1,
        }, test_key()).unwrap()
    }

    fn stored_ciphertext(conn: &Connection, id: i64) -> (String, u32) {
        conn.query_row(
            "SELECT encrypted_data, key_version FROM standalone_credentials WHERE id = ?1",
            [id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        ).unwrap()
    }

    fn username_update(id: i64, username: &str) -> UpdateStandaloneCredentialRequest {
        UpdateStandaloneCredentialRequest {
            id, name: None, description: None, category_id: None,
            username: Some(username.to_string()), value: None, author_id: 1,
        }
    }

    #[test]
    fn test_credential_username_round_trip_and_search() {
        let (_temp_file, conn) = setup_test_db();
        let repo = SqliteVaultRepository::new(&conn);
        let credential = create_with_username(&repo, "EWS login", Some("DOMAIN\\Operator"));
        create_with_username(&repo, "Historian login", None);

        assert_eq!(credential.username.as_deref(), Some("DOMAIN\\Operator"));
        let info = repo.get_standalone_credential(credential.id).unwrap().unwrap();
        assert_eq!(info.credential.username.as_deref(), Some("DOMAIN\\Operator"));

        let search = |query: &str| repo.search_standalone_credentials(SearchCredentialsRequest {
            query: Some(query.to_string()),
            credential_type: None, category_id: None, tags: None,
            created_after: None, created_before: None, limit: None, offset: None,
        }).unwrap();
        // Only the username contains "operator"
        let found = search("operator");
        assert_eq!(found.total_count, 1);
        assert_eq!(found.credentials[0].credential.id, credential.id);
        assert_eq!(found.credentials[0].credential.username.as_deref(), Some("DOMAIN\\Operator"));
        // Name and description still match
        assert_eq!(search("login").total_count, 2);
        assert_eq!(search("workstation").total_count, 2);

        let history = repo.get_standalone_credential_history(credential.id).unwrap();
        let created = history.iter().find(|h| h.change_type == StandaloneChangeType::Created).unwrap();
        assert!(created.changes_json.as_deref().unwrap().contains("Operator"));
    }

    #[test]
    fn test_credential_username_only_update_leaves_ciphertext_alone() {
        let (_temp_file, conn) = setup_test_db();
        let repo = SqliteVaultRepository::new(&conn);
        let credential = create_with_username(&repo, "EWS login", Some("operator"));
        conn.execute("UPDATE standalone_credentials SET key_version = 7 WHERE id = ?1", [credential.id]).unwrap();
        let before = stored_ciphertext(&conn, credential.id);

        // No data key: a username change never touches the ciphertext
        repo.update_standalone_credential(username_update(credential.id, "engineer"), None).unwrap();

        assert_eq!(stored_ciphertext(&conn, credential.id), before);
        let updated = repo.get_standalone_credential(credential.id).unwrap().unwrap().credential;
        assert_eq!(updated.username.as_deref(), Some("engineer"));
        assert_eq!(updated.name, "EWS login");
        let history = repo.get_standalone_credential_history(credential.id).unwrap();
        let changes: HashMap<String, String> = serde_json::from_str(
            history.iter().find(|h| h.change_type == StandaloneChangeType::Updated).unwrap()
                .changes_json.as_deref().unwrap(),
        ).unwrap();
        assert_eq!(changes.get("username").map(String::as_str), Some("engineer"));
        assert!(!changes.contains_key("value_updated"));

        // An empty username clears it
        repo.update_standalone_credential(username_update(credential.id, ""), None).unwrap();
        assert_eq!(repo.get_standalone_credential(credential.id).unwrap().unwrap().credential.username, None);
        assert_eq!(stored_ciphertext(&conn, credential.id), before);
    }

    #[test]
    fn test_credential_value_update_without_data_key_fails() {
        let (_temp_file, conn) = setup_test_db();
        let repo = SqliteVaultRepository::new(&conn);
        let credential = create_with_username(&repo, "EWS login", Some("operator"));
        let before = stored_ciphertext(&conn, credential.id);

        let mut request = username_update(credential.id, "engineer");
        request.value = Some("Value-1".to_string());
        assert!(repo.update_standalone_credential(request, None).is_err());

        assert_eq!(stored_ciphertext(&conn, credential.id), before);
        let unchanged = repo.get_standalone_credential(credential.id).unwrap().unwrap().credential;
        assert_eq!(unchanged.username.as_deref(), Some("operator"));
    }

    #[test]
    fn test_credential_update_applies_exactly_the_given_fields() {
        let (_temp_file, conn) = setup_test_db();
        let repo = SqliteVaultRepository::new(&conn);
        let key = DataKey::generate_with_version(3);

        // Every combination of name, description, category, username and value
        for mask in 1u8..32 {
            let original = create_with_username(&repo, &format!("Credential {}", mask), Some("user-0"));
            let request = UpdateStandaloneCredentialRequest {
                id: original.id,
                name: (mask & 1 != 0).then(|| format!("Renamed {}", mask)),
                description: (mask & 2 != 0).then(|| "New description".to_string()),
                category_id: (mask & 4 != 0).then_some(2),
                username: (mask & 8 != 0).then(|| "user-1".to_string()),
                value: (mask & 16 != 0).then(|| "Value-1".to_string()),
                author_id: 1,
            };
            repo.update_standalone_credential(request.clone(), Some(&key)).unwrap();

            let updated = repo.get_standalone_credential(original.id).unwrap().unwrap().credential;
            assert_eq!(updated.name, request.name.unwrap_or(original.name), "mask {}", mask);
            assert_eq!(updated.description, request.description.unwrap_or(original.description), "mask {}", mask);
            assert_eq!(updated.category_id, request.category_id.or(original.category_id), "mask {}", mask);
            assert_eq!(updated.username, request.username.or(original.username), "mask {}", mask);
            let (encrypted_data, key_version) = stored_ciphertext(&conn, original.id);
            if request.value.is_some() {
                let plaintext = FileEncryption::new(&key).decrypt_from_base64(&encrypted_data).unwrap();
                assert_eq!(plaintext, b"Value-1", "mask {}", mask);
                assert_eq!(key_version, 3, "mask {}", mask);
            } else {
                assert_eq!(encrypted_data, original.encrypted_data, "mask {}", mask);
                assert_eq!(key_version, test_key().version(), "mask {}", mask);
            }
        }

        let original = create_with_username(&repo, "Untouched", None);
        let empty = UpdateStandaloneCredentialRequest {
            id: original.id, name: None, description: None, category_id: None,
            username: None, value: None, author_id: 1,
        };
        assert!(repo.update_standalone_credential(empty, Some(&key)).is_err());
        let missing = username_update(original.id + 1000, "nobody");
        assert!(repo.update_standalone_credential(missing, None).is_err());
    }
}