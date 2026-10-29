import { pgTable, text, timestamp } from 'drizzle-orm/pg-core';

// The store's tables. A migration only adds (tables, columns, indexes): a new container can fail after it and leave
// the old one online, still reading the old names (docs/STORE.md, §1.3). The accounts, plugins and versions come
// with STORE-2 and STORE-3.

/** What the service notes about itself: the last beat of the queue (`queue.beat`), read by /api/health. */
export const serviceState = pgTable('service_state', {
  key: text('key').primaryKey(),
  at: timestamp('at', { withTimezone: true }).notNull().defaultNow(),
});
