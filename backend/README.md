# Backend Database Setup & Migration Guide

## MongoDB Schema & Migrations

This backend uses MongoDB for storing URL mappings and analytics. All schema, indexes, and initial data are managed via a Rust-based migration system.

### Features

- **Schema validation** for `urls` and `analytics` collections
- **Automatic index creation** for performance
- **Migration runner** with up/down support and version tracking
- **Seed scripts** for development/test data
- **Configurable connection pooling**

---

## Running Migrations

Migrations are managed in `src/migrations/` and are applied automatically at startup or can be run manually.

**To run migrations at startup:**

- The migration runner is invoked in `main.rs` before the server starts.
- All pending migrations are applied in order.

**To run migrations manually (example):**

```rust
// In your main.rs or a CLI entrypoint
use backend::migrations::runner::run_migrations;
// ...
let client = Client::with_options(client_options)?;
let db = client.database("shortener");
run_migrations(&db).await?;
```

---

## Configuring Connection Pooling

The MongoDB client uses a connection pool for efficient access. Pool settings are configurable via environment variables:

- `MONGODB_MAX_POOL_SIZE` (default: 20)
- `MONGODB_MIN_POOL_SIZE` (default: 0)
- `MONGODB_MAX_IDLE_TIME_MS` (default: 300000)
- `MONGODB_CONNECT_TIMEOUT_MS` (default: 10000)

Set these in your `.env` file or CI/CD environment as needed.

---

## Seeding the Database

A seed migration (`m003_seed_data`) inserts sample URLs and analytics data for development/testing. You can add more seed scripts as needed in `src/migrations/scripts/`.

---

## CI/CD Integration

- The migration runner can be invoked as part of your deployment pipeline to ensure the database is always up to date.
- Example: Add a migration step before starting the backend service.

---

## Adding New Migrations

1. Create a new script in `src/migrations/scripts/` (e.g., `m004_new_feature.rs`).
2. Implement the `Migration` trait with `up` and `down` methods.
3. Register the migration in `src/migrations/mod.rs`.

---

## Troubleshooting

- If migrations fail, check the logs for detailed error messages.
- The `migrations` collection in MongoDB tracks applied migrations and their timestamps.
- To rollback, implement the `down` method in your migration scripts and invoke as needed.

---

For further details, see code comments in the migration and main files.

---

## MongoDB Backup & Restore Procedures (Production)

### Data Persistence
- MongoDB data is stored in a named Docker volume (`mongodata`).
- Data persists across container restarts and stack upgrades.

### Backup (Snapshot)
To create a backup of the MongoDB data volume:

```sh
docker exec <mongo_container_name> mongodump --archive=/data/db/backup.archive
# Copy the backup to the host
docker cp <mongo_container_name>:/data/db/backup.archive ./backup.archive
```

### Restore
To restore from a backup:

```sh
docker cp ./backup.archive <mongo_container_name>:/data/db/backup.archive
docker exec <mongo_container_name> mongorestore --drop --archive=/data/db/backup.archive
```

### Notes
- Replace `<mongo_container_name>` with the actual container name (e.g., `links-shortener-mongo-1`).
- For automated scheduled backups, use a cron job or a backup container.
- Always test restores in a staging environment before production.
