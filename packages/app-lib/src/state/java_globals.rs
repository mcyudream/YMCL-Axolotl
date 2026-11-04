use serde::{Deserialize, Serialize};
use sqlx::Row;

#[derive(Debug, PartialEq, Eq, Hash, Serialize, Deserialize, Clone)]
pub struct JavaVersion {
    pub parsed_version: u32,
    pub version: String,
    pub architecture: String,
    pub path: String,
    pub distribution: Option<String>,
}

impl JavaVersion {
    pub async fn get(
        major_version: u32,
        exec: impl sqlx::Executor<'_, Database = sqlx::Sqlite>,
    ) -> crate::Result<Option<JavaVersion>> {
        let row = sqlx::query(
            "
            SELECT
                java_versions.major_version,
                java_versions.full_version,
                java_versions.architecture,
                java_versions.path,
                java_versions.distribution
            FROM java_default_versions
            INNER JOIN java_versions
                ON java_versions.major_version = java_default_versions.major_version
                AND java_versions.path = java_default_versions.path
            WHERE java_default_versions.major_version = $1
            ",
        )
        .bind(major_version as i64)
        .fetch_optional(exec)
        .await?;

        Ok(row.map(|row| JavaVersion {
            parsed_version: row.get::<i64, _>("major_version") as u32,
            version: row.get("full_version"),
            architecture: row.get("architecture"),
            path: crate::portable::resolve(row.get("path")),
            distribution: row.get("distribution"),
        }))
    }

    pub async fn get_all_defaults(
        exec: impl sqlx::Executor<'_, Database = sqlx::Sqlite>,
    ) -> crate::Result<Vec<Self>> {
        let rows = sqlx::query(
            "
            SELECT
                java_versions.major_version,
                java_versions.full_version,
                java_versions.architecture,
                java_versions.path,
                java_versions.distribution
            FROM java_default_versions
            INNER JOIN java_versions
                ON java_versions.major_version = java_default_versions.major_version
                AND java_versions.path = java_default_versions.path
            ORDER BY java_versions.major_version DESC
            ",
        )
        .fetch_all(exec)
        .await?;

        Ok(rows
            .into_iter()
            .map(|row| JavaVersion {
                parsed_version: row.get::<i64, _>("major_version") as u32,
                version: row.get("full_version"),
                architecture: row.get("architecture"),
                path: crate::portable::resolve(row.get("path")),
                distribution: row.get("distribution"),
            })
            .collect())
    }

    pub async fn get_all(
        exec: impl sqlx::Executor<'_, Database = sqlx::Sqlite>,
    ) -> crate::Result<Vec<Self>> {
        let rows = sqlx::query!(
            r#"SELECT major_version, full_version, architecture, path, distribution as "distribution?: String" FROM java_versions"#
        )
        .fetch_all(exec)
        .await?;

        Ok(rows
            .into_iter()
            .map(|x| JavaVersion {
                parsed_version: x.major_version as u32,
                version: x.full_version,
                architecture: x.architecture,
                path: crate::portable::resolve(&x.path),
                distribution: x.distribution,
            })
            .collect())
    }

    pub async fn upsert(
        &self,
        exec: impl sqlx::Executor<'_, Database = sqlx::Sqlite>,
    ) -> crate::Result<()> {
        let major_version = self.parsed_version as i32;
        let path = crate::portable::store(&self.path);

        sqlx::query!(
            "
            INSERT INTO java_versions (major_version, full_version, architecture, path, distribution)
            VALUES ($1, $2, $3, $4, $5)
            ON CONFLICT (path) DO UPDATE SET
                major_version = $1,
                full_version = $2,
                architecture = $3,
                distribution = $5
            ",
            major_version,
            self.version,
            self.architecture,
            path,
            self.distribution,
        )
            .execute(exec)
            .await?;

        Ok(())
    }

    pub async fn set_default(
        major_version: u32,
        path: &str,
        exec: impl sqlx::Executor<'_, Database = sqlx::Sqlite>,
    ) -> crate::Result<()> {
        sqlx::query(
            "
            INSERT INTO java_default_versions (major_version, path)
            VALUES ($1, $2)
            ON CONFLICT (major_version) DO UPDATE SET path = $2
            ",
        )
        .bind(major_version as i64)
        .bind(crate::portable::store(path))
        .execute(exec)
        .await?;

        Ok(())
    }

    pub async fn remove_default(
        major_version: u32,
        exec: impl sqlx::Executor<'_, Database = sqlx::Sqlite>,
    ) -> crate::Result<()> {
        sqlx::query(
            "DELETE FROM java_default_versions WHERE major_version = $1",
        )
        .bind(major_version as i64)
        .execute(exec)
        .await?;

        Ok(())
    }

    pub async fn remove_default_for_path(
        path: &str,
        exec: impl sqlx::Executor<'_, Database = sqlx::Sqlite>,
    ) -> crate::Result<()> {
        sqlx::query("DELETE FROM java_default_versions WHERE path = $1")
            .bind(crate::portable::store(path))
            .execute(exec)
            .await?;

        Ok(())
    }

    pub async fn delete(
        path: &str,
        exec: impl sqlx::Executor<'_, Database = sqlx::Sqlite>,
    ) -> crate::Result<()> {
        let path = crate::portable::store(path);
        sqlx::query!("DELETE FROM java_versions WHERE path = $1", path)
            .execute(exec)
            .await?;
        Ok(())
    }

    pub async fn update_path(
        old_path: &str,
        new_path: &str,
        exec: impl sqlx::Executor<'_, Database = sqlx::Sqlite>,
    ) -> crate::Result<()> {
        sqlx::query("UPDATE java_versions SET path = $1 WHERE path = $2")
            .bind(crate::portable::store(new_path))
            .bind(crate::portable::store(old_path))
            .execute(exec)
            .await?;

        Ok(())
    }

    pub async fn remove(
        major_version: u32,
        exec: impl sqlx::Executor<'_, Database = sqlx::Sqlite>,
    ) -> crate::Result<()> {
        let version = major_version as i32;
        sqlx::query("DELETE FROM java_versions WHERE major_version = $1")
            .bind(version)
            .execute(exec)
            .await?;
        Ok(())
    }
}
