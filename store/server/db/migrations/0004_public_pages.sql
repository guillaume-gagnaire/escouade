CREATE TABLE "reports" (
	"id" serial PRIMARY KEY NOT NULL,
	"plugin_id" text NOT NULL,
	"version" text,
	"reason" text NOT NULL,
	"details" text NOT NULL,
	"email" text NOT NULL,
	"lang" text DEFAULT 'fr' NOT NULL,
	"state" text DEFAULT 'open' NOT NULL,
	"created_at" timestamp with time zone DEFAULT now() NOT NULL
);
--> statement-breakpoint
ALTER TABLE "versions" ADD COLUMN "docs" jsonb;--> statement-breakpoint
CREATE INDEX "reports_plugin" ON "reports" USING btree ("plugin_id");--> statement-breakpoint
CREATE INDEX "reports_state" ON "reports" USING btree ("state");