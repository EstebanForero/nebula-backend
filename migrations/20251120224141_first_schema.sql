CREATE TABLE users (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    username        TEXT NOT NULL UNIQUE,
    email           TEXT NOT NULL UNIQUE,
    password_hash   TEXT NOT NULL,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE rooms (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name            TEXT NOT NULL,
    visibility      TEXT NOT NULL DEFAULT 'public',
    password_hash   TEXT NULL,                     -- only for private rooms if using password
    created_by      UUID NOT NULL REFERENCES users(id) ON DELETE RESTRICT,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),

    -- Optional: enforce only these values
    CONSTRAINT chk_room_visibility
        CHECK (visibility IN ('public', 'private'))
);

CREATE TABLE room_members (
    room_id     UUID NOT NULL REFERENCES rooms(id) ON DELETE CASCADE,
    user_id     UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    role        TEXT NOT NULL DEFAULT 'member',
    joined_at   TIMESTAMPTZ NOT NULL DEFAULT now(),

    PRIMARY KEY (room_id, user_id),

    
    CONSTRAINT chk_room_member_role
        CHECK (role IN ('owner', 'member'))
);

CREATE TABLE messages (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    room_id     UUID NOT NULL REFERENCES rooms(id) ON DELETE CASCADE,
    sender_id   UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    content     TEXT NOT NULL,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- Indexes

CREATE INDEX idx_messages_room_created_at
    ON messages (room_id, created_at DESC);

CREATE INDEX idx_messages_room_id
    ON messages (room_id);

CREATE TABLE room_read_state (
    room_id              UUID NOT NULL REFERENCES rooms(id) ON DELETE CASCADE,
    user_id              UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    last_read_message_id UUID NULL REFERENCES messages(id) ON DELETE SET NULL,
    last_read_at         TIMESTAMPTZ NOT NULL DEFAULT now(),

    PRIMARY KEY (room_id, user_id)
);

