CREATE TABLE IF NOT EXISTS User
(
    id           INTEGER PRIMARY KEY AUTOINCREMENT,
    wrong_pass   INTEGER DEFAULT 0,
    next_attempt INTEGER DEFAULT 0,
    name         TEXT    DEFAULT '',
    username     TEXT    DEFAULT '',
    password     TEXT    DEFAULT '',
    permissions  TEXT    DEFAULT ''
);
-- system user can't auth forever
INSERT INTO User (id, name, username, permissions, next_attempt)
VALUES (0, 'SYSTEM', 'SYSTEM', '*', 9223372036854775807);
INSERT INTO User (id, name, username, permissions)
VALUES (1, 'Admin', 'admin', '*');