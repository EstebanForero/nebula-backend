use sqlx::PgPool;

struct PostgresDatabase {
    pool: PgPool,
}
