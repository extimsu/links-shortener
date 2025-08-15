// Migration trait and registry for MongoDB migrations
use mongodb::Database;
use anyhow::Result;

#[async_trait::async_trait]
pub trait Migration {
    fn version(&self) -> i64;
    fn name(&self) -> &'static str;
    async fn up(&self, db: &Database) -> Result<()>;
    async fn down(&self, db: &Database) -> Result<()>;
}

pub mod scripts;

// Registry of all migrations
pub fn all_migrations() -> Vec<Box<dyn Migration + Send + Sync>> {
    vec![
        Box::new(scripts::m001_initial_setup::InitialSetup),
        Box::new(scripts::m002_create_indexes::CreateIndexes),
        Box::new(scripts::m003_seed_data::SeedData),
        // Add more migrations here as needed
    ]
}
