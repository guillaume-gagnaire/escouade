CREATE TABLE "discord_quota" (
	"channel" text NOT NULL,
	"login" text NOT NULL,
	"day" date NOT NULL,
	"sent" integer DEFAULT 0 NOT NULL,
	"skipped" jsonb DEFAULT '[]'::jsonb NOT NULL,
	CONSTRAINT "discord_quota_channel_login_day_pk" PRIMARY KEY("channel","login","day")
);
