ALTER TABLE "plugins" ADD COLUMN "path" text DEFAULT '' NOT NULL;--> statement-breakpoint
ALTER TABLE "plugins" ADD COLUMN "source_changed_at" timestamp with time zone;--> statement-breakpoint
ALTER TABLE "plugins" ADD COLUMN "source_from" text;--> statement-breakpoint
ALTER TABLE "versions" ADD COLUMN "path" text DEFAULT '' NOT NULL;