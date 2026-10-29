CREATE TABLE "installs_daily" (
	"plugin_id" text NOT NULL,
	"version" text NOT NULL,
	"day" date NOT NULL,
	"event" text NOT NULL,
	"count" integer DEFAULT 0 NOT NULL,
	CONSTRAINT "installs_daily_plugin_id_version_day_event_pk" PRIMARY KEY("plugin_id","version","day","event")
);
--> statement-breakpoint
CREATE TABLE "revocation_lists" (
	"serial" integer PRIMARY KEY NOT NULL,
	"json" text NOT NULL,
	"minisig" text NOT NULL,
	"issued_at" timestamp with time zone NOT NULL,
	"received_at" timestamp with time zone DEFAULT now() NOT NULL
);
