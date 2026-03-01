-- name: push_subscriptions
-- created_at: 1772100000

BEGIN;

CREATE TABLE IF NOT EXISTS "PushSubscription" (
	id INTEGER PRIMARY KEY AUTOINCREMENT,
	"user_name" TEXT,
	"channel" TEXT,
	"endpoint" TEXT,
	"p256dh_key" TEXT,
	"auth_key" TEXT,
	"active" INTEGER,
	"updated_at" TEXT
);

CREATE UNIQUE INDEX IF NOT EXISTS "push_subscription_unique_endpoint"
ON "PushSubscription" ("channel", "endpoint");

COMMIT;
