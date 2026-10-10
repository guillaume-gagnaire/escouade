import { defineZone } from '../types';

// The errors the backend sends as codes, written here.
export default defineZone('errors', {
  notFound: '{path} not found',
});
