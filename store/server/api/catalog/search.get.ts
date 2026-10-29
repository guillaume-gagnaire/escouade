import { getQuery } from 'h3';
import { catalog, search, type Query } from '../../utils/listing';
import { service } from '../../utils/service';

// The search, on the server (decided on 2026-10-29): the words of `q` in the names, keywords, categories, authors and
// descriptions, accents and case aside; the filters of the page; the sort (relevance, installations, recent).
export default defineEventHandler(async (event) => {
  const q = getQuery(event);
  const on = (v: unknown) => v === '1' || v === 'on' || v === 'true';
  const sort = ['relevance', 'installs', 'recent'].includes(String(q.sort)) ? (String(q.sort) as Query['sort']) : undefined;
  return search(await catalog(service.db), {
    q: typeof q.q === 'string' ? q.q : '',
    cat: typeof q.cat === 'string' ? q.cat : undefined,
    verified: on(q.verified),
    noProgram: on(q.noprog),
    noNetwork: on(q.nonet),
    noSecret: on(q.nosecret),
    sort,
  });
});
