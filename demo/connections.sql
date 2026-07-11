CREATE TABLE connections (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  name TEXT NOT NULL UNIQUE,
  type TEXT NOT NULL CHECK( type IN ('postgres','sqlite') ),
  url TEXT NOT NULL,
  environment TEXT NOT NULL CHECK( environment IN ('local', 'dev','staging','prod') ),
  metadata TEXT
);
PRAGMA user_version = 1;

INSERT INTO connections (name, type, url, environment, metadata) VALUES
  ('demo-postgres', 'postgres', 'postgres://d7s_user@localhost:5432/d7s_test', 'dev',
   '{"password_storage":"dont_save"}'),
  ('demo-sqlite', 'sqlite', 'sample.db', 'dev', '{}');
