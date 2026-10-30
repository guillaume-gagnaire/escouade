CREATE TABLE "admin_tokens" (
	"id" text PRIMARY KEY NOT NULL,
	"account_id" integer NOT NULL,
	"label" text NOT NULL,
	"hint" text NOT NULL,
	"created_at" timestamp with time zone DEFAULT now() NOT NULL,
	"used_at" timestamp with time zone,
	"revoked_at" timestamp with time zone
);
--> statement-breakpoint
CREATE TABLE "signatures" (
	"plugin_id" text NOT NULL,
	"version" text NOT NULL,
	"version_sha256" text NOT NULL,
	"minisig" text NOT NULL,
	"key_id" text NOT NULL,
	"signed_at" timestamp with time zone NOT NULL,
	"received_at" timestamp with time zone DEFAULT now() NOT NULL,
	"summary" jsonb,
	CONSTRAINT "signatures_plugin_id_version_pk" PRIMARY KEY("plugin_id","version")
);
--> statement-breakpoint
ALTER TABLE "reports" ADD COLUMN "closed_at" timestamp with time zone;--> statement-breakpoint
ALTER TABLE "reports" ADD COLUMN "answer" text;--> statement-breakpoint
ALTER TABLE "versions" ADD COLUMN "review_state" text;--> statement-breakpoint
ALTER TABLE "versions" ADD COLUMN "review_message" text;--> statement-breakpoint
ALTER TABLE "versions" ADD COLUMN "reviewed_at" timestamp with time zone;--> statement-breakpoint
ALTER TABLE "versions" ADD COLUMN "review_report" text;--> statement-breakpoint
ALTER TABLE "versions" ADD COLUMN "review_verdict" text;--> statement-breakpoint
ALTER TABLE "versions" ADD COLUMN "review_report_at" timestamp with time zone;--> statement-breakpoint
ALTER TABLE "admin_tokens" ADD CONSTRAINT "admin_tokens_account_id_accounts_id_fk" FOREIGN KEY ("account_id") REFERENCES "public"."accounts"("id") ON DELETE cascade ON UPDATE no action;--> statement-breakpoint
CREATE INDEX "admin_tokens_account" ON "admin_tokens" USING btree ("account_id");