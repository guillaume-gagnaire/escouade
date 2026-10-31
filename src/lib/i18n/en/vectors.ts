import type { Tree } from '../types';

// « Recherche par le sens » (ESC-254): the vectors’ model, one for every project of this machine, used by the shared memory and the docs.
export default {
  allTitle: 'Every project',
  label: 'Search by meaning',
  desc: 'Besides the words, the shared memory and the docs are searched by meaning, with a small multilingual model ({model}, {license} license) that runs on this machine. One download of {size}, for every project; after that, nothing leaves the machine.',
  download: 'Download ({size})',
  resume: 'Resume ({pct}%)',
  cancel: 'Cancel',
  downloaded: 'Downloaded',
  delete: 'Delete',
  deleteTitle: 'Delete the model ({size}): the search goes by the words alone until the next download.',
  progress: 'Downloading the model',
} as const satisfies Tree;
