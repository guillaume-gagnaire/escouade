CREATE TABLE "plugins" (
	"id" text PRIMARY KEY NOT NULL,
	"account_id" integer,
	"repo_id" bigint NOT NULL,
	"repo_full_name" text NOT NULL,
	"installation_id" bigint,
	"categories" jsonb DEFAULT '[]'::jsonb NOT NULL,
	"keywords" jsonb DEFAULT '[]'::jsonb NOT NULL,
	"auto_publish" boolean DEFAULT false NOT NULL,
	"created_at" timestamp with time zone DEFAULT now() NOT NULL
);
--> statement-breakpoint
CREATE TABLE "versions" (
	"id" serial PRIMARY KEY NOT NULL,
	"plugin_id" text,
	"account_id" integer,
	"origin" text DEFAULT 'author' NOT NULL,
	"installation_id" bigint NOT NULL,
	"repo_id" bigint NOT NULL,
	"repo_full_name" text NOT NULL,
	"tag" text NOT NULL,
	"commit" text NOT NULL,
	"tag_date" timestamp with time zone,
	"version" text,
	"state" text DEFAULT 'checking' NOT NULL,
	"step" text,
	"error" text,
	"checks" jsonb DEFAULT '[]'::jsonb NOT NULL,
	"manifest" jsonb,
	"manifest_sha256" text,
	"archive_sha256" text,
	"archive_size" integer,
	"binaries" jsonb,
	"screenshots" jsonb,
	"version_json" text,
	"version_sha256" text,
	"review" boolean DEFAULT true NOT NULL,
	"created_at" timestamp with time zone DEFAULT now() NOT NULL,
	"checked_at" timestamp with time zone,
	"published_at" timestamp with time zone,
	"tag_moved_at" timestamp with time zone,
	"tag_moved_to" text
);
--> statement-breakpoint
ALTER TABLE "plugins" ADD CONSTRAINT "plugins_account_id_accounts_id_fk" FOREIGN KEY ("account_id") REFERENCES "public"."accounts"("id") ON DELETE set null ON UPDATE no action;--> statement-breakpoint
ALTER TABLE "versions" ADD CONSTRAINT "versions_account_id_accounts_id_fk" FOREIGN KEY ("account_id") REFERENCES "public"."accounts"("id") ON DELETE set null ON UPDATE no action;--> statement-breakpoint
CREATE INDEX "plugins_account" ON "plugins" USING btree ("account_id");--> statement-breakpoint
CREATE INDEX "plugins_repo" ON "plugins" USING btree ("repo_id");--> statement-breakpoint
CREATE INDEX "versions_plugin" ON "versions" USING btree ("plugin_id");--> statement-breakpoint
CREATE INDEX "versions_account" ON "versions" USING btree ("account_id");--> statement-breakpoint
CREATE INDEX "versions_repo_tag" ON "versions" USING btree ("repo_id","tag");--> statement-breakpoint
CREATE UNIQUE INDEX "versions_published" ON "versions" USING btree ("plugin_id","version") WHERE "versions"."state" = 'published';