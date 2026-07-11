CREATE TABLE users (
  id INTEGER PRIMARY KEY,
  username TEXT NOT NULL,
  email TEXT NOT NULL,
  full_name TEXT
);
CREATE TABLE orders (
  id INTEGER PRIMARY KEY,
  user_id INTEGER NOT NULL REFERENCES users(id),
  total REAL NOT NULL,
  status TEXT NOT NULL
);
INSERT INTO users (username, email, full_name) VALUES
  ('ada', 'ada@example.com', 'Ada Lovelace'),
  ('grace', 'grace@example.com', 'Grace Hopper'),
  ('alan', 'alan@example.com', 'Alan Turing'),
  ('kathy', 'kathy@example.com', 'Katherine Johnson');
INSERT INTO orders (user_id, total, status) VALUES
  (1, 42.50, 'paid'),
  (1, 13.00, 'pending'),
  (2, 99.99, 'paid'),
  (3, 7.25, 'cancelled'),
  (4, 120.00, 'paid');
