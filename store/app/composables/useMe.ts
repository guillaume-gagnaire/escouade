/** Who is signed in (the header, the account's page): their login, or null. Read once per page, on the server too. */
export function useMe() {
  return useFetch<{ account: { login: string; githubId: number; admin: boolean; queue?: number } | null }>('/api/me', {
    key: 'me',
    default: () => ({ account: null }),
  });
}
