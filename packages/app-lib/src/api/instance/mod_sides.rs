use crate::state::{
    ContentItem, ContentProviderRef, ModSide, ModSideInfo, ModSideSource,
    ProjectType, State,
};
use serde::{Deserialize, Serialize};
use sqlx::{Row, SqlitePool};
use std::collections::HashMap;

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModSideScope {
    Instance,
    Local,
}

// Project rules survive upgrades and file renames. The loader isolates
// Forge/Fabric variants, whose supported environments may differ.
fn identities(item: &ContentItem) -> Vec<String> {
    let loader = item.loader.as_deref().unwrap_or("unknown");
    let mut keys = item
        .provider_refs
        .iter()
        .map(|reference| match reference {
            ContentProviderRef::Modrinth { project_id, .. } => {
                format!("modrinth:{project_id}:{loader}")
            }
            ContentProviderRef::CurseForge { project_id, .. } => {
                format!("curseforge:{}:{loader}", project_id.get())
            }
            ContentProviderRef::McArchive { project_id, .. } => {
                format!("mcarchive:{project_id}:{loader}")
            }
        })
        .collect::<Vec<_>>();
    if let Some(project) = item.project.as_ref()
        && project.id.starts_with("local:")
        && !project.id.starts_with("local:file:")
    {
        keys.push(format!("{}:{loader}", project.id));
    }
    keys.push(format!("sha1:{}:{loader}", item.id));
    keys
}

