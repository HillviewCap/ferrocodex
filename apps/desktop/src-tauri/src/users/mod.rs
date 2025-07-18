use anyhow::Result;
use rusqlite::{Connection, Row};
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum UserRole {
    Administrator,
    Engineer,
}

impl fmt::Display for UserRole {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            UserRole::Administrator => write!(f, "Administrator"),
            UserRole::Engineer => write!(f, "Engineer"),
        }
    }
}

impl std::str::FromStr for UserRole {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        match s {
            "Administrator" => Ok(UserRole::Administrator),
            "Engineer" => Ok(UserRole::Engineer),
            _ => Err(anyhow::anyhow!("Invalid user role: {}", s)),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: i64,
    pub username: String,
    pub password_hash: String,
    pub role: UserRole,
    pub created_at: String,
    pub updated_at: String,
    pub is_active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserInfo {
    pub id: i64,
    pub username: String,
    pub role: UserRole,
    pub created_at: String,
    pub is_active: bool,
}

impl From<User> for UserInfo {
    fn from(user: User) -> Self {
        UserInfo {
            id: user.id,
            username: user.username,
            role: user.role,
            created_at: user.created_at,
            is_active: user.is_active,
        }
    }
}

#[derive(Debug, Clone)]
pub struct CreateUserRequest {
    pub username: String,
    pub password: String,
    pub role: UserRole,
}

pub trait UserRepository {
    fn create_user(&self, request: CreateUserRequest) -> Result<User>;
    fn find_by_username(&self, username: &str) -> Result<Option<User>>;
    fn find_by_id(&self, id: i64) -> Result<Option<User>>;
    fn update_user(&self, user: &User) -> Result<()>;
    fn deactivate_user(&self, id: i64) -> Result<()>;
    fn has_admin_users(&self) -> Result<bool>;
    fn create_engineer_account(&self, username: String, password: String, created_by_admin: i64) -> Result<User>;
    fn list_all_users(&self) -> Result<Vec<User>>;
    fn reactivate_user(&self, id: i64) -> Result<()>;
}

pub struct SqliteUserRepository<'a> {
    conn: &'a Connection,
}

impl<'a> SqliteUserRepository<'a> {
    pub fn new(conn: &'a Connection) -> Self {
        Self { conn }
    }

    pub fn initialize_schema(&self) -> Result<()> {
        self.conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS users (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                username TEXT NOT NULL UNIQUE,
                password_hash TEXT NOT NULL,
                role TEXT NOT NULL,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                is_active BOOLEAN DEFAULT 1
            );

            CREATE INDEX IF NOT EXISTS idx_users_username ON users(username);
            CREATE INDEX IF NOT EXISTS idx_users_role ON users(role);
            CREATE INDEX IF NOT EXISTS idx_users_active ON users(is_active);
            "#,
        )?;
        Ok(())
    }

    fn row_to_user(row: &Row) -> rusqlite::Result<User> {
        let role_str: String = row.get("role")?;
        let role = role_str.parse::<UserRole>().map_err(|_| {
            rusqlite::Error::FromSqlConversionFailure(
                0,
                rusqlite::types::Type::Text,
                Box::new(std::io::Error::new(std::io::ErrorKind::InvalidData, "Invalid role")),
            )
        })?;

        Ok(User {
            id: row.get("id")?,
            username: row.get("username")?,
            password_hash: row.get("password_hash")?,
            role,
            created_at: row.get("created_at")?,
            updated_at: row.get("updated_at")?,
            is_active: row.get("is_active")?,
        })
    }
}

impl<'a> UserRepository for SqliteUserRepository<'a> {
    fn create_user(&self, request: CreateUserRequest) -> Result<User> {
        // Validate username
        if request.username.trim().is_empty() {
            return Err(anyhow::anyhow!("Username cannot be empty"));
        }
        if request.username.len() < 3 {
            return Err(anyhow::anyhow!("Username must be at least 3 characters long"));
        }
        if request.username.len() > 50 {
            return Err(anyhow::anyhow!("Username cannot exceed 50 characters"));
        }

        // Hash the password
        let password_hash = crate::auth::hash_password(&request.password)?;

        let mut stmt = self.conn.prepare(
            "INSERT INTO users (username, password_hash, role) VALUES (?1, ?2, ?3) RETURNING *"
        )?;

        let user = stmt.query_row(
            (&request.username, &password_hash, &request.role.to_string()),
            Self::row_to_user,
        )?;

        Ok(user)
    }

