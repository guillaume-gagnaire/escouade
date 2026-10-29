ALTER TABLE "plugins" ADD COLUMN "unlisted_at" timestamp with time zone;--> statement-breakpoint
ALTER TABLE "plugins" ADD COLUMN "deleted_at" timestamp with time zone;--> statement-breakpoint
ALTER TABLE "plugins" ADD COLUMN "purged_at" timestamp with time zone;--> statement-breakpoint
ALTER TABLE "plugins" ADD COLUMN "retired_versions" jsonb DEFAULT '[]'::jsonb NOT NULL;--> statement-breakpoint
ALTER TABLE "plugins" ADD COLUMN "transfer_to" integer;--> statement-breakpoint
ALTER TABLE "plugins" ADD COLUMN "transfer_asked_at" timestamp with time zone;--> statement-breakpoint
ALTER TABLE "plugins" ADD COLUMN "transferred_at" timestamp with time zone;--> statement-breakpoint
ALTER TABLE "plugins" ADD COLUMN "transferred_from" text;--> statement-breakpoint
ALTER TABLE "plugins" ADD COLUMN "frozen_until" timestamp with time zone;--> statement-breakpoint
ALTER TABLE "plugins" ADD COLUMN "repo_moved_at" timestamp with time zone;--> statement-breakpoint
ALTER TABLE "versions" ADD COLUMN "yanked_at" timestamp with time zone;--> statement-breakpoint
ALTER TABLE "plugins" ADD CONSTRAINT "plugins_transfer_to_accounts_id_fk" FOREIGN KEY ("transfer_to") REFERENCES "public"."accounts"("id") ON DELETE set null ON UPDATE no action;