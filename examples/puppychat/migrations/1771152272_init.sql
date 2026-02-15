-- name: init
-- created_at: 1771152272

BEGIN;

CREATE TABLE IF NOT EXISTS "Message" (id INTEGER PRIMARY KEY AUTOINCREMENT, "author" TEXT, "body" TEXT, "image_url" TEXT, "time" TEXT, "channel_id" INTEGER, "dm_thread_key" TEXT);

CREATE TABLE IF NOT EXISTS "Channel" (id INTEGER PRIMARY KEY AUTOINCREMENT, "name" TEXT, "display_name" TEXT, "messages" TEXT);

CREATE TABLE IF NOT EXISTS "DirectMessage" (id INTEGER PRIMARY KEY AUTOINCREMENT, "name" TEXT, "display_name" TEXT, "online" INTEGER, "messages" TEXT);

CREATE TABLE IF NOT EXISTS "User" (id INTEGER PRIMARY KEY AUTOINCREMENT, "name" TEXT, "password" TEXT);

CREATE TABLE IF NOT EXISTS "Session" (id INTEGER PRIMARY KEY AUTOINCREMENT, "session_key" TEXT, "user_name" TEXT);

COMMIT;
