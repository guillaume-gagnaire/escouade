import { defineConfig } from 'drizzle-kit';

// `npm run db:generate` writes the next migration from server/db/schema.ts; nothing here reaches a database.
export default defineConfig({
  dialect: 'postgresql',
  schema: './server/db/schema.ts',
  out: './server/db/migrations',
});
