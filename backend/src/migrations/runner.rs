use mongodb::{Database, bson::{doc, DateTime as MongoDateTime}};
use anyhow::Result;
use crate::migrations::{Migration, all_migrations};

pub async fn run_migrations(db: &Database) -> Result<()> {
    let migrations_coll = db.collection("migrations");
    let mut applied_versions = vec![];
    let cursor = migrations_coll.find(None, None).await?;
    for result in cursor {
        if let Ok(doc) = result {
            if let Some(version) = doc.get_i64("version").ok() {
                applied_versions.push(version);
            }
        }
    }
    let migrations = all_migrations();
    for migration in migrations {
        if !applied_versions.contains(&migration.version()) {
            println!("Applying migration {}: {}", migration.version(), migration.name());
            migration.up(db).await?;
            migrations_coll.insert_one(doc! {
                "version": migration.version(),
                "name": migration.name(),
                "applied_at": MongoDateTime::now()
            }, None).await?;
        }
    }
    Ok(())
}
