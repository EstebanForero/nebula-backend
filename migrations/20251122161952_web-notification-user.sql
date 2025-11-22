-- Add migration script here

-- Up migration
ALTER TABLE users
    ADD COLUMN webpush_endpoint TEXT,
    ADD COLUMN webpush_p256dh   TEXT,
    ADD COLUMN webpush_auth     TEXT;

-- Down migration
ALTER TABLE users
    DROP COLUMN webpush_endpoint,
    DROP COLUMN webpush_p256dh,
    DROP COLUMN webpush_auth;
