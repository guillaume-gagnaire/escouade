### claude-sonnet-5-5

60 runs · 10 tasks · model claude-sonnet-5-5 (medium) · commit 9632cde9

#### Totals by mode

| | grep | index-hook | tokensave |
|---|---|---|---|
| Runs (right) | 20 (20) | 20 (20) | 20 (20) |
| Tool calls | 457 | 456 | 382 |
| · Bash | 50 | 43 | 52 |
| · Edit | 251 | 249 | 132 |
| · Glob | 1 | 0 | 0 |
| · Grep | 73 | 61 | 50 |
| · Read | 82 | 80 | 51 |
| · index_body | 0 | 6 | 0 |
| · index_callers | 0 | 10 | 0 |
| · index_search | 0 | 7 | 0 |
| · tokensave_callers | 0 | 0 | 4 |
| · tokensave_multi_str_replace | 0 | 0 | 24 |
| · tokensave_read | 0 | 0 | 9 |
| · tokensave_search | 0 | 0 | 23 |
| · tokensave_str_replace | 0 | 0 | 37 |
| Grep after the index | 0 | 53 | 42 |
| Glob / Read after the index | 0 / 0 | 0 / 67 | 0 / 51 |
| Rereads | 27 | 23 | 7 |
| Places to reach | 374 | 374 | 374 |
| Missed at the first check (runs that checked) | 6 (20) | 4 (20) | 19 (20) |
| Missed at the end | 0 | 0 | 0 |
| Checks of the terminal (failed) | 41 (5) | 36 (2) | 39 (3) |
| Cycles check → fix | 10 | 9 | 10 |
| Commands refused | 18 | 14 | 26 |
| Runs completed after the index | 0 / 20 | 17 / 20 | 18 / 20 |
| Hook: searches seen / answered | 0 / 0 | 104 / 20 | 0 / 0 |
| Tokens: input | 0.3 k | 0.3 k | 0.3 k |
| · cache written | 281.3 k | 269.4 k | 308.5 k |
| · cache read | 2564.5 k | 3383.4 k | 4199.2 k |
| · output | 84.9 k | 83.4 k | 80.6 k |
| · total | 2931.1 k | 3736.6 k | 4588.6 k |
| Tokens of the tools' answers: the index's | 0.0 k | 10.0 k | 30.3 k |
| · Bash | 13.7 k | 9.2 k | 11.7 k |
| · Edit | 7.0 k | 7.2 k | 3.9 k |
| · Glob | 0.0 k | 0.0 k | 0.0 k |
| · Grep | 46.5 k | 36.0 k | 34.7 k |
| · Read | 28.5 k | 27.6 k | 23.3 k |
| · index_body | 0.0 k | 1.4 k | 0.0 k |
| · index_callers | 0.0 k | 6.7 k | 0.0 k |
| · index_search | 0.0 k | 1.9 k | 0.0 k |
| · tokensave_callers | 0.0 k | 0.0 k | 3.7 k |
| · tokensave_multi_str_replace | 0.0 k | 0.0 k | 1.4 k |
| · tokensave_read | 0.0 k | 0.0 k | 6.4 k |
| · tokensave_search | 0.0 k | 0.0 k | 16.4 k |
| · tokensave_str_replace | 0.0 k | 0.0 k | 2.4 k |
| Cost ($, Claude Code's count) | 2.232 | 2.251 | 2.460 |
| · against grep | +0 % | +1 % | +10 % |
| Time (s) | 3404 | 3116 | 3361 |
| Turns | 477 | 476 | 402 |

#### By task: right, calls, cost, against grep

| Task | grep | index-hook | tokensave |
|---|---|---|---|
| plugins-forget | 2/2 · 30 calls · 0.165 $ | 2/2 · 27 calls · 0.148 $ (-10 %) | 2/2 · 29 calls · 0.314 $ (+90 %) |
| drafts-files | 2/2 · 37 calls · 0.110 $ | 2/2 · 32 calls · 0.103 $ (-6 %) | 2/2 · 30 calls · 0.117 $ (+6 %) |
| checkout-of | 2/2 · 28 calls · 0.121 $ | 2/2 · 40 calls · 0.157 $ (+29 %) | 2/2 · 23 calls · 0.109 $ (-10 %) |
| account-skip | 2/2 · 39 calls · 0.212 $ | 2/2 · 43 calls · 0.193 $ (-9 %) | 2/2 · 46 calls · 0.279 $ (+32 %) |
| commit-root | 2/2 · 95 calls · 0.649 $ | 2/2 · 86 calls · 0.628 $ (-3 %) | 2/2 · 87 calls · 0.687 $ (+6 %) |
| i18n-unverified | 2/2 · 33 calls · 0.171 $ | 2/2 · 30 calls · 0.147 $ (-14 %) | 2/2 · 28 calls · 0.180 $ (+5 %) |
| fact-docs-gaps | 2/2 · 43 calls · 0.159 $ | 2/2 · 49 calls · 0.253 $ (+59 %) | 2/2 · 39 calls · 0.194 $ (+22 %) |
| app-env-struct | 2/2 · 51 calls · 0.227 $ | 2/2 · 39 calls · 0.215 $ (-5 %) | 2/2 · 41 calls · 0.264 $ (+16 %) |
| slugify-gone | 2/2 · 47 calls · 0.259 $ | 2/2 · 57 calls · 0.241 $ (-7 %) | 2/2 · 35 calls · 0.216 $ (-17 %) |
| missing-account-ts | 2/2 · 54 calls · 0.158 $ | 2/2 · 53 calls · 0.165 $ (+4 %) | 2/2 · 24 calls · 0.101 $ (-36 %) |

#### Every run

| Task | Mode | # | Right | Calls by tool | Grep after | Rereads | Tokens (in / cache w / cache r / out) | $ | s | Turns |
|---|---|---|---|---|---|---|---|---|---|---|
| plugins-forget | grep | 1 | yes | Bash 2, Edit 8, Grep 3, Read 6 | 0 | 3 | 18 / 8623 / 107429 / 3656 | 0.082 | 92 | 20 |
| plugins-forget | grep | 2 | yes | Bash 1, Edit 5, Grep 3, Read 2 | 0 | 1 | 14 / 12720 / 91233 / 2279 | 0.083 | 43 | 12 |
| plugins-forget | index-hook | 1 | yes | Bash 1, Edit 6, Grep 5, Read 2, index_callers 1 | 5 | 0 | 16 / 9062 / 147328 / 2619 | 0.077 | 99 | 16 |
| plugins-forget | index-hook | 2 | yes | Bash 1, Edit 5, Grep 5, index_callers 1 | 5 | 0 | 10 / 10336 / 91469 / 2027 | 0.071 | 38 | 13 |
| plugins-forget | tokensave | 1 | yes | Bash 2, Edit 5, Grep 3, Read 1, tokensave_callers 1, tokensave_search 1 | 3 | 0 | 14 / 31449 / 159561 / 3058 | 0.172 | 95 | 14 |
| plugins-forget | tokensave | 2 | yes | Bash 2, Edit 7, Grep 5, Read 1, tokensave_search 1 | 5 | 0 | 18 / 19597 / 240703 / 3868 | 0.141 | 50 | 17 |
| drafts-files | grep | 1 | yes | Bash 1, Edit 11, Grep 2 | 0 | 0 | 10 / 4845 / 43010 / 2233 | 0.046 | 39 | 15 |
| drafts-files | grep | 2 | yes | Bash 2, Edit 10, Grep 2, Read 9 | 0 | 2 | 12 / 5670 / 55354 / 3575 | 0.064 | 50 | 24 |
| drafts-files | index-hook | 1 | yes | Bash 1, Edit 8, Grep 1, index_search 1 | 1 | 0 | 10 / 4495 / 68872 / 1825 | 0.043 | 64 | 12 |
| drafts-files | index-hook | 2 | yes | Bash 1, Edit 8, Grep 2, Read 9, index_search 1 | 2 | 2 | 12 / 5050 / 89510 / 3098 | 0.060 | 50 | 22 |
| drafts-files | tokensave | 1 | yes | Bash 1, Edit 1, Grep 3, Read 1, tokensave_multi_str_replace 2, tokensave_read 1, tokensave_str_replace 4 | 1 | 0 | 10 / 6187 / 88685 / 2348 | 0.057 | 106 | 14 |
| drafts-files | tokensave | 2 | yes | Bash 3, Grep 3, tokensave_str_replace 11 | 1 | 0 | 12 / 4681 / 112596 / 2953 | 0.060 | 70 | 18 |
| checkout-of | grep | 1 | yes | Bash 1, Edit 11, Grep 2, Read 1 | 0 | 0 | 10 / 7922 / 45497 / 2896 | 0.065 | 61 | 16 |
| checkout-of | grep | 2 | yes | Bash 1, Edit 11, Grep 1 | 0 | 0 | 8 / 6421 / 35660 / 2678 | 0.056 | 53 | 14 |
| checkout-of | index-hook | 1 | yes | Bash 1, Edit 11, Grep 1, index_callers 1 | 1 | 0 | 8 / 8033 / 61092 / 2725 | 0.066 | 59 | 15 |
| checkout-of | index-hook | 2 | yes | Bash 2, Edit 13, Grep 1, Read 9, index_callers 1 | 1 | 4 | 14 / 9414 / 126838 / 4111 | 0.091 | 68 | 27 |
| checkout-of | tokensave | 1 | yes | Bash 1, Edit 11, Grep 1 | 0 | 0 | 8 / 6819 / 72150 / 2806 | 0.063 | 64 | 14 |
| checkout-of | tokensave | 2 | yes | Bash 2, Grep 3, tokensave_multi_str_replace 3, tokensave_str_replace 2 | 0 | 0 | 12 / 3099 / 115987 / 2217 | 0.046 | 85 | 11 |
| account-skip | grep | 1 | yes | Bash 2, Edit 14, Grep 3, Read 1 | 0 | 0 | 18 / 14937 / 120591 / 4226 | 0.114 | 69 | 21 |
| account-skip | grep | 2 | yes | Bash 2, Edit 12, Grep 2, Read 3 | 0 | 2 | 16 / 12092 / 101548 / 3932 | 0.098 | 64 | 20 |
| account-skip | index-hook | 1 | yes | Bash 2, Edit 15, Grep 1, index_body 3, index_callers 1 | 1 | 0 | 14 / 11770 / 120064 / 4005 | 0.099 | 67 | 23 |
| account-skip | index-hook | 2 | yes | Bash 1, Edit 14, Grep 3, Read 1, index_body 1, index_callers 1 | 3 | 0 | 14 / 10060 / 116734 / 4230 | 0.094 | 66 | 22 |
| account-skip | tokensave | 1 | yes | Bash 1, Edit 13, Grep 3, Read 1, tokensave_callers 1, tokensave_read 1, tokensave_search 3 | 3 | 0 | 18 / 20838 / 228581 / 4304 | 0.149 | 71 | 24 |
| account-skip | tokensave | 2 | yes | Bash 2, Edit 3, Grep 3, Read 4, tokensave_callers 1, tokensave_multi_str_replace 1, tokensave_search 1, tokensave_str_replace 8 | 3 | 1 | 20 / 14608 / 236986 / 4768 | 0.130 | 89 | 24 |
| commit-root | grep | 1 | yes | Bash 9, Edit 19, Grep 6, Read 12 | 0 | 3 | 40 / 41099 / 510551 / 9361 | 0.309 | 613 | 47 |
| commit-root | grep | 2 | yes | Bash 6, Edit 22, Grep 12, Read 9 | 0 | 1 | 38 / 46154 / 520275 / 10265 | 0.339 | 614 | 50 |
| commit-root | index-hook | 1 | yes | Bash 5, Edit 17, Grep 6, Read 10, index_search 1 | 6 | 2 | 28 / 33240 / 406077 / 7996 | 0.254 | 481 | 40 |
| commit-root | index-hook | 2 | yes | Bash 7, Edit 23, Grep 5, Read 11, index_search 1 | 5 | 2 | 44 / 47585 / 761099 / 10796 | 0.374 | 691 | 48 |
| commit-root | tokensave | 1 | yes | Bash 6, Edit 21, Grep 3, Read 10, tokensave_search 2 | 3 | 1 | 38 / 38723 / 653470 / 9207 | 0.312 | 512 | 43 |
| commit-root | tokensave | 2 | yes | Bash 7, Edit 19, Grep 4, Read 8, tokensave_read 5, tokensave_search 2 | 4 | 2 | 36 / 51285 / 695559 / 10011 | 0.375 | 676 | 46 |
| i18n-unverified | grep | 1 | yes | Bash 2, Edit 7, Glob 1, Grep 7 | 0 | 0 | 22 / 13912 / 134457 / 2974 | 0.099 | 90 | 18 |
| i18n-unverified | grep | 2 | yes | Bash 2, Edit 7, Grep 5, Read 2 | 0 | 0 | 16 / 8927 / 94083 / 2720 | 0.072 | 80 | 17 |
| i18n-unverified | index-hook | 1 | yes | Bash 1, Edit 7, Grep 4, Read 2, index_search 1 | 4 | 0 | 16 / 9973 / 136527 / 2645 | 0.080 | 61 | 16 |
| i18n-unverified | index-hook | 2 | yes | Bash 1, Edit 7, Grep 4, Read 2, index_search 1 | 4 | 0 | 14 / 6796 / 110587 / 2834 | 0.067 | 85 | 16 |
| i18n-unverified | tokensave | 1 | yes | Bash 1, Edit 7, Grep 2, Read 2, tokensave_search 1 | 2 | 0 | 12 / 12985 / 129072 / 2389 | 0.089 | 53 | 14 |
| i18n-unverified | tokensave | 2 | yes | Bash 2, Edit 8, Grep 2, Read 2, tokensave_search 1 | 2 | 0 | 14 / 11624 / 156968 / 2876 | 0.091 | 87 | 16 |
| fact-docs-gaps | grep | 1 | yes | Bash 3, Edit 12, Grep 2, Read 2 | 0 | 0 | 14 / 8891 / 65944 / 3129 | 0.073 | 188 | 20 |
| fact-docs-gaps | grep | 2 | yes | Bash 5, Edit 12, Grep 7 | 0 | 0 | 20 / 8280 / 116382 / 4072 | 0.086 | 484 | 25 |
| fact-docs-gaps | index-hook | 1 | yes | Bash 5, Edit 14, Grep 5, Read 6, index_search 1 | 5 | 0 | 24 / 16653 / 240906 / 5147 | 0.142 | 235 | 32 |
| fact-docs-gaps | index-hook | 2 | yes | Bash 3, Edit 12, Grep 3 | 0 | 0 | 16 / 14763 / 166309 / 3543 | 0.111 | 358 | 19 |
| fact-docs-gaps | tokensave | 1 | yes | Bash 5, Grep 1, Read 3, tokensave_multi_str_replace 4, tokensave_search 1, tokensave_str_replace 4 | 1 | 0 | 16 / 10321 / 158712 / 3535 | 0.093 | 248 | 19 |
| fact-docs-gaps | tokensave | 2 | yes | Bash 5, Grep 3, Read 4, tokensave_multi_str_replace 4, tokensave_search 1, tokensave_str_replace 4 | 3 | 0 | 20 / 10114 / 220892 / 3915 | 0.102 | 510 | 22 |
| app-env-struct | grep | 1 | yes | Bash 2, Edit 14, Grep 2, Read 7 | 0 | 2 | 14 / 13469 / 84674 / 5198 | 0.114 | 70 | 26 |
| app-env-struct | grep | 2 | yes | Bash 1, Edit 14, Grep 3, Read 8 | 0 | 3 | 14 / 12566 / 90698 / 5360 | 0.113 | 117 | 27 |
| app-env-struct | index-hook | 1 | yes | Bash 1, Edit 9, Grep 1, index_body 1, index_callers 1 | 1 | 0 | 10 / 10690 / 81192 / 2908 | 0.080 | 65 | 14 |
| app-env-struct | index-hook | 2 | yes | Bash 2, Edit 17, Grep 4, Read 1, index_body 1, index_callers 1 | 4 | 0 | 16 / 16175 / 158906 / 5444 | 0.135 | 96 | 27 |
| app-env-struct | tokensave | 1 | yes | Bash 2, Edit 5, Grep 1, Read 6, tokensave_multi_str_replace 3, tokensave_search 3 | 1 | 2 | 18 / 14660 / 198190 / 6027 | 0.139 | 80 | 21 |
| app-env-struct | tokensave | 2 | yes | Bash 2, Grep 2, Read 6, tokensave_callers 1, tokensave_multi_str_replace 3, tokensave_read 1, tokensave_search 2, tokensave_str_replace 4 | 2 | 1 | 16 / 14109 / 185860 / 5066 | 0.126 | 156 | 22 |
| slugify-gone | grep | 1 | yes | Bash 2, Edit 15, Grep 3, Read 7 | 0 | 5 | 16 / 19783 / 141936 / 4759 | 0.141 | 154 | 28 |
| slugify-gone | grep | 2 | yes | Bash 2, Edit 15, Grep 2, Read 1 | 0 | 0 | 12 / 17201 / 101595 / 3952 | 0.119 | 345 | 21 |
| slugify-gone | index-hook | 1 | yes | Bash 3, Edit 16, Grep 3, Read 6, index_callers 1 | 3 | 2 | 20 / 13652 / 196078 / 4819 | 0.122 | 155 | 30 |
| slugify-gone | index-hook | 2 | yes | Bash 2, Edit 15, Grep 2, Read 8, index_callers 1 | 2 | 5 | 16 / 13797 / 153251 / 4839 | 0.119 | 237 | 29 |
| slugify-gone | tokensave | 1 | yes | Bash 2, Edit 18, Grep 3, Read 1, tokensave_search 1 | 3 | 0 | 16 / 13358 / 174121 / 4575 | 0.117 | 53 | 26 |
| slugify-gone | tokensave | 2 | yes | Bash 2, Grep 1, Read 1, tokensave_multi_str_replace 4, tokensave_read 1, tokensave_search 1 | 1 | 0 | 14 / 13724 / 161062 / 2790 | 0.099 | 239 | 11 |
| missing-account-ts | grep | 1 | yes | Bash 2, Edit 16, Grep 3, Read 12 | 0 | 5 | 12 / 11358 / 60862 / 4620 | 0.098 | 77 | 34 |
| missing-account-ts | grep | 2 | yes | Bash 2, Edit 16, Grep 3 | 0 | 0 | 10 / 6463 / 42753 / 3059 | 0.061 | 102 | 22 |
| missing-account-ts | index-hook | 1 | yes | Bash 1, Edit 16, Grep 2, Read 13 | 0 | 6 | 10 / 10815 / 79227 / 4626 | 0.097 | 46 | 33 |
| missing-account-ts | index-hook | 2 | yes | Bash 2, Edit 16, Grep 3 | 0 | 0 | 10 / 7064 / 71377 / 3212 | 0.068 | 94 | 22 |
| missing-account-ts | tokensave | 1 | yes | Bash 2, Edit 7, Grep 2, tokensave_search 1 | 2 | 0 | 12 / 5183 / 104872 / 1995 | 0.051 | 47 | 13 |
| missing-account-ts | tokensave | 2 | yes | Bash 2, Edit 7, Grep 2, tokensave_search 1 | 2 | 0 | 12 / 5159 / 105140 / 1849 | 0.050 | 72 | 13 |

#### Changes at many places, run by run

The places still missed when the agent first ran a check (— when it never did) and at its end, its checks (failed), its cycles « check → fix » and the commands refused.

| Task | Mode | # | Right | Places | Missed first | Missed end | Checks (failed) | Cycles | Refused |
|---|---|---|---|---|---|---|---|---|---|
| plugins-forget | grep | 1 | yes | 16 | 0 | 0 | 1 (0) | 1 | 2 |
| plugins-forget | grep | 2 | yes | 16 | 0 | 0 | 1 (0) | 0 | 0 |
| plugins-forget | index-hook | 1 | yes | 16 | 0 | 0 | 1 (0) | 0 | 0 |
| plugins-forget | index-hook | 2 | yes | 16 | 0 | 0 | 1 (0) | 0 | 0 |
| plugins-forget | tokensave | 1 | yes | 16 | 0 | 0 | 1 (0) | 1 | 2 |
| plugins-forget | tokensave | 2 | yes | 16 | 0 | 0 | 1 (0) | 1 | 2 |
| drafts-files | grep | 1 | yes | 19 | 0 | 0 | 1 (0) | 0 | 0 |
| drafts-files | grep | 2 | yes | 19 | 0 | 0 | 1 (0) | 1 | 2 |
| drafts-files | index-hook | 1 | yes | 19 | 0 | 0 | 1 (0) | 0 | 0 |
| drafts-files | index-hook | 2 | yes | 19 | 0 | 0 | 1 (0) | 0 | 0 |
| drafts-files | tokensave | 1 | yes | 19 | 0 | 0 | 1 (0) | 0 | 0 |
| drafts-files | tokensave | 2 | yes | 19 | 19 | 0 | 2 (0) | 0 | 2 |
| checkout-of | grep | 1 | yes | 26 | 0 | 0 | 1 (0) | 0 | 0 |
| checkout-of | grep | 2 | yes | 26 | 0 | 0 | 1 (0) | 0 | 0 |
| checkout-of | index-hook | 1 | yes | 26 | 0 | 0 | 1 (0) | 0 | 0 |
| checkout-of | index-hook | 2 | yes | 26 | 0 | 0 | 1 (0) | 0 | 2 |
| checkout-of | tokensave | 1 | yes | 26 | 0 | 0 | 1 (0) | 0 | 0 |
| checkout-of | tokensave | 2 | yes | 26 | 0 | 0 | 1 (0) | 0 | 2 |
| account-skip | grep | 1 | yes | 10 | 0 | 0 | 1 (0) | 0 | 2 |
| account-skip | grep | 2 | yes | 10 | 0 | 0 | 1 (0) | 1 | 2 |
| account-skip | index-hook | 1 | yes | 10 | 0 | 0 | 1 (0) | 1 | 2 |
| account-skip | index-hook | 2 | yes | 10 | 0 | 0 | 1 (0) | 0 | 0 |
| account-skip | tokensave | 1 | yes | 10 | 0 | 0 | 1 (0) | 0 | 0 |
| account-skip | tokensave | 2 | yes | 10 | 0 | 0 | 1 (0) | 1 | 2 |
| commit-root | grep | 1 | yes | 9 | 2 | 0 | 8 (4) | 3 | 2 |
| commit-root | grep | 2 | yes | 9 | 0 | 0 | 5 (1) | 1 | 2 |
| commit-root | index-hook | 1 | yes | 9 | 0 | 0 | 4 (0) | 1 | 2 |
| commit-root | index-hook | 2 | yes | 9 | 0 | 0 | 6 (2) | 3 | 2 |
| commit-root | tokensave | 1 | yes | 9 | 0 | 0 | 5 (1) | 2 | 2 |
| commit-root | tokensave | 2 | yes | 9 | 0 | 0 | 6 (2) | 2 | 2 |
| i18n-unverified | grep | 1 | yes | 14 | 0 | 0 | 2 (0) | 0 | 0 |
| i18n-unverified | grep | 2 | yes | 14 | 0 | 0 | 1 (0) | 1 | 2 |
| i18n-unverified | index-hook | 1 | yes | 14 | 0 | 0 | 1 (0) | 0 | 0 |
| i18n-unverified | index-hook | 2 | yes | 14 | 0 | 0 | 1 (0) | 0 | 0 |
| i18n-unverified | tokensave | 1 | yes | 14 | 0 | 0 | 1 (0) | 0 | 0 |
| i18n-unverified | tokensave | 2 | yes | 14 | 0 | 0 | 1 (0) | 1 | 2 |
| fact-docs-gaps | grep | 1 | yes | 21 | 0 | 0 | 2 (0) | 1 | 2 |
| fact-docs-gaps | grep | 2 | yes | 21 | 4 | 0 | 5 (0) | 1 | 0 |
| fact-docs-gaps | index-hook | 1 | yes | 21 | 4 | 0 | 5 (0) | 2 | 0 |
| fact-docs-gaps | index-hook | 2 | yes | 21 | 0 | 0 | 2 (0) | 1 | 2 |
| fact-docs-gaps | tokensave | 1 | yes | 21 | 0 | 0 | 4 (0) | 0 | 2 |
| fact-docs-gaps | tokensave | 2 | yes | 21 | 0 | 0 | 4 (0) | 0 | 2 |
| app-env-struct | grep | 1 | yes | 8 | 0 | 0 | 1 (0) | 0 | 2 |
| app-env-struct | grep | 2 | yes | 8 | 0 | 0 | 1 (0) | 0 | 0 |
| app-env-struct | index-hook | 1 | yes | 8 | 0 | 0 | 1 (0) | 0 | 0 |
| app-env-struct | index-hook | 2 | yes | 8 | 0 | 0 | 1 (0) | 1 | 2 |
| app-env-struct | tokensave | 1 | yes | 8 | 0 | 0 | 1 (0) | 1 | 2 |
| app-env-struct | tokensave | 2 | yes | 8 | 0 | 0 | 1 (0) | 0 | 2 |
| slugify-gone | grep | 1 | yes | 32 | 0 | 0 | 2 (0) | 0 | 0 |
| slugify-gone | grep | 2 | yes | 32 | 0 | 0 | 2 (0) | 0 | 0 |
| slugify-gone | index-hook | 1 | yes | 32 | 0 | 0 | 2 (0) | 0 | 2 |
| slugify-gone | index-hook | 2 | yes | 32 | 0 | 0 | 2 (0) | 0 | 0 |
| slugify-gone | tokensave | 1 | yes | 32 | 0 | 0 | 1 (0) | 1 | 2 |
| slugify-gone | tokensave | 2 | yes | 32 | 0 | 0 | 2 (0) | 0 | 0 |
| missing-account-ts | grep | 1 | yes | 32 | 0 | 0 | 2 (0) | 0 | 0 |
| missing-account-ts | grep | 2 | yes | 32 | 0 | 0 | 2 (0) | 0 | 0 |
| missing-account-ts | index-hook | 1 | yes | 32 | 0 | 0 | 1 (0) | 0 | 0 |
| missing-account-ts | index-hook | 2 | yes | 32 | 0 | 0 | 2 (0) | 0 | 0 |
| missing-account-ts | tokensave | 1 | yes | 32 | 0 | 0 | 2 (0) | 0 | 0 |
| missing-account-ts | tokensave | 2 | yes | 32 | 0 | 0 | 2 (0) | 0 | 0 |

#### Completed after the index

The runs that called the index, then searched or read the files anyway (Grep, Glob, Read after its first call).

| Task | Mode | # | Index calls | Grep / Glob / Read after |
|---|---|---|---|---|
| plugins-forget | tokensave | 1 | 2 | 3 / 0 / 1 |
| commit-root | tokensave | 1 | 2 | 3 / 0 / 10 |
| commit-root | index-hook | 1 | 1 | 6 / 0 / 10 |
| plugins-forget | index-hook | 1 | 1 | 5 / 0 / 2 |
| drafts-files | tokensave | 1 | 7 | 1 / 0 / 1 |
| drafts-files | index-hook | 1 | 1 | 1 / 0 / 0 |
| checkout-of | index-hook | 1 | 1 | 1 / 0 / 0 |
| account-skip | tokensave | 1 | 5 | 3 / 0 / 1 |
| account-skip | index-hook | 1 | 4 | 1 / 0 / 0 |
| i18n-unverified | tokensave | 1 | 1 | 2 / 0 / 2 |
| i18n-unverified | index-hook | 1 | 1 | 4 / 0 / 2 |
| fact-docs-gaps | tokensave | 1 | 9 | 1 / 0 / 3 |
| fact-docs-gaps | index-hook | 1 | 1 | 5 / 0 / 6 |
| app-env-struct | index-hook | 1 | 2 | 1 / 0 / 0 |
| app-env-struct | tokensave | 1 | 6 | 1 / 0 / 6 |
| slugify-gone | index-hook | 1 | 1 | 3 / 0 / 6 |
| slugify-gone | tokensave | 1 | 1 | 3 / 0 / 1 |
| missing-account-ts | tokensave | 1 | 1 | 2 / 0 / 0 |
| plugins-forget | tokensave | 2 | 1 | 5 / 0 / 1 |
| plugins-forget | index-hook | 2 | 1 | 5 / 0 / 0 |
| drafts-files | tokensave | 2 | 11 | 1 / 0 / 0 |
| drafts-files | index-hook | 2 | 1 | 2 / 0 / 9 |
| checkout-of | index-hook | 2 | 1 | 1 / 0 / 9 |
| account-skip | tokensave | 2 | 11 | 3 / 0 / 4 |
| account-skip | index-hook | 2 | 2 | 3 / 0 / 1 |
| commit-root | tokensave | 2 | 7 | 4 / 0 / 8 |
| i18n-unverified | tokensave | 2 | 1 | 2 / 0 / 2 |
| commit-root | index-hook | 2 | 1 | 5 / 0 / 11 |
| i18n-unverified | index-hook | 2 | 1 | 4 / 0 / 2 |
| fact-docs-gaps | tokensave | 2 | 9 | 3 / 0 / 4 |
| app-env-struct | tokensave | 2 | 11 | 2 / 0 / 6 |
| app-env-struct | index-hook | 2 | 2 | 4 / 0 / 1 |
| slugify-gone | tokensave | 2 | 6 | 1 / 0 / 1 |
| slugify-gone | index-hook | 2 | 1 | 2 / 0 / 8 |
| missing-account-ts | tokensave | 2 | 1 | 2 / 0 / 0 |

### claude-haiku-5-5

60 runs · 10 tasks · model claude-haiku-5-5 (medium) · commit 9632cde9

#### Totals by mode

| | grep | index-hook | tokensave |
|---|---|---|---|
| Runs (right) | 20 (20) | 20 (20) | 20 (20) |
| Tool calls | 619 | 588 | 529 |
| · Bash | 82 | 67 | 56 |
| · Edit | 247 | 240 | 154 |
| · Glob | 2 | 0 | 1 |
| · Grep | 83 | 76 | 93 |
| · Mcp__escouade__index_search | 0 | 1 | 0 |
| · Mcp__tokensave__tokensave_multi_str_replace | 0 | 0 | 1 |
| · Monitor | 0 | 1 | 0 |
| · Read | 205 | 171 | 101 |
| · index_body | 0 | 6 | 0 |
| · index_callers | 0 | 13 | 0 |
| · index_search | 0 | 13 | 0 |
| · tokensave_body | 0 | 0 | 15 |
| · tokensave_callers | 0 | 0 | 3 |
| · tokensave_context | 0 | 0 | 1 |
| · tokensave_multi_str_replace | 0 | 0 | 31 |
| · tokensave_read | 0 | 0 | 44 |
| · tokensave_search | 0 | 0 | 11 |
| · tokensave_str_replace | 0 | 0 | 18 |
| Grep after the index | 0 | 44 | 58 |
| Glob / Read after the index | 0 / 0 | 0 / 115 | 1 / 52 |
| Rereads | 70 | 50 | 37 |
| Places to reach | 374 | 374 | 374 |
| Missed at the first check (runs that checked) | 0 (20) | 8 (20) | 6 (20) |
| Missed at the end | 0 | 0 | 0 |
| Checks of the terminal (failed) | 78 (8) | 65 (9) | 55 (5) |
| Cycles check → fix | 5 | 4 | 2 |
| Commands refused | 8 | 4 | 2 |
| Runs completed after the index | 0 / 20 | 13 / 20 | 13 / 20 |
| Hook: searches seen / answered | 0 / 0 | 143 / 16 | 0 / 0 |
| Tokens: input | 0.5 k | 0.4 k | 0.5 k |
| · cache written | 642.7 k | 527.5 k | 524.7 k |
| · cache read | 7596.7 k | 6688.3 k | 8011.7 k |
| · output | 187.5 k | 166.8 k | 173.7 k |
| · total | 8427.4 k | 7383.0 k | 8710.5 k |
| Tokens of the tools' answers: the index's | 0.0 k | 12.7 k | 44.4 k |
| · Bash | 54.5 k | 35.9 k | 26.1 k |
| · Edit | 7.1 k | 7.2 k | 4.5 k |
| · Glob | 0.0 k | 0.0 k | 0.0 k |
| · Grep | 72.1 k | 58.3 k | 60.8 k |
| · Mcp__escouade__index_search | 0.0 k | 0.0 k | 0.0 k |
| · Mcp__tokensave__tokensave_multi_str_replace | 0.0 k | 0.0 k | 0.1 k |
| · Monitor | 0.0 k | 0.0 k | 0.0 k |
| · Read | 102.9 k | 62.3 k | 32.3 k |
| · index_body | 0.0 k | 1.3 k | 0.0 k |
| · index_callers | 0.0 k | 7.1 k | 0.0 k |
| · index_search | 0.0 k | 4.4 k | 0.0 k |
| · tokensave_body | 0.0 k | 0.0 k | 1.8 k |
| · tokensave_callers | 0.0 k | 0.0 k | 2.5 k |
| · tokensave_context | 0.0 k | 0.0 k | 2.9 k |
| · tokensave_multi_str_replace | 0.0 k | 0.0 k | 1.8 k |
| · tokensave_read | 0.0 k | 0.0 k | 30.4 k |
| · tokensave_search | 0.0 k | 0.0 k | 4.2 k |
| · tokensave_str_replace | 0.0 k | 0.0 k | 0.9 k |
| Cost ($, Claude Code's count) | 0.333 | 0.256 | 0.319 |
| · against grep | +0 % | -23 % | -4 % |
| Time (s) | 7177 | 5781 | 5476 |
| Turns | 639 | 608 | 549 |

#### By task: right, calls, cost, against grep

| Task | grep | index-hook | tokensave |
|---|---|---|---|
| plugins-forget | 2/2 · 46 calls · 0.027 $ | 2/2 · 40 calls · 0.025 $ (-8 %) | 2/2 · 53 calls · 0.030 $ (+11 %) |
| drafts-files | 2/2 · 48 calls · 0.017 $ | 2/2 · 44 calls · 0.011 $ (-35 %) | 2/2 · 28 calls · 0.009 $ (-44 %) |
| checkout-of | 2/2 · 53 calls · 0.012 $ | 2/2 · 56 calls · 0.015 $ (+18 %) | 2/2 · 38 calls · 0.013 $ (+3 %) |
| account-skip | 2/2 · 69 calls · 0.036 $ | 2/2 · 57 calls · 0.022 $ (-37 %) | 2/2 · 57 calls · 0.026 $ (-27 %) |
| commit-root | 2/2 · 124 calls · 0.138 $ | 2/2 · 105 calls · 0.077 $ (-44 %) | 2/2 · 106 calls · 0.139 $ (+0 %) |
| i18n-unverified | 2/2 · 44 calls · 0.015 $ | 2/2 · 43 calls · 0.016 $ (+6 %) | 2/2 · 30 calls · 0.016 $ (+6 %) |
| fact-docs-gaps | 2/2 · 71 calls · 0.028 $ | 2/2 · 63 calls · 0.024 $ (-14 %) | 2/2 · 54 calls · 0.021 $ (-25 %) |
| app-env-struct | 2/2 · 52 calls · 0.028 $ | 2/2 · 63 calls · 0.027 $ (-5 %) | 2/2 · 49 calls · 0.029 $ (+4 %) |
| slugify-gone | 2/2 · 62 calls · 0.021 $ | 2/2 · 69 calls · 0.028 $ (+32 %) | 2/2 · 54 calls · 0.022 $ (+3 %) |
| missing-account-ts | 2/2 · 50 calls · 0.011 $ | 2/2 · 48 calls · 0.011 $ (+5 %) | 2/2 · 60 calls · 0.014 $ (+31 %) |

#### Every run

| Task | Mode | # | Right | Calls by tool | Grep after | Rereads | Tokens (in / cache w / cache r / out) | $ | s | Turns |
|---|---|---|---|---|---|---|---|---|---|---|
| plugins-forget | grep | 1 | yes | Bash 3, Edit 9, Grep 6, Read 7 | 0 | 3 | 24 / 31687 / 272241 / 12105 | 0.015 | 294 | 26 |
| plugins-forget | grep | 2 | yes | Bash 4, Edit 9, Grep 3, Read 5 | 0 | 1 | 22 / 24656 / 225612 / 8739 | 0.012 | 184 | 22 |
| plugins-forget | index-hook | 1 | yes | Bash 2, Edit 4, Grep 5, Read 5, index_callers 1, index_search 1 | 5 | 1 | 18 / 35002 / 194591 / 5642 | 0.012 | 285 | 19 |
| plugins-forget | index-hook | 2 | yes | Bash 2, Edit 11, Grep 4, Read 4, index_callers 1 | 4 | 1 | 20 / 29729 / 280435 / 8159 | 0.013 | 166 | 23 |
| plugins-forget | tokensave | 1 | yes | Bash 3, Edit 9, Grep 8, Read 1, tokensave_read 1 | 5 | 0 | 24 / 37899 / 331030 / 7310 | 0.015 | 265 | 23 |
| plugins-forget | tokensave | 2 | yes | Bash 2, Edit 8, Grep 10, Read 5, tokensave_callers 1, tokensave_read 4, tokensave_search 1 | 9 | 1 | 30 / 27137 / 454030 / 10310 | 0.015 | 201 | 32 |
| drafts-files | grep | 1 | yes | Bash 2, Edit 10, Grep 2, Read 9 | 0 | 2 | 12 / 36009 / 171681 / 5063 | 0.011 | 217 | 24 |
| drafts-files | grep | 2 | yes | Bash 2, Edit 11, Grep 3, Read 9 | 0 | 2 | 16 / 9713 / 94191 / 4566 | 0.005 | 114 | 26 |
| drafts-files | index-hook | 1 | yes | Bash 1, Edit 10, Grep 4, Read 9, index_search 1 | 4 | 2 | 18 / 10776 / 159182 / 4554 | 0.006 | 87 | 26 |
| drafts-files | index-hook | 2 | yes | Bash 2, Edit 8, Grep 2, Read 7 | 0 | 0 | 14 / 8504 / 115051 / 3967 | 0.005 | 107 | 20 |
| drafts-files | tokensave | 1 | yes | Bash 1, Grep 3, Read 3, tokensave_multi_str_replace 3, tokensave_search 1, tokensave_str_replace 4 | 3 | 1 | 12 / 8934 / 118457 / 4279 | 0.005 | 108 | 16 |
| drafts-files | tokensave | 2 | yes | Bash 2, Edit 8, Grep 2, Read 1 | 0 | 0 | 12 / 6921 / 116264 / 3346 | 0.004 | 120 | 14 |
| checkout-of | grep | 1 | yes | Bash 2, Edit 11, Grep 2, Read 11 | 0 | 5 | 14 / 13331 / 90821 / 5684 | 0.006 | 162 | 27 |
| checkout-of | grep | 2 | yes | Bash 3, Edit 11, Grep 2, Read 11 | 0 | 5 | 18 / 10194 / 123539 / 5471 | 0.006 | 163 | 28 |
| checkout-of | index-hook | 1 | yes | Bash 2, Edit 11, Grep 2, Read 11, index_search 1 | 2 | 5 | 14 / 14684 / 137522 / 5898 | 0.007 | 166 | 28 |
| checkout-of | index-hook | 2 | yes | Bash 2, Edit 11, Grep 2, Read 11, index_body 1, index_callers 1, index_search 1 | 2 | 6 | 14 / 13774 / 141124 / 6593 | 0.007 | 163 | 30 |
| checkout-of | tokensave | 1 | yes | Bash 2, Edit 11, Grep 2, Read 3 | 0 | 0 | 12 / 10667 / 125020 / 5146 | 0.006 | 166 | 19 |
| checkout-of | tokensave | 2 | yes | Bash 2, Edit 5, Grep 3, Read 10 | 0 | 4 | 16 / 10715 / 181436 / 5819 | 0.007 | 169 | 21 |
| account-skip | grep | 1 | yes | Bash 3, Edit 11, Grep 4, Read 10 | 0 | 4 | 22 / 31388 / 198850 / 7803 | 0.012 | 346 | 29 |
| account-skip | grep | 2 | yes | Bash 15, Edit 12, Grep 4, Read 10 | 0 | 4 | 34 / 56427 / 492456 / 14580 | 0.024 | 1213 | 42 |
| account-skip | index-hook | 1 | yes | Bash 2, Edit 13, Grep 4, Read 2, index_callers 1 | 3 | 1 | 20 / 19335 / 209348 / 7161 | 0.010 | 239 | 23 |
| account-skip | index-hook | 2 | yes | Bash 3, Edit 13, Grep 3, Read 12, index_body 1, index_callers 1, index_search 2 | 3 | 6 | 26 / 23893 / 312026 / 9910 | 0.013 | 333 | 36 |
| account-skip | tokensave | 1 | yes | Bash 2, Edit 8, Grep 3, Read 4, tokensave_body 5, tokensave_search 4, tokensave_str_replace 3 | 3 | 2 | 26 / 19893 / 324406 / 8577 | 0.012 | 208 | 30 |
| account-skip | tokensave | 2 | yes | Bash 3, Edit 11, Grep 1, Read 2, tokensave_body 9, tokensave_callers 1, tokensave_search 1 | 1 | 1 | 30 / 26413 / 456005 / 9077 | 0.014 | 787 | 29 |
| commit-root | grep | 1 | yes | Bash 11, Edit 31, Glob 1, Grep 11, Read 19 | 0 | 7 | 86 / 99891 / 2619874 / 28434 | 0.095 | 1672 | 74 |
| commit-root | grep | 2 | yes | Bash 6, Edit 24, Grep 9, Read 12 | 0 | 1 | 48 / 85316 / 1380996 / 24875 | 0.043 | 751 | 52 |
| commit-root | index-hook | 1 | yes | Bash 14, Edit 25, Grep 7, Read 12, index_body 3, index_callers 3, index_search 1 | 7 | 2 | 62 / 82890 / 1737848 / 23814 | 0.046 | 1542 | 66 |
| commit-root | index-hook | 2 | yes | Bash 4, Edit 21, Grep 6, Read 8, index_search 1 | 6 | 1 | 42 / 59730 / 996409 / 18791 | 0.031 | 609 | 41 |
| commit-root | tokensave | 1 | yes | Bash 5, Edit 21, Grep 13, tokensave_context 1, tokensave_read 14 | 13 | 0 | 54 / 95930 / 1937535 / 26653 | 0.099 | 611 | 55 |
| commit-root | tokensave | 2 | yes | Bash 5, Edit 21, Glob 1, Grep 8, Mcp__tokensave__tokensave_multi_str_replace 1, Read 14, tokensave_multi_str_replace 1, tokensave_search 1 | 8 | 4 | 54 / 68094 / 1569792 / 21605 | 0.040 | 684 | 53 |
| i18n-unverified | grep | 1 | yes | Bash 2, Edit 7, Grep 4, Read 8 | 0 | 1 | 16 / 18896 / 131171 / 4786 | 0.007 | 72 | 22 |
| i18n-unverified | grep | 2 | yes | Bash 2, Edit 7, Grep 4, Read 10 | 0 | 3 | 16 / 19674 / 123809 / 5379 | 0.008 | 76 | 24 |
| i18n-unverified | index-hook | 1 | yes | Bash 2, Edit 7, Grep 5, Read 8 | 0 | 1 | 18 / 19887 / 201669 / 4652 | 0.008 | 102 | 23 |
| i18n-unverified | index-hook | 2 | yes | Bash 2, Edit 7, Grep 4, Read 8 | 0 | 1 | 16 / 19068 / 172441 / 4740 | 0.008 | 108 | 22 |
| i18n-unverified | tokensave | 1 | yes | Bash 2, Grep 3, Read 2, tokensave_multi_str_replace 7 | 1 | 0 | 12 / 21556 / 153467 / 4608 | 0.008 | 65 | 15 |
| i18n-unverified | tokensave | 2 | yes | Bash 3, Grep 5, tokensave_multi_str_replace 3, tokensave_read 1, tokensave_str_replace 4 | 2 | 0 | 16 / 19964 / 199455 / 4381 | 0.008 | 108 | 17 |
| fact-docs-gaps | grep | 1 | yes | Bash 6, Edit 12, Grep 5, Read 8 | 0 | 0 | 26 / 24791 / 286808 / 7528 | 0.012 | 310 | 32 |
| fact-docs-gaps | grep | 2 | yes | Bash 8, Edit 12, Glob 1, Grep 8, Read 11 | 0 | 1 | 34 / 33995 / 458649 / 9538 | 0.016 | 590 | 41 |
| fact-docs-gaps | index-hook | 1 | yes | Bash 6, Edit 11, Grep 8, Read 9 | 0 | 0 | 26 / 24770 / 333111 / 8815 | 0.013 | 242 | 35 |
| fact-docs-gaps | index-hook | 2 | yes | Bash 5, Edit 10, Grep 7, Read 7 | 0 | 0 | 22 / 22659 / 262898 / 7746 | 0.011 | 543 | 30 |
| fact-docs-gaps | tokensave | 1 | yes | Bash 4, Edit 12, Grep 5, Read 5 | 0 | 1 | 26 / 21770 / 370523 / 6347 | 0.011 | 271 | 27 |
| fact-docs-gaps | tokensave | 2 | yes | Bash 4, Edit 10, Grep 6, Read 8 | 0 | 2 | 22 / 17035 / 282319 / 6774 | 0.010 | 433 | 29 |
| app-env-struct | grep | 1 | yes | Bash 2, Edit 13, Grep 2, Read 7 | 0 | 1 | 14 / 47156 / 236771 / 10028 | 0.017 | 218 | 25 |
| app-env-struct | grep | 2 | yes | Bash 2, Edit 13, Grep 3, Read 10 | 0 | 4 | 18 / 25199 / 156055 / 9540 | 0.011 | 297 | 29 |
| app-env-struct | index-hook | 1 | yes | Bash 2, Edit 12, Grep 2, Read 8, index_callers 1, index_search 1 | 2 | 2 | 16 / 21525 / 176590 / 9848 | 0.011 | 217 | 27 |
| app-env-struct | index-hook | 2 | yes | Bash 6, Edit 13, Grep 2, Mcp__escouade__index_search 1, Monitor 1, Read 12, index_callers 1, index_search 1 | 2 | 5 | 34 / 28010 / 475689 / 10823 | 0.016 | 321 | 38 |
| app-env-struct | tokensave | 1 | yes | Bash 3, Grep 4, tokensave_body 1, tokensave_callers 1, tokensave_multi_str_replace 6, tokensave_read 8 | 4 | 0 | 22 / 28028 / 324951 / 12105 | 0.015 | 238 | 24 |
| app-env-struct | tokensave | 2 | yes | Bash 4, Grep 4, tokensave_multi_str_replace 4, tokensave_read 8, tokensave_search 1, tokensave_str_replace 5 | 4 | 0 | 22 / 30127 / 319271 / 10280 | 0.014 | 467 | 27 |
| slugify-gone | grep | 1 | yes | Bash 3, Edit 15, Grep 2, Read 12 | 0 | 8 | 20 / 25611 / 228027 / 7059 | 0.011 | 157 | 33 |
| slugify-gone | grep | 2 | yes | Bash 2, Edit 15, Grep 3, Read 10 | 0 | 6 | 16 / 26569 / 168690 / 6950 | 0.010 | 188 | 31 |
| slugify-gone | index-hook | 1 | yes | Bash 3, Edit 15, Grep 2, Read 10, index_callers 1, index_search 1 | 2 | 6 | 16 / 31632 / 221485 / 7639 | 0.012 | 208 | 33 |
| slugify-gone | index-hook | 2 | yes | Bash 3, Edit 15, Grep 2, Read 12, index_body 1, index_callers 2, index_search 2 | 2 | 8 | 22 / 40351 / 318527 / 9247 | 0.016 | 168 | 38 |
| slugify-gone | tokensave | 1 | yes | Bash 2, Grep 4, Read 11, tokensave_multi_str_replace 4, tokensave_read 8, tokensave_search 1 | 4 | 7 | 18 / 22060 / 243292 / 7787 | 0.011 | 177 | 31 |
| slugify-gone | tokensave | 2 | yes | Bash 3, Grep 3, Read 12, tokensave_multi_str_replace 3, tokensave_search 1, tokensave_str_replace 2 | 1 | 8 | 18 / 24726 / 232275 / 7894 | 0.011 | 286 | 25 |
| missing-account-ts | grep | 1 | yes | Bash 2, Edit 7, Grep 3, Read 13 | 0 | 6 | 12 / 12078 / 69851 / 4570 | 0.005 | 60 | 26 |
| missing-account-ts | grep | 2 | yes | Bash 2, Edit 7, Grep 3, Read 13 | 0 | 6 | 12 / 10109 / 66620 / 4840 | 0.005 | 93 | 26 |
| missing-account-ts | index-hook | 1 | yes | Bash 2, Edit 7, Grep 3, Read 8 | 0 | 1 | 14 / 10384 / 119156 / 3832 | 0.005 | 70 | 21 |
| missing-account-ts | index-hook | 2 | yes | Bash 2, Edit 16, Grep 2, Read 8 | 0 | 1 | 14 / 10868 / 123193 / 4983 | 0.006 | 105 | 29 |
| missing-account-ts | tokensave | 1 | yes | Bash 2, Edit 16, Grep 3, Read 12 | 0 | 5 | 12 / 13754 / 124420 / 5795 | 0.007 | 46 | 34 |
| missing-account-ts | tokensave | 2 | yes | Bash 2, Edit 14, Grep 3, Read 8 | 0 | 1 | 14 / 13080 / 147703 / 5597 | 0.007 | 67 | 28 |

#### Changes at many places, run by run

The places still missed when the agent first ran a check (— when it never did) and at its end, its checks (failed), its cycles « check → fix » and the commands refused.

| Task | Mode | # | Right | Places | Missed first | Missed end | Checks (failed) | Cycles | Refused |
|---|---|---|---|---|---|---|---|---|---|
| plugins-forget | grep | 1 | yes | 16 | 0 | 0 | 3 (0) | 0 | 0 |
| plugins-forget | grep | 2 | yes | 16 | 0 | 0 | 3 (0) | 1 | 2 |
| plugins-forget | index-hook | 1 | yes | 16 | 0 | 0 | 2 (1) | 0 | 0 |
| plugins-forget | index-hook | 2 | yes | 16 | 0 | 0 | 2 (0) | 0 | 0 |
| plugins-forget | tokensave | 1 | yes | 16 | 2 | 0 | 3 (1) | 1 | 0 |
| plugins-forget | tokensave | 2 | yes | 16 | 0 | 0 | 2 (0) | 0 | 0 |
| drafts-files | grep | 1 | yes | 19 | 0 | 0 | 2 (0) | 0 | 0 |
| drafts-files | grep | 2 | yes | 19 | 0 | 0 | 2 (0) | 0 | 0 |
| drafts-files | index-hook | 1 | yes | 19 | 0 | 0 | 1 (0) | 0 | 0 |
| drafts-files | index-hook | 2 | yes | 19 | 0 | 0 | 2 (0) | 0 | 0 |
| drafts-files | tokensave | 1 | yes | 19 | 0 | 0 | 1 (0) | 0 | 0 |
| drafts-files | tokensave | 2 | yes | 19 | 0 | 0 | 2 (0) | 0 | 0 |
| checkout-of | grep | 1 | yes | 26 | 0 | 0 | 2 (0) | 0 | 0 |
| checkout-of | grep | 2 | yes | 26 | 0 | 0 | 3 (0) | 0 | 0 |
| checkout-of | index-hook | 1 | yes | 26 | 0 | 0 | 2 (0) | 0 | 0 |
| checkout-of | index-hook | 2 | yes | 26 | 0 | 0 | 2 (0) | 0 | 0 |
| checkout-of | tokensave | 1 | yes | 26 | 0 | 0 | 2 (0) | 0 | 0 |
| checkout-of | tokensave | 2 | yes | 26 | 0 | 0 | 2 (0) | 0 | 0 |
| account-skip | grep | 1 | yes | 10 | 0 | 0 | 3 (0) | 0 | 0 |
| account-skip | grep | 2 | yes | 10 | 0 | 0 | 15 (3) | 0 | 0 |
| account-skip | index-hook | 1 | yes | 10 | 0 | 0 | 2 (1) | 0 | 0 |
| account-skip | index-hook | 2 | yes | 10 | 0 | 0 | 3 (2) | 0 | 0 |
| account-skip | tokensave | 1 | yes | 10 | 0 | 0 | 2 (0) | 0 | 0 |
| account-skip | tokensave | 2 | yes | 10 | 0 | 0 | 3 (2) | 0 | 0 |
| commit-root | grep | 1 | yes | 9 | 0 | 0 | 11 (3) | 1 | 0 |
| commit-root | grep | 2 | yes | 9 | 0 | 0 | 6 (2) | 2 | 0 |
| commit-root | index-hook | 1 | yes | 9 | 0 | 0 | 14 (3) | 2 | 0 |
| commit-root | index-hook | 2 | yes | 9 | 0 | 0 | 4 (0) | 0 | 0 |
| commit-root | tokensave | 1 | yes | 9 | 0 | 0 | 4 (0) | 1 | 2 |
| commit-root | tokensave | 2 | yes | 9 | 0 | 0 | 5 (0) | 0 | 0 |
| i18n-unverified | grep | 1 | yes | 14 | 0 | 0 | 2 (0) | 0 | 0 |
| i18n-unverified | grep | 2 | yes | 14 | 0 | 0 | 2 (0) | 0 | 0 |
| i18n-unverified | index-hook | 1 | yes | 14 | 0 | 0 | 2 (0) | 0 | 0 |
| i18n-unverified | index-hook | 2 | yes | 14 | 0 | 0 | 2 (0) | 0 | 0 |
| i18n-unverified | tokensave | 1 | yes | 14 | 0 | 0 | 2 (0) | 0 | 0 |
| i18n-unverified | tokensave | 2 | yes | 14 | 0 | 0 | 3 (0) | 0 | 0 |
| fact-docs-gaps | grep | 1 | yes | 21 | 0 | 0 | 5 (0) | 0 | 2 |
| fact-docs-gaps | grep | 2 | yes | 21 | 0 | 0 | 6 (0) | 1 | 4 |
| fact-docs-gaps | index-hook | 1 | yes | 21 | 0 | 0 | 5 (0) | 0 | 2 |
| fact-docs-gaps | index-hook | 2 | yes | 21 | 8 | 0 | 5 (0) | 1 | 0 |
| fact-docs-gaps | tokensave | 1 | yes | 21 | 0 | 0 | 4 (0) | 0 | 0 |
| fact-docs-gaps | tokensave | 2 | yes | 21 | 4 | 0 | 4 (0) | 0 | 0 |
| app-env-struct | grep | 1 | yes | 8 | 0 | 0 | 2 (0) | 0 | 0 |
| app-env-struct | grep | 2 | yes | 8 | 0 | 0 | 2 (0) | 0 | 0 |
| app-env-struct | index-hook | 1 | yes | 8 | 0 | 0 | 2 (0) | 0 | 0 |
| app-env-struct | index-hook | 2 | yes | 8 | 0 | 0 | 5 (0) | 1 | 2 |
| app-env-struct | tokensave | 1 | yes | 8 | 0 | 0 | 3 (1) | 0 | 0 |
| app-env-struct | tokensave | 2 | yes | 8 | 0 | 0 | 4 (1) | 0 | 0 |
| slugify-gone | grep | 1 | yes | 32 | 0 | 0 | 3 (0) | 0 | 0 |
| slugify-gone | grep | 2 | yes | 32 | 0 | 0 | 2 (0) | 0 | 0 |
| slugify-gone | index-hook | 1 | yes | 32 | 0 | 0 | 3 (0) | 0 | 0 |
| slugify-gone | index-hook | 2 | yes | 32 | 0 | 0 | 3 (2) | 0 | 0 |
| slugify-gone | tokensave | 1 | yes | 32 | 0 | 0 | 2 (0) | 0 | 0 |
| slugify-gone | tokensave | 2 | yes | 32 | 0 | 0 | 3 (0) | 0 | 0 |
| missing-account-ts | grep | 1 | yes | 32 | 0 | 0 | 2 (0) | 0 | 0 |
| missing-account-ts | grep | 2 | yes | 32 | 0 | 0 | 2 (0) | 0 | 0 |
| missing-account-ts | index-hook | 1 | yes | 32 | 0 | 0 | 2 (0) | 0 | 0 |
| missing-account-ts | index-hook | 2 | yes | 32 | 0 | 0 | 2 (0) | 0 | 0 |
| missing-account-ts | tokensave | 1 | yes | 32 | 0 | 0 | 2 (0) | 0 | 0 |
| missing-account-ts | tokensave | 2 | yes | 32 | 0 | 0 | 2 (0) | 0 | 0 |

#### Completed after the index

The runs that called the index, then searched or read the files anyway (Grep, Glob, Read after its first call).

| Task | Mode | # | Index calls | Grep / Glob / Read after |
|---|---|---|---|---|
| plugins-forget | index-hook | 1 | 2 | 5 / 0 / 5 |
| plugins-forget | tokensave | 1 | 1 | 5 / 0 / 1 |
| commit-root | tokensave | 1 | 15 | 13 / 0 / 0 |
| commit-root | index-hook | 1 | 7 | 7 / 0 / 12 |
| drafts-files | index-hook | 1 | 1 | 4 / 0 / 9 |
| drafts-files | tokensave | 1 | 8 | 3 / 0 / 3 |
| checkout-of | index-hook | 1 | 1 | 2 / 0 / 11 |
| account-skip | index-hook | 1 | 1 | 3 / 0 / 1 |
| account-skip | tokensave | 1 | 12 | 3 / 0 / 4 |
| i18n-unverified | tokensave | 1 | 7 | 1 / 0 / 0 |
| app-env-struct | index-hook | 1 | 2 | 2 / 0 / 8 |
| app-env-struct | tokensave | 1 | 16 | 4 / 0 / 0 |
| slugify-gone | index-hook | 1 | 2 | 2 / 0 / 10 |
| slugify-gone | tokensave | 1 | 13 | 4 / 0 / 11 |
| plugins-forget | index-hook | 2 | 1 | 4 / 0 / 4 |
| plugins-forget | tokensave | 2 | 6 | 9 / 0 / 5 |
| checkout-of | index-hook | 2 | 3 | 2 / 0 / 11 |
| account-skip | index-hook | 2 | 4 | 3 / 0 / 12 |
| commit-root | index-hook | 2 | 1 | 6 / 0 / 8 |
| account-skip | tokensave | 2 | 11 | 1 / 0 / 2 |
| commit-root | tokensave | 2 | 2 | 8 / 1 / 14 |
| app-env-struct | index-hook | 2 | 2 | 2 / 0 / 12 |
| i18n-unverified | tokensave | 2 | 8 | 2 / 0 / 0 |
| app-env-struct | tokensave | 2 | 18 | 4 / 0 / 0 |
| slugify-gone | tokensave | 2 | 6 | 1 / 0 / 12 |
| slugify-gone | index-hook | 2 | 5 | 2 / 0 / 12 |

### claude-opus-5-5

10 runs · 5 tasks · model claude-opus-5-5 (medium) · commit 9632cde9

#### Totals by mode

| | grep | index-hook |
|---|---|---|
| Runs (right) | 5 (5) | 5 (5) |
| Tool calls | 146 | 152 |
| · Bash | 20 | 22 |
| · Edit | 61 | 61 |
| · Glob | 0 | 1 |
| · Grep | 29 | 16 |
| · Read | 36 | 43 |
| · index_body | 0 | 6 |
| · index_callers | 0 | 1 |
| · index_impact | 0 | 1 |
| · index_search | 0 | 1 |
| Grep after the index | 0 | 10 |
| Glob / Read after the index | 0 / 0 | 0 / 28 |
| Rereads | 3 | 8 |
| Places to reach | 88 | 88 |
| Missed at the first check (runs that checked) | 4 (5) | 4 (5) |
| Missed at the end | 0 | 0 |
| Checks of the terminal (failed) | 19 (3) | 20 (4) |
| Cycles check → fix | 1 | 2 |
| Commands refused | 2 | 4 |
| Runs completed after the index | 0 / 5 | 3 / 5 |
| Hook: searches seen / answered | 0 / 0 | 38 / 2 |
| Tokens: input | 0.1 k | 0.1 k |
| · cache written | 140.7 k | 158.8 k |
| · cache read | 1672.7 k | 2258.5 k |
| · output | 32.4 k | 31.6 k |
| · total | 1845.9 k | 2449.1 k |
| Tokens of the tools' answers: the index's | 0.0 k | 2.9 k |
| · Bash | 13.5 k | 18.9 k |
| · Edit | 1.9 k | 1.9 k |
| · Glob | 0.0 k | 0.0 k |
| · Grep | 16.8 k | 11.2 k |
| · Read | 24.4 k | 26.4 k |
| · index_body | 0.0 k | 1.1 k |
| · index_callers | 0.0 k | 0.6 k |
| · index_impact | 0.0 k | 0.9 k |
| · index_search | 0.0 k | 0.3 k |
| Cost ($, Claude Code's count) | 2.108 | 2.355 |
| · against grep | +0 % | +12 % |
| Time (s) | 1539 | 1479 |
| Turns | 151 | 157 |

#### By task: right, calls, cost, against grep

| Task | grep | index-hook |
|---|---|---|
| plugins-forget | 1/1 · 20 calls · 0.312 $ | 1/1 · 19 calls · 0.399 $ (+28 %) |
| account-skip | 1/1 · 25 calls · 0.445 $ | 1/1 · 32 calls · 0.480 $ (+8 %) |
| commit-root | 1/1 · 54 calls · 0.874 $ | 1/1 · 55 calls · 1.045 $ (+20 %) |
| fact-docs-gaps | 1/1 · 29 calls · 0.324 $ | 1/1 · 28 calls · 0.279 $ (-14 %) |
| missing-account-ts | 1/1 · 18 calls · 0.153 $ | 1/1 · 18 calls · 0.152 $ (-0 %) |

#### Every run

| Task | Mode | # | Right | Calls by tool | Grep after | Rereads | Tokens (in / cache w / cache r / out) | $ | s | Turns |
|---|---|---|---|---|---|---|---|---|---|---|
| plugins-forget | grep | 1 | yes | Bash 3, Edit 9, Grep 4, Read 4 | 0 | 0 | 24 / 20780 / 200526 / 5295 | 0.312 | 163 | 21 |
| plugins-forget | index-hook | 1 | yes | Bash 2, Edit 9, Grep 2, Read 5, index_impact 1 | 2 | 1 | 20 / 32052 / 222173 / 4879 | 0.399 | 134 | 20 |
| account-skip | grep | 1 | yes | Bash 4, Edit 12, Grep 5, Read 4 | 0 | 1 | 28 / 31066 / 304922 / 6749 | 0.445 | 591 | 26 |
| account-skip | index-hook | 1 | yes | Bash 6, Edit 12, Grep 1, Read 9, index_body 3, index_callers 1 | 1 | 2 | 32 / 33692 / 373666 / 6791 | 0.480 | 710 | 33 |
| commit-root | grep | 1 | yes | Bash 6, Edit 24, Grep 11, Read 13 | 0 | 2 | 50 / 56948 / 844899 / 12464 | 0.874 | 478 | 55 |
| commit-root | index-hook | 1 | yes | Bash 7, Edit 23, Grep 7, Read 14, index_body 3, index_search 1 | 7 | 5 | 62 / 65542 / 1341291 / 12617 | 1.045 | 377 | 56 |
| fact-docs-gaps | grep | 1 | yes | Bash 5, Edit 9, Grep 7, Read 8 | 0 | 0 | 26 / 21134 / 254465 / 5216 | 0.324 | 263 | 30 |
| fact-docs-gaps | index-hook | 1 | yes | Bash 5, Edit 10, Glob 1, Grep 4, Read 8 | 0 | 0 | 22 / 16903 / 241049 / 4782 | 0.279 | 217 | 29 |
| missing-account-ts | grep | 1 | yes | Bash 2, Edit 7, Grep 2, Read 7 | 0 | 0 | 12 / 10738 / 67904 / 2667 | 0.153 | 44 | 19 |
| missing-account-ts | index-hook | 1 | yes | Bash 2, Edit 7, Grep 2, Read 7 | 0 | 0 | 10 / 10605 / 80320 / 2561 | 0.152 | 40 | 19 |

#### Changes at many places, run by run

The places still missed when the agent first ran a check (— when it never did) and at its end, its checks (failed), its cycles « check → fix » and the commands refused.

| Task | Mode | # | Right | Places | Missed first | Missed end | Checks (failed) | Cycles | Refused |
|---|---|---|---|---|---|---|---|---|---|
| plugins-forget | grep | 1 | yes | 16 | 0 | 0 | 3 (0) | 0 | 0 |
| plugins-forget | index-hook | 1 | yes | 16 | 0 | 0 | 2 (0) | 0 | 0 |
| account-skip | grep | 1 | yes | 10 | 0 | 0 | 4 (3) | 0 | 0 |
| account-skip | index-hook | 1 | yes | 10 | 0 | 0 | 6 (3) | 0 | 0 |
| commit-root | grep | 1 | yes | 9 | 0 | 0 | 5 (0) | 1 | 2 |
| commit-root | index-hook | 1 | yes | 9 | 0 | 0 | 6 (1) | 2 | 2 |
| fact-docs-gaps | grep | 1 | yes | 21 | 4 | 0 | 5 (0) | 0 | 0 |
| fact-docs-gaps | index-hook | 1 | yes | 21 | 4 | 0 | 4 (0) | 0 | 2 |
| missing-account-ts | grep | 1 | yes | 32 | 0 | 0 | 2 (0) | 0 | 0 |
| missing-account-ts | index-hook | 1 | yes | 32 | 0 | 0 | 2 (0) | 0 | 0 |

#### Completed after the index

The runs that called the index, then searched or read the files anyway (Grep, Glob, Read after its first call).

| Task | Mode | # | Index calls | Grep / Glob / Read after |
|---|---|---|---|---|
| plugins-forget | index-hook | 1 | 1 | 2 / 0 / 5 |
| account-skip | index-hook | 1 | 4 | 1 / 0 / 9 |
| commit-root | index-hook | 1 | 4 | 7 / 0 / 14 |


#### Changes checked again

The changes refused at their run, checked again (`recheck`) by a check widened after reading them.

| Task | Mode | Model | # | Now | At the run | Why widened |
|---|---|---|---|---|---|---|
| plugins-forget | tokensave | sonnet | 1 | right | 0 place(s) missed, 1 homonym(s) touched: src-tauri/src/codeindex_links_tests.rs: homonym « plugin_runtime.forget( » touched, 7/8 left | Le 8e « plugin_runtime.forget( » de codeindex_links_tests.rs n’est pas une fixture : il est dans le test ignoré qui compte sur le dépôt les appelants de Runtime::forget. Le renommer avec la méthode est juste ; 7 fixtures gardées au lieu de 8. |
| plugins-forget | grep | haiku | 1 | right | 0 place(s) missed, 1 homonym(s) touched: src-tauri/src/codeindex_links_tests.rs: homonym « plugin_runtime.forget( » touched, 7/8 left | Le 8e « plugin_runtime.forget( » de codeindex_links_tests.rs n’est pas une fixture : il est dans le test ignoré qui compte sur le dépôt les appelants de Runtime::forget. Le renommer avec la méthode est juste ; 7 fixtures gardées au lieu de 8. |
| plugins-forget | tokensave | sonnet | 2 | right | 0 place(s) missed, 1 homonym(s) touched: src-tauri/src/codeindex_links_tests.rs: homonym « plugin_runtime.forget( » touched, 7/8 left | Le 8e « plugin_runtime.forget( » de codeindex_links_tests.rs n’est pas une fixture : il est dans le test ignoré qui compte sur le dépôt les appelants de Runtime::forget. Le renommer avec la méthode est juste ; 7 fixtures gardées au lieu de 8. |
| plugins-forget | index-hook | haiku | 2 | right | 0 place(s) missed, 1 homonym(s) touched: src-tauri/src/codeindex_links_tests.rs: homonym « plugin_runtime.forget( » touched, 7/8 left | Le 8e « plugin_runtime.forget( » de codeindex_links_tests.rs n’est pas une fixture : il est dans le test ignoré qui compte sur le dépôt les appelants de Runtime::forget. Le renommer avec la méthode est juste ; 7 fixtures gardées au lieu de 8. |
| plugins-forget | grep | haiku | 2 | right | 0 place(s) missed, 1 homonym(s) touched: src-tauri/src/codeindex_links_tests.rs: homonym « plugin_runtime.forget( » touched, 7/8 left | Le 8e « plugin_runtime.forget( » de codeindex_links_tests.rs n’est pas une fixture : il est dans le test ignoré qui compte sur le dépôt les appelants de Runtime::forget. Le renommer avec la méthode est juste ; 7 fixtures gardées au lieu de 8. |
| plugins-forget | tokensave | haiku | 2 | right | 0 place(s) missed, 1 homonym(s) touched: src-tauri/src/codeindex_links_tests.rs: homonym « plugin_runtime.forget( » touched, 7/8 left | Le 8e « plugin_runtime.forget( » de codeindex_links_tests.rs n’est pas une fixture : il est dans le test ignoré qui compte sur le dépôt les appelants de Runtime::forget. Le renommer avec la méthode est juste ; 7 fixtures gardées au lieu de 8. |
| slugify-gone | index-hook | haiku | 2 | right | `cargo test --manifest-path src-tauri/Cargo.toml --lib slug` failed: error: linking with `link.exe` failed: exit code: 1104 ⏎   | ⏎   = note: "C:\\Program Files |  |
