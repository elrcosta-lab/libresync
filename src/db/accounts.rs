use rusqlite::params;

use crate::auth::models::{Account, AccountStatus};
use crate::db::{Database, DbError};

fn status_to_str(s: &AccountStatus) -> &'static str {
    match s {
        AccountStatus::Active => "Active",
        AccountStatus::Revoked => "Revoked",
        AccountStatus::Expired => "Expired",
        AccountStatus::RequiresReauth => "RequiresReauth",
    }
}

fn status_from_str(s: &str) -> Result<AccountStatus, DbError> {
    match s {
        "Active" => Ok(AccountStatus::Active),
        "Revoked" => Ok(AccountStatus::Revoked),
        "Expired" => Ok(AccountStatus::Expired),
        "RequiresReauth" => Ok(AccountStatus::RequiresReauth),
        other => Err(DbError::InvalidAccountStatus(other.to_string())),
    }
}

pub fn insert_account(db: &Database, account: &Account) -> Result<(), DbError> {
    let conn = db.conn();
    conn.execute(
        "INSERT INTO accounts (id, email, display_name, avatar_url, scope, token_expires_at, status, is_active, created_at, last_sync_at, quota_total, quota_used)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
        params![
            account.id,
            account.email,
            account.display_name,
            account.avatar_url,
            account.scope,
            account.token_expires_at,
            status_to_str(&account.status),
            account.is_active as i32,
            account.created_at,
            account.last_sync_at,
            account.quota_total,
            account.quota_used,
        ],
    )?;
    Ok(())
}

fn row_to_account(row: &rusqlite::Row) -> Result<Account, rusqlite::Error> {
    Ok(Account {
        id: row.get("id")?,
        email: row.get("email")?,
        display_name: row.get("display_name")?,
        avatar_url: row.get("avatar_url")?,
        scope: row.get("scope")?,
        token_expires_at: row.get("token_expires_at")?,
        status: status_from_str(row.get::<_, String>("status")?.as_str())
            .unwrap_or(AccountStatus::Active),
        is_active: row.get::<_, i32>("is_active")? != 0,
        created_at: row.get("created_at")?,
        last_sync_at: row.get("last_sync_at")?,
        quota_total: row.get("quota_total")?,
        quota_used: row.get("quota_used")?,
    })
}

pub fn get_account(db: &Database, id: &str) -> Result<Option<Account>, DbError> {
    let conn = db.conn();
    let mut stmt = conn.prepare(
        "SELECT id, email, display_name, avatar_url, scope, token_expires_at, status, is_active, created_at, last_sync_at, quota_total, quota_used FROM accounts WHERE id = ?1",
    )?;
    let mut rows = stmt.query(params![id])?;
    match rows.next()? {
        Some(row) => Ok(Some(row_to_account(row)?)),
        None => Ok(None),
    }
}

pub fn get_account_by_email(db: &Database, email: &str) -> Result<Option<Account>, DbError> {
    let conn = db.conn();
    let mut stmt = conn.prepare(
        "SELECT id, email, display_name, avatar_url, scope, token_expires_at, status, is_active, created_at, last_sync_at, quota_total, quota_used FROM accounts WHERE email = ?1",
    )?;
    let mut rows = stmt.query(params![email])?;
    match rows.next()? {
        Some(row) => Ok(Some(row_to_account(row)?)),
        None => Ok(None),
    }
}

pub fn list_accounts(db: &Database) -> Result<Vec<Account>, DbError> {
    let conn = db.conn();
    let mut stmt = conn.prepare(
        "SELECT id, email, display_name, avatar_url, scope, token_expires_at, status, is_active, created_at, last_sync_at, quota_total, quota_used FROM accounts ORDER BY created_at ASC",
    )?;
    let rows = stmt.query_map([], row_to_account)?;
    let mut accounts = Vec::new();
    for row in rows {
        accounts.push(row?);
    }
    Ok(accounts)
}

pub fn update_account(db: &Database, account: &Account) -> Result<(), DbError> {
    let conn = db.conn();
    let affected = conn.execute(
        "UPDATE accounts SET email = ?2, display_name = ?3, avatar_url = ?4, scope = ?5, token_expires_at = ?6, status = ?7, is_active = ?8, last_sync_at = ?9, quota_total = ?10, quota_used = ?11 WHERE id = ?1",
        params![
            account.id,
            account.email,
            account.display_name,
            account.avatar_url,
            account.scope,
            account.token_expires_at,
            status_to_str(&account.status),
            account.is_active as i32,
            account.last_sync_at,
            account.quota_total,
            account.quota_used,
        ],
    )?;
    if affected == 0 {
        return Err(DbError::AccountNotFound(account.id.clone()));
    }
    Ok(())
}

pub fn delete_account(db: &Database, id: &str) -> Result<(), DbError> {
    let conn = db.conn();
    let affected = conn.execute("DELETE FROM accounts WHERE id = ?1", params![id])?;
    if affected == 0 {
        return Err(DbError::AccountNotFound(id.to_string()));
    }
    Ok(())
}

pub fn set_active_account(db: &Database, id: &str) -> Result<(), DbError> {
    let conn = db.conn();
    conn.execute("UPDATE accounts SET is_active = 0", [])?;
    let affected =
        conn.execute("UPDATE accounts SET is_active = 1 WHERE id = ?1", params![id])?;
    if affected == 0 {
        return Err(DbError::AccountNotFound(id.to_string()));
    }
    Ok(())
}