    fn find_by_username(&self, username: &str) -> Result<Option<User>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, username, password_hash, role, created_at, updated_at, is_active 
             FROM users WHERE username = ?1 AND is_active = 1"
        )?;

        let result = stmt.query_row([username], Self::row_to_user);
        
        match result {
            Ok(user) => Ok(Some(user)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    fn find_by_id(&self, id: i64) -> Result<Option<User>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, username, password_hash, role, created_at, updated_at, is_active 
             FROM users WHERE id = ?1"
        )?;

        let result = stmt.query_row([id], Self::row_to_user);
        
        match result {
            Ok(user) => Ok(Some(user)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    fn update_user(&self, user: &User) -> Result<()> {
        self.conn.execute(
            "UPDATE users SET username = ?1, password_hash = ?2, role = ?3, 
             updated_at = CURRENT_TIMESTAMP, is_active = ?4 WHERE id = ?5",
            (
                &user.username,
                &user.password_hash,
                &user.role.to_string(),
                &user.is_active,
                &user.id,
            ),
        )?;
        Ok(())
    }

    fn deactivate_user(&self, id: i64) -> Result<()> {
        self.conn.execute(
            "UPDATE users SET is_active = 0, updated_at = CURRENT_TIMESTAMP WHERE id = ?1",
            [id],
        )?;
        Ok(())
    }

    fn has_admin_users(&self) -> Result<bool> {
        let mut stmt = self.conn.prepare(
            "SELECT COUNT(*) FROM users WHERE role = 'Administrator' AND is_active = 1"
        )?;
        let count: i64 = stmt.query_row([], |row| row.get(0))?;
        Ok(count > 0)
    }

    fn create_engineer_account(&self, username: String, password: String, created_by_admin: i64) -> Result<User> {
        // Validate username
        if username.trim().is_empty() {
            return Err(anyhow::anyhow!("Username cannot be empty"));
        }
        if username.len() < 3 {
            return Err(anyhow::anyhow!("Username must be at least 3 characters long"));
        }
        if username.len() > 50 {
            return Err(anyhow::anyhow!("Username cannot exceed 50 characters"));
        }

        // Validate password
        if password.len() < 8 {
            return Err(anyhow::anyhow!("Password must be at least 8 characters long"));
        }

        // Check if username already exists
        if let Some(_) = self.find_by_username(&username)? {
            return Err(anyhow::anyhow!("Username already exists"));
        }

        // Verify that the creating user is an admin
        if let Some(admin_user) = self.find_by_id(created_by_admin)? {
            if admin_user.role != UserRole::Administrator || !admin_user.is_active {
                return Err(anyhow::anyhow!("Only active administrators can create engineer accounts"));
            }
        } else {
            return Err(anyhow::anyhow!("Creating user not found"));
        }

        // Hash the password
        let password_hash = crate::auth::hash_password(&password)?;

        let mut stmt = self.conn.prepare(
            "INSERT INTO users (username, password_hash, role) VALUES (?1, ?2, ?3) RETURNING *"
        )?;

        let user = stmt.query_row(
            (&username, &password_hash, &UserRole::Engineer.to_string()),
            Self::row_to_user,
        )?;

        Ok(user)
    }

    fn list_all_users(&self) -> Result<Vec<User>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, username, password_hash, role, created_at, updated_at, is_active 
             FROM users ORDER BY created_at DESC"
        )?;

        let user_iter = stmt.query_map([], Self::row_to_user)?;
        let mut users = Vec::new();

        for user in user_iter {
            users.push(user?);
        }

        Ok(users)
    }

    fn reactivate_user(&self, id: i64) -> Result<()> {
        let rows_affected = self.conn.execute(
            "UPDATE users SET is_active = 1, updated_at = CURRENT_TIMESTAMP WHERE id = ?1",
            [id],
        )?;
        
        if rows_affected == 0 {
            return Err(anyhow::anyhow!("User not found"));
        }
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;
    use rusqlite::Connection;
    use crate::auth;

    fn setup_test_db() -> (NamedTempFile, Connection) {
        let temp_file = NamedTempFile::new().unwrap();
        let conn = Connection::open(temp_file.path()).unwrap();
        
        // Initialize auth module for testing
        auth::tests::init_for_testing();
        
        let repo = SqliteUserRepository::new(&conn);
        repo.initialize_schema().unwrap();
        
        (temp_file, conn)
    }

    #[test]
    fn test_user_creation() {
        let (_temp_file, conn) = setup_test_db();
        let repo = SqliteUserRepository::new(&conn);

        let request = CreateUserRequest {
            username: "admin".to_string(),
            password: "password123".to_string(),
            role: UserRole::Administrator,
        };

        let user = repo.create_user(request).unwrap();
        assert_eq!(user.username, "admin");
        assert_eq!(user.role, UserRole::Administrator);
        assert!(user.is_active);
        assert!(!user.password_hash.is_empty());
    }

    #[test]
    fn test_username_validation() {
        let (_temp_file, conn) = setup_test_db();
        let repo = SqliteUserRepository::new(&conn);

        // Test empty username
        let request = CreateUserRequest {
            username: "".to_string(),
            password: "password123".to_string(),
            role: UserRole::Administrator,
        };
        assert!(repo.create_user(request).is_err());

        // Test short username
        let request = CreateUserRequest {
            username: "ad".to_string(),
            password: "password123".to_string(),
            role: UserRole::Administrator,
        };
        assert!(repo.create_user(request).is_err());

        // Test long username
        let request = CreateUserRequest {
            username: "a".repeat(51),
            password: "password123".to_string(),
            role: UserRole::Administrator,
        };
        assert!(repo.create_user(request).is_err());
    }

    #[test]
    fn test_find_by_username() {
        let (_temp_file, conn) = setup_test_db();
        let repo = SqliteUserRepository::new(&conn);

        let request = CreateUserRequest {
            username: "testuser".to_string(),
            password: "password123".to_string(),
            role: UserRole::Engineer,
        };

        let created_user = repo.create_user(request).unwrap();
        let found_user = repo.find_by_username("testuser").unwrap().unwrap();
        
        assert_eq!(created_user.id, found_user.id);
        assert_eq!(created_user.username, found_user.username);
    }

    #[test]
    fn test_username_uniqueness() {
        let (_temp_file, conn) = setup_test_db();
        let repo = SqliteUserRepository::new(&conn);

        let request1 = CreateUserRequest {
            username: "admin".to_string(),
            password: "password123".to_string(),
            role: UserRole::Administrator,
        };

        let request2 = CreateUserRequest {
            username: "admin".to_string(),
            password: "different_password".to_string(),
            role: UserRole::Engineer,
        };

        repo.create_user(request1).unwrap();
        assert!(repo.create_user(request2).is_err());
    }

    #[test]
    fn test_has_admin_users() {
        let (_temp_file, conn) = setup_test_db();
        let repo = SqliteUserRepository::new(&conn);

        // Initially no admin users
        assert!(!repo.has_admin_users().unwrap());

        // Create an engineer user
        let engineer_request = CreateUserRequest {
            username: "engineer".to_string(),
            password: "password123".to_string(),
            role: UserRole::Engineer,
        };
        repo.create_user(engineer_request).unwrap();
        
        // Still no admin users
        assert!(!repo.has_admin_users().unwrap());

        // Create an admin user
        let admin_request = CreateUserRequest {
            username: "admin".to_string(),
            password: "password123".to_string(),
            role: UserRole::Administrator,
        };
        repo.create_user(admin_request).unwrap();
        
        // Now we have admin users
        assert!(repo.has_admin_users().unwrap());
    }

    #[test]
    fn test_role_serialization() {
        assert_eq!(UserRole::Administrator.to_string(), "Administrator");
        assert_eq!(UserRole::Engineer.to_string(), "Engineer");
        
        assert_eq!("Administrator".parse::<UserRole>().unwrap(), UserRole::Administrator);
        assert_eq!("Engineer".parse::<UserRole>().unwrap(), UserRole::Engineer);
        
        assert!("InvalidRole".parse::<UserRole>().is_err());
    }

    #[test]
    fn test_create_engineer_account() {
        let (_temp_file, conn) = setup_test_db();
        let repo = SqliteUserRepository::new(&conn);

        // Create an admin user first
        let admin_request = CreateUserRequest {
            username: "admin".to_string(),
            password: "password123".to_string(),
            role: UserRole::Administrator,
        };
        let admin_user = repo.create_user(admin_request).unwrap();

        // Create an engineer account
        let engineer = repo.create_engineer_account(
            "engineer1".to_string(),
            "password123".to_string(),
            admin_user.id,
        ).unwrap();

        assert_eq!(engineer.username, "engineer1");
        assert_eq!(engineer.role, UserRole::Engineer);
        assert!(engineer.is_active);
    }

    #[test]
    fn test_create_engineer_account_validation() {
        let (_temp_file, conn) = setup_test_db();
        let repo = SqliteUserRepository::new(&conn);

        // Create an admin user first
        let admin_request = CreateUserRequest {
            username: "admin".to_string(),
            password: "password123".to_string(),
            role: UserRole::Administrator,
        };
        let admin_user = repo.create_user(admin_request).unwrap();

        // Test username validation
        assert!(repo.create_engineer_account("".to_string(), "password123".to_string(), admin_user.id).is_err());
        assert!(repo.create_engineer_account("ab".to_string(), "password123".to_string(), admin_user.id).is_err());
        assert!(repo.create_engineer_account("a".repeat(51), "password123".to_string(), admin_user.id).is_err());

        // Test password validation
        assert!(repo.create_engineer_account("engineer1".to_string(), "short".to_string(), admin_user.id).is_err());

        // Test duplicate username
        repo.create_engineer_account("engineer1".to_string(), "password123".to_string(), admin_user.id).unwrap();
        assert!(repo.create_engineer_account("engineer1".to_string(), "password456".to_string(), admin_user.id).is_err());
    }

    #[test]
    fn test_create_engineer_account_admin_validation() {
        let (_temp_file, conn) = setup_test_db();
        let repo = SqliteUserRepository::new(&conn);

        // Create an engineer user
        let engineer_request = CreateUserRequest {
            username: "engineer".to_string(),
            password: "password123".to_string(),
            role: UserRole::Engineer,
        };
        let engineer_user = repo.create_user(engineer_request).unwrap();

        // Engineer should not be able to create accounts
        assert!(repo.create_engineer_account(
            "engineer1".to_string(),
            "password123".to_string(),
            engineer_user.id,
        ).is_err());

        // Non-existent user should not be able to create accounts
        assert!(repo.create_engineer_account(
            "engineer1".to_string(),
            "password123".to_string(),
            999,
        ).is_err());
    }

    #[test]
    fn test_list_all_users() {
        let (_temp_file, conn) = setup_test_db();
        let repo = SqliteUserRepository::new(&conn);

        // Create multiple users
        let admin_request = CreateUserRequest {
            username: "admin".to_string(),
            password: "password123".to_string(),
            role: UserRole::Administrator,
        };
        let _admin_user = repo.create_user(admin_request).unwrap();

        let engineer_request = CreateUserRequest {
            username: "engineer".to_string(),
            password: "password123".to_string(),
            role: UserRole::Engineer,
        };
        repo.create_user(engineer_request).unwrap();

        // List all users
        let users = repo.list_all_users().unwrap();
        assert_eq!(users.len(), 2);
        
        // Check that both users are present
        let usernames: Vec<&String> = users.iter().map(|u| &u.username).collect();
        assert!(usernames.contains(&&"admin".to_string()));
        assert!(usernames.contains(&&"engineer".to_string()));
    }

    #[test]
    fn test_reactivate_user() {
        let (_temp_file, conn) = setup_test_db();
        let repo = SqliteUserRepository::new(&conn);

        // Create a user
        let request = CreateUserRequest {
            username: "testuser".to_string(),
            password: "password123".to_string(),
            role: UserRole::Engineer,
        };
        let user = repo.create_user(request).unwrap();

        // Deactivate the user
        repo.deactivate_user(user.id).unwrap();

        // Verify user is deactivated
        let deactivated_user = repo.find_by_id(user.id).unwrap().unwrap();
        assert!(!deactivated_user.is_active);

        // Reactivate the user
        repo.reactivate_user(user.id).unwrap();

        // Verify user is reactivated
        let reactivated_user = repo.find_by_id(user.id).unwrap().unwrap();
        assert!(reactivated_user.is_active);
    }

    #[test]
    fn test_reactivate_nonexistent_user() {
        let (_temp_file, conn) = setup_test_db();
        let repo = SqliteUserRepository::new(&conn);

        // Try to reactivate a user that doesn't exist
        assert!(repo.reactivate_user(999).is_err());
    }
}