pub(crate) async fn hydrate_mod_sides(
    instance_id: &str,
    items: &mut [ContentItem],
    pool: &SqlitePool,
) -> crate::Result<()> {
    let rows = sqlx::query(
        "SELECT scope, identity, side FROM mod_side_overrides WHERE scope IN ('', ?)",
    )
    .bind(instance_id)
    .fetch_all(pool)
    .await?;
    let mut rules = HashMap::new();
    for row in rows {
        let side: String = row.try_get("side")?;
        let side: ModSide =
            serde_json::from_value(serde_json::Value::String(side))?;
        rules.insert(
            (
                row.try_get::<String, _>("scope")?,
                row.try_get::<String, _>("identity")?,
            ),
            side,
        );
    }
    let mut detections = HashMap::new();
    let mod_ids = items
        .iter()
        .filter(|item| item.project_type == ProjectType::Mod)
        .map(|item| item.id.as_str())
        .collect::<Vec<_>>();
    for chunk in mod_ids.chunks(500) {
        let mut query = sqlx::QueryBuilder::<sqlx::Sqlite>::new(
            "SELECT sha1, metadata FROM mod_side_detections WHERE sha1 IN (",
        );
        let mut separated = query.separated(", ");
        for id in chunk {
            separated.push_bind(id);
        }
        separated.push_unseparated(")");
        for row in query.build().fetch_all(pool).await? {
            let data: String = row.try_get("metadata")?;
            if let Ok(info) = serde_json::from_str::<ModSideInfo>(&data) {
                detections.insert(row.try_get::<String, _>("sha1")?, info);
            }
        }
    }
    let mut changed_detections = Vec::new();
    for item in items
        .iter_mut()
        .filter(|item| item.project_type == ProjectType::Mod)
    {
        let mut detected = item.mod_side.clone().unwrap_or_default();
        let cached = detections.get(&item.id);
        if matches!(
            detected.source,
            ModSideSource::Unknown | ModSideSource::Embedded
        ) {
            if let Some(cached) = cached.filter(|info| {
                matches!(
                    info.source,
                    ModSideSource::Modrinth | ModSideSource::Curseforge
                )
            }) {
                detected = cached.clone();
            }
        } else if matches!(
            detected.source,
            ModSideSource::Modrinth | ModSideSource::Curseforge
        ) && cached != Some(&detected)
        {
            changed_detections
                .push((item.id.clone(), serde_json::to_string(&detected)?));
        }
        let keys = identities(item);
        item.mod_side = Some(
            [
                (instance_id, ModSideSource::Instance),
                ("", ModSideSource::Local),
            ]
            .into_iter()
            .find_map(|(scope, source)| {
                keys.iter().find_map(|key| {
                    rules
                        .get(&(scope.to_string(), key.clone()))
                        .map(|side| ModSideInfo::manual(*side, source))
                })
            })
            .unwrap_or(detected),
        );
    }
    if !changed_detections.is_empty() {
        let mut tx = pool.begin().await?;
        for (id, data) in changed_detections {
            sqlx::query(
                "INSERT INTO mod_side_detections (sha1, metadata) VALUES (?, ?)
                 ON CONFLICT(sha1) DO UPDATE SET metadata = excluded.metadata",
            )
            .bind(id)
            .bind(data)
            .execute(&mut *tx)
            .await?;
        }
        tx.commit().await?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item() -> ContentItem {
        serde_json::from_value(serde_json::json!({
            "file_name": "example.jar", "file_path": "mods/example.jar", "id": "file-hash",
            "size": 1, "enabled": true, "project_type": "mod", "provider_refs": [{
                "provider": "curseforge", "project_id": 123, "file_id": 456
            }], "external": false, "loader": "forge",
        })).unwrap()
    }

    async fn pool() -> SqlitePool {
        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .unwrap();
        sqlx::raw_sql(include_str!(
            "../../../migrations/20261123120000_mod-sides.sql"
        ))
        .execute(&pool)
        .await
        .unwrap();
        pool
    }

    #[tokio::test]
    async fn scoped_overrides_survive_upgrade_without_affecting_other_loaders()
    {
        let pool = pool().await;
        sqlx::query("INSERT INTO mod_side_overrides VALUES ('', 'curseforge:123:forge', 'client'), ('pack-a', 'curseforge:123:forge', 'server')")
            .execute(&pool).await.unwrap();
        let mut items = vec![item()];
        hydrate_mod_sides("pack-a", &mut items, &pool)
            .await
            .unwrap();
        assert_eq!(
            items[0].mod_side.as_ref().unwrap().source,
            ModSideSource::Instance
        );
        assert_eq!(items[0].mod_side.as_ref().unwrap().side, ModSide::Server);
        items[0].id = "new-version-hash".into();
        items[0].file_path = "mods/renamed-new-version.jar".into();
        items[0].mod_side = None;
        hydrate_mod_sides("pack-b", &mut items, &pool)
            .await
            .unwrap();
        assert_eq!(
            items[0].mod_side.as_ref().unwrap().source,
            ModSideSource::Local
        );
        assert_eq!(items[0].mod_side.as_ref().unwrap().side, ModSide::Client);
        items[0].loader = Some("fabric".into());
        items[0].mod_side = None;
        hydrate_mod_sides("pack-b", &mut items, &pool)
            .await
            .unwrap();
        assert_eq!(items[0].mod_side.as_ref().unwrap().side, ModSide::Unknown);
        sqlx::query("DELETE FROM mod_side_overrides WHERE scope = 'pack-a'")
            .execute(&pool)
            .await
            .unwrap();
        items[0].loader = Some("forge".into());
        items[0].mod_side = None;
        hydrate_mod_sides("pack-a", &mut items, &pool)
            .await
            .unwrap();
        assert_eq!(
            items[0].mod_side.as_ref().unwrap().source,
            ModSideSource::Local
        );
    }

    #[tokio::test]
    async fn provider_detection_is_available_offline_and_manual_unknown_wins() {
        let pool = pool().await;
        let mut items = vec![item()];
        items[0].mod_side = Some(ModSideInfo::manual(
            ModSide::Client,
            ModSideSource::Curseforge,
        ));
        hydrate_mod_sides("pack", &mut items, &pool).await.unwrap();
        items[0].mod_side = None;
        hydrate_mod_sides("pack", &mut items, &pool).await.unwrap();
        assert_eq!(
            items[0].mod_side.as_ref().unwrap().source,
            ModSideSource::Curseforge
        );
        sqlx::query("INSERT INTO mod_side_overrides VALUES ('pack', 'curseforge:123:forge', 'unknown')")
            .execute(&pool).await.unwrap();
        items[0].mod_side = None;
        hydrate_mod_sides("pack", &mut items, &pool).await.unwrap();
        assert_eq!(items[0].mod_side.as_ref().unwrap().side, ModSide::Unknown);
        assert_eq!(
            items[0].mod_side.as_ref().unwrap().source,
            ModSideSource::Instance
        );
        items[0].id = "different-file".into();
        items[0].mod_side = None;
        hydrate_mod_sides("other-pack", &mut items, &pool)
            .await
            .unwrap();
        assert_eq!(
            items[0].mod_side.as_ref().unwrap().source,
            ModSideSource::Unknown
        );
    }
}

/// `None` removes only the chosen scope, revealing inherited/automatic data.
pub async fn set_mod_side(
    instance_id: &str,
    paths: Vec<String>,
    scope: ModSideScope,
    side: Option<ModSide>,
) -> crate::Result<Vec<ContentItem>> {
    let state = State::get().await?;
    let _guard = state.lock_instance_content(instance_id).await;
    let snapshot = super::get_content_snapshot(instance_id).await?;
    if paths.is_empty() {
        return Err(
            crate::ErrorKind::InputError("No mods selected".into()).into()
        );
    }
    let mut keys = Vec::new();
    for path in &paths {
        let item = snapshot
            .items
            .iter()
            .filter_map(|item| item.content.as_ref())
            .find(|item| {
                item.file_path == *path && item.project_type == ProjectType::Mod
            })
            .ok_or_else(|| {
                crate::ErrorKind::InputError(format!(
                    "Mod is no longer installed: {path}"
                ))
            })?;
        let mut item_keys = identities(item);
        // File hashes are a fallback identity. Do not leave a version-specific
        // override behind after a project's rule is removed on a newer release.
        if side.is_some() && item_keys.len() > 1 {
            item_keys.retain(|key| !key.starts_with("sha1:"));
        }
        keys.extend(item_keys);
    }
    keys.sort();
    keys.dedup();
    let content_set = crate::state::instances::adapters::sqlite::content_rows::get_applied_content_set(
        instance_id, &state.pool,
    ).await?.ok_or_else(|| crate::ErrorKind::InputError("Instance has no content set".into()))?;
    let affected_instances: Vec<String> =
        if matches!(scope, ModSideScope::Local) {
            sqlx::query_scalar(
            "SELECT id FROM instances WHERE applied_content_set_id IS NOT NULL",
        )
        .fetch_all(&state.pool)
        .await?
        } else {
            vec![instance_id.to_string()]
        };
    let scope = match scope {
        ModSideScope::Instance => instance_id,
        ModSideScope::Local => "",
    };
    let mut tx = state.pool.begin().await?;
    for key in keys {
        if let Some(side) = side {
            sqlx::query(
                "INSERT INTO mod_side_overrides (scope, identity, side) VALUES (?, ?, ?)
                 ON CONFLICT(scope, identity) DO UPDATE SET side = excluded.side",
            )
            .bind(scope)
            .bind(key)
            .bind(side.as_str())
            .execute(&mut *tx)
            .await?;
        } else {
            sqlx::query("DELETE FROM mod_side_overrides WHERE scope = ? AND identity = ?")
                .bind(scope)
                .bind(key)
                .execute(&mut *tx)
                .await?;
        }
    }
    if scope.is_empty() {
        sqlx::query("UPDATE instance_content_sets SET revision = revision + 1, modified = ? WHERE id IN (SELECT applied_content_set_id FROM instances)")
            .bind(chrono::Utc::now().timestamp()).execute(&mut *tx).await?;
    } else {
        crate::state::instances::adapters::sqlite::content_rows::bump_content_set_revision_in_transaction(
            &content_set.id, &mut tx,
        ).await?;
    }
    tx.commit().await?;
    for affected in affected_instances {
        super::emit_content_changed(&affected).await?;
    }
    Ok(super::get_content_snapshot(instance_id)
        .await?
        .items
        .into_iter()
        .filter_map(|item| item.content)
        .filter(|item| item.project_type == ProjectType::Mod)
        .collect())
}
