import { catalog, home } from '../../utils/listing';
import { service } from '../../utils/service';

// The home page's lists (docs/STORE.md §6): the categories and their counts, the verified, the new, the most installed.
export default defineEventHandler(async () => home(await catalog(service.db)));
