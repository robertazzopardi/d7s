#!/bin/bash
set -e

echo "Initializing d7s test database..."

# Wait for Postgres to be ready
until pg_isready -U d7s_user -d d7s_test; do
    echo "Waiting for Postgres to be ready..."
    sleep 1
done

echo "Postgres is ready. Running seed script..."
# The seed.sql will be automatically run by Docker entrypoint
# This script is mainly for logging and any additional setup

echo "Database initialization complete!"

# Create additional databases for future multi-database support
echo "Creating additional databases..."

# Create databases
psql -v ON_ERROR_STOP=1 --username "$POSTGRES_USER" --dbname "$POSTGRES_DB" <<-EOSQL
    -- These databases are prepared for future multi-database support
    -- The app doesn't support this yet per connection, but we're setting up the infrastructure

    CREATE DATABASE test_db_1;
    CREATE DATABASE test_db_2;

    -- Grant permissions
    GRANT ALL PRIVILEGES ON DATABASE test_db_1 TO d7s_user;
    GRANT ALL PRIVILEGES ON DATABASE test_db_2 TO d7s_user;
EOSQL

# Setup test_db_1
echo "Setting up test_db_1..."
psql -v ON_ERROR_STOP=1 --username "$POSTGRES_USER" --dbname "test_db_1" <<-EOSQL
    CREATE SCHEMA IF NOT EXISTS test_schema;

    CREATE TABLE test_schema.test_table (
        id SERIAL PRIMARY KEY,
        name VARCHAR(100),
        created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
    );

    INSERT INTO test_schema.test_table (name) VALUES
        ('Test item 1 from DB 1'),
        ('Test item 2 from DB 1'),
        ('Test item 3 from DB 1');

    GRANT ALL PRIVILEGES ON ALL TABLES IN SCHEMA test_schema TO d7s_user;
    GRANT ALL PRIVILEGES ON ALL SEQUENCES IN SCHEMA test_schema TO d7s_user;
EOSQL

# Setup test_db_2
echo "Setting up test_db_2..."
psql -v ON_ERROR_STOP=1 --username "$POSTGRES_USER" --dbname "test_db_2" <<-EOSQL
    CREATE SCHEMA IF NOT EXISTS test_schema;

    CREATE TABLE test_schema.test_table (
        id SERIAL PRIMARY KEY,
        name VARCHAR(100),
        created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
    );

    INSERT INTO test_schema.test_table (name) VALUES
        ('Test item 1 from DB 2'),
        ('Test item 2 from DB 2');

    GRANT ALL PRIVILEGES ON ALL TABLES IN SCHEMA test_schema TO d7s_user;
    GRANT ALL PRIVILEGES ON ALL SEQUENCES IN SCHEMA test_schema TO d7s_user;
EOSQL

echo "Additional databases created successfully!"
echo "Test database setup is ready!"
