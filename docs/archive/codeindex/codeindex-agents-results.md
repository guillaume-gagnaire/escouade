160 runs · 20 tasks · model claude-haiku-5-5 (medium) · commit 303fb5f2

#### Totals by mode

| | grep | index-hook | index-only | tokensave |
|---|---|---|---|---|
| Runs (right) | 40 (39) | 40 (39) | 40 (39) | 40 (37) |
| Tool calls | 260 | 255 | 324 | 301 |
| · Edit | 14 | 15 | 12 | 14 |
| · Glob | 2 | 0 | 3 | 1 |
| · Grep | 147 | 71 | 18 | 77 |
| · Read | 97 | 47 | 25 | 20 |
| · index_body | 0 | 28 | 64 | 0 |
| · index_callees | 0 | 7 | 10 | 0 |
| · index_callers | 0 | 17 | 37 | 0 |
| · index_context | 0 | 23 | 27 | 0 |
| · index_search | 0 | 47 | 128 | 0 |
| · tokensave_body | 0 | 0 | 0 | 26 |
| · tokensave_callees | 0 | 0 | 0 | 2 |
| · tokensave_callers | 0 | 0 | 0 | 23 |
| · tokensave_context | 0 | 0 | 0 | 36 |
| · tokensave_grep_placeholder_never | 0 | 0 | 0 | 1 |
| · tokensave_read | 0 | 0 | 0 | 52 |
| · tokensave_search | 0 | 0 | 0 | 49 |
| Grep after the index | 0 | 59 | 13 | 76 |
| Glob / Read after the index | 0 / 0 | 0 / 43 | 3 / 25 | 1 / 19 |
| Rereads | 32 | 14 | 9 | 4 |
| Runs completed after the index | 0 / 40 | 29 / 40 | 23 / 40 | 31 / 40 |
| Hook: searches seen / answered | 0 / 0 | 71 / 1 | 0 / 0 | 0 / 0 |
| Tokens: input | 0.5 k | 0.4 k | 0.5 k | 0.5 k |
| · cache written | 495.3 k | 410.4 k | 460.7 k | 532.6 k |
| · cache read | 2941.2 k | 3703.6 k | 4205.5 k | 5162.3 k |
| · output | 70.8 k | 64.5 k | 62.6 k | 71.3 k |
| · total | 3507.7 k | 4178.9 k | 4729.3 k | 5766.7 k |
| Tokens of the tools' answers: the index's | 0.0 k | 85.6 k | 174.3 k | 145.4 k |
| · Edit | 0.5 k | 0.5 k | 0.5 k | 0.5 k |
| · Glob | 1.7 k | 0.0 k | 2.5 k | 0.9 k |
| · Grep | 147.6 k | 61.6 k | 0.6 k | 81.3 k |
| · Read | 56.7 k | 20.2 k | 13.8 k | 8.7 k |
| · index_body | 0.0 k | 11.5 k | 26.7 k | 0.0 k |
| · index_callees | 0.0 k | 6.0 k | 14.2 k | 0.0 k |
| · index_callers | 0.0 k | 7.6 k | 15.4 k | 0.0 k |
| · index_context | 0.0 k | 41.8 k | 41.7 k | 0.0 k |
| · index_search | 0.0 k | 18.8 k | 76.2 k | 0.0 k |
| · tokensave_body | 0.0 k | 0.0 k | 0.0 k | 13.4 k |
| · tokensave_callees | 0.0 k | 0.0 k | 0.0 k | 4.3 k |
| · tokensave_callers | 0.0 k | 0.0 k | 0.0 k | 18.6 k |
| · tokensave_context | 0.0 k | 0.0 k | 0.0 k | 45.2 k |
| · tokensave_grep_placeholder_never | 0.0 k | 0.0 k | 0.0 k | 0.0 k |
| · tokensave_read | 0.0 k | 0.0 k | 0.0 k | 38.8 k |
| · tokensave_search | 0.0 k | 0.0 k | 0.0 k | 25.1 k |
| Cost ($, Claude Code's count) | 0.164 | 0.151 | 0.166 | 0.194 |
| · against grep | +0 % | -8 % | +1 % | +18 % |
| Time (s) | 604 | 599 | 561 | 630 |
| Turns | 300 | 295 | 364 | 341 |

#### By task: right, calls, cost, against grep

| Task | grep | index-hook | index-only | tokensave |
|---|---|---|---|---|
| q-modeles-harness | 2/2 · 13 calls · 0.006 $ | 2/2 · 8 calls · 0.005 $ (-25 %) | 2/2 · 7 calls · 0.003 $ (-49 %) | 2/2 · 9 calls · 0.005 $ (-21 %) |
| q-compte-limite | 1/2 · 28 calls · 0.017 $ | 2/2 · 12 calls · 0.009 $ (-46 %) | 1/2 · 16 calls · 0.009 $ (-48 %) | 2/2 · 11 calls · 0.009 $ (-47 %) |
| q-mcp-codex | 2/2 · 17 calls · 0.011 $ | 2/2 · 17 calls · 0.009 $ (-15 %) | 2/2 · 24 calls · 0.013 $ (+19 %) | 1/2 · 17 calls · 0.008 $ (-22 %) |
| q-branche-premier-message | 2/2 · 20 calls · 0.013 $ | 2/2 · 9 calls · 0.009 $ (-30 %) | 2/2 · 17 calls · 0.010 $ (-20 %) | 2/2 · 18 calls · 0.013 $ (+0 %) |
| q-reprise-limite | 2/2 · 11 calls · 0.009 $ | 2/2 · 13 calls · 0.006 $ (-24 %) | 2/2 · 13 calls · 0.011 $ (+24 %) | 2/2 · 13 calls · 0.014 $ (+63 %) |
| q-zoom-borne | 2/2 · 11 calls · 0.006 $ | 2/2 · 9 calls · 0.006 $ (-14 %) | 2/2 · 10 calls · 0.004 $ (-35 %) | 2/2 · 9 calls · 0.005 $ (-18 %) |
| q-lance-processus | 2/2 · 14 calls · 0.010 $ | 2/2 · 23 calls · 0.014 $ (+47 %) | 2/2 · 27 calls · 0.015 $ (+54 %) | 2/2 · 12 calls · 0.007 $ (-26 %) |
| q-ticket-termine | 2/2 · 16 calls · 0.011 $ | 2/2 · 14 calls · 0.009 $ (-12 %) | 2/2 · 18 calls · 0.013 $ (+22 %) | 2/2 · 12 calls · 0.009 $ (-20 %) |
| q-langue-defaut | 2/2 · 9 calls · 0.006 $ | 1/2 · 10 calls · 0.004 $ (-22 %) | 2/2 · 21 calls · 0.009 $ (+50 %) | 0/2 · 7 calls · 0.004 $ (-36 %) |
| q-permission-plugin | 2/2 · 12 calls · 0.007 $ | 2/2 · 9 calls · 0.004 $ (-46 %) | 2/2 · 15 calls · 0.005 $ (-23 %) | 2/2 · 34 calls · 0.016 $ (+136 %) |
| q-store-controles | 2/2 · 15 calls · 0.009 $ | 2/2 · 10 calls · 0.008 $ (-7 %) | 2/2 · 13 calls · 0.007 $ (-16 %) | 2/2 · 15 calls · 0.012 $ (+40 %) |
| q-sauvegarde-etat | 2/2 · 8 calls · 0.005 $ | 2/2 · 18 calls · 0.010 $ (+86 %) | 2/2 · 36 calls · 0.013 $ (+147 %) | 2/2 · 20 calls · 0.009 $ (+67 %) |
| q-worktree-cree | 2/2 · 14 calls · 0.010 $ | 2/2 · 20 calls · 0.011 $ (+3 %) | 2/2 · 16 calls · 0.006 $ (-37 %) | 2/2 · 17 calls · 0.011 $ (+10 %) |
| q-quota-compte | 2/2 · 7 calls · 0.007 $ | 2/2 · 9 calls · 0.007 $ (-3 %) | 2/2 · 10 calls · 0.006 $ (-15 %) | 2/2 · 17 calls · 0.010 $ (+46 %) |
| q-cree-agent | 2/2 · 8 calls · 0.005 $ | 2/2 · 3 calls · 0.003 $ (-46 %) | 2/2 · 3 calls · 0.003 $ (-49 %) | 2/2 · 8 calls · 0.005 $ (-9 %) |
| c-zoom-225 | 2/2 · 13 calls · 0.006 $ | 2/2 · 18 calls · 0.007 $ (+23 %) | 2/2 · 15 calls · 0.007 $ (+26 %) | 2/2 · 16 calls · 0.008 $ (+38 %) |
| c-sauvegarde-250 | 2/2 · 15 calls · 0.008 $ | 2/2 · 12 calls · 0.008 $ (+5 %) | 2/2 · 24 calls · 0.012 $ (+53 %) | 2/2 · 30 calls · 0.023 $ (+192 %) |
| c-marge-reprise | 2/2 · 11 calls · 0.009 $ | 2/2 · 18 calls · 0.012 $ (+25 %) | 2/2 · 17 calls · 0.007 $ (-29 %) | 2/2 · 13 calls · 0.011 $ (+19 %) |
| c-locale-fra | 2/2 · 11 calls · 0.007 $ | 2/2 · 14 calls · 0.007 $ (-6 %) | 2/2 · 10 calls · 0.007 $ (+2 %) | 2/2 · 16 calls · 0.011 $ (+53 %) |
| c-ansi-noir | 2/2 · 7 calls · 0.003 $ | 2/2 · 9 calls · 0.004 $ (+30 %) | 2/2 · 12 calls · 0.006 $ (+66 %) | 2/2 · 7 calls · 0.004 $ (+30 %) |

#### Every run

| Task | Mode | # | Right | Calls by tool | Grep after | Rereads | Tokens (in / cache w / cache r / out) | $ | s | Turns |
|---|---|---|---|---|---|---|---|---|---|---|
| q-modeles-harness | grep | 1 | yes | Grep 3, Read 3 | 0 | 1 | 10 / 11629 / 60052 / 1481 | 0.004 | 14 | 7 |
| q-modeles-harness | grep | 2 | yes | Grep 6, Read 1 | 0 | 0 | 12 / 5360 / 64425 / 1587 | 0.003 | 10 | 8 |
| q-modeles-harness | index-hook | 1 | yes | Grep 2, Read 1, index_body 1, index_search 1 | 2 | 0 | 8 / 9581 / 61932 / 1075 | 0.003 | 11 | 6 |
| q-modeles-harness | index-hook | 2 | yes | Grep 1, index_body 1, index_search 1 | 1 | 0 | 6 / 3501 / 44951 / 795 | 0.002 | 8 | 4 |
| q-modeles-harness | index-only | 1 | yes | Grep 1, index_body 1, index_search 1 | 1 | 0 | 6 / 4602 / 36433 / 808 | 0.002 | 8 | 4 |
| q-modeles-harness | index-only | 2 | yes | index_body 1, index_callers 1, index_search 2 | 0 | 0 | 6 / 3155 / 40085 / 843 | 0.001 | 6 | 5 |
| q-modeles-harness | tokensave | 1 | yes | Grep 2, tokensave_body 1, tokensave_context 1, tokensave_read 1 | 2 | 0 | 10 / 7612 / 92164 / 1128 | 0.003 | 12 | 6 |
| q-modeles-harness | tokensave | 2 | yes | tokensave_body 1, tokensave_context 1, tokensave_read 1, tokensave_search 1 | 0 | 0 | 6 / 4037 / 49984 / 1132 | 0.002 | 8 | 5 |
| q-compte-limite | grep | 1 | yes | Grep 6, Read 9 | 0 | 4 | 20 / 25513 / 175841 / 3885 | 0.009 | 27 | 16 |
| q-compte-limite | grep | 2 | no (the answer misses what was asked) | Grep 6, Read 7 | 0 | 3 | 24 / 20241 / 228925 / 2778 | 0.008 | 27 | 14 |
| q-compte-limite | index-hook | 1 | yes | Grep 2, index_body 2, index_callers 1, index_context 1, index_search 1 | 2 | 0 | 10 / 17724 / 103871 / 1293 | 0.005 | 7 | 8 |
| q-compte-limite | index-hook | 2 | yes | Grep 1, index_body 2, index_callees 1, index_context 1 | 1 | 0 | 8 / 11772 / 73969 / 1091 | 0.004 | 11 | 6 |
| q-compte-limite | index-only | 1 | yes | Grep 1, index_body 1, index_callers 1, index_context 1, index_search 1 | 1 | 0 | 8 / 11166 / 61904 / 1017 | 0.003 | 15 | 6 |
| q-compte-limite | index-only | 2 | no (the answer misses what was asked) | index_body 2, index_context 1, index_search 8 | 0 | 0 | 14 / 14226 / 128706 / 2040 | 0.005 | 19 | 12 |
| q-compte-limite | tokensave | 1 | yes | Grep 2, Read 1, tokensave_body 1, tokensave_context 1 | 2 | 0 | 8 / 14905 / 84426 / 1403 | 0.005 | 12 | 6 |
| q-compte-limite | tokensave | 2 | yes | Grep 1, Read 1, tokensave_callers 1, tokensave_context 1, tokensave_read 1, tokensave_search 1 | 1 | 0 | 10 / 12596 / 109104 / 1276 | 0.004 | 11 | 7 |
| q-mcp-codex | grep | 1 | yes | Grep 7, Read 3 | 0 | 1 | 16 / 16608 / 124097 / 3050 | 0.006 | 19 | 11 |
| q-mcp-codex | grep | 2 | yes | Grep 5, Read 2 | 0 | 1 | 10 / 15493 / 61826 / 1804 | 0.005 | 14 | 8 |
| q-mcp-codex | index-hook | 1 | yes | Grep 2, Read 1, index_body 2, index_search 3 | 1 | 0 | 14 / 14395 / 145013 / 2171 | 0.005 | 22 | 9 |
| q-mcp-codex | index-hook | 2 | yes | Grep 4, Read 3, index_context 1, index_search 1 | 4 | 1 | 14 / 7105 / 121744 / 2102 | 0.004 | 21 | 10 |
| q-mcp-codex | index-only | 1 | yes | Grep 1, index_body 5, index_search 7 | 1 | 0 | 20 / 19156 / 219569 / 2088 | 0.007 | 17 | 14 |
| q-mcp-codex | index-only | 2 | yes | Grep 1, index_body 3, index_callees 1, index_search 6 | 0 | 0 | 14 / 15805 / 134648 / 2322 | 0.006 | 15 | 12 |
| q-mcp-codex | tokensave | 1 | yes | Grep 6, Read 1, tokensave_context 1, tokensave_read 2 | 6 | 0 | 14 / 13232 / 144476 / 2505 | 0.005 | 18 | 11 |
| q-mcp-codex | tokensave | 2 | no (the answer misses what was asked) | Grep 2, Read 2, tokensave_body 1, tokensave_context 1, tokensave_search 1 | 2 | 1 | 10 / 6540 / 93201 / 1595 | 0.003 | 14 | 8 |
| q-branche-premier-message | grep | 1 | yes | Grep 8, Read 5 | 0 | 3 | 20 / 21153 / 156429 / 3135 | 0.007 | 18 | 14 |
| q-branche-premier-message | grep | 2 | yes | Grep 5, Read 2 | 0 | 1 | 14 / 17176 / 111048 / 2048 | 0.006 | 19 | 8 |
| q-branche-premier-message | index-hook | 1 | yes | Grep 2, Read 1, index_context 1 | 2 | 0 | 8 / 18634 / 76157 / 1295 | 0.005 | 8 | 5 |
| q-branche-premier-message | index-hook | 2 | yes | index_body 2, index_callees 1, index_context 1, index_search 1 | 0 | 0 | 8 / 12152 / 73508 / 1453 | 0.004 | 14 | 6 |
| q-branche-premier-message | index-only | 1 | yes | index_body 2, index_callees 1, index_context 1, index_search 4 | 0 | 0 | 14 / 15354 / 135031 / 1572 | 0.005 | 16 | 9 |
| q-branche-premier-message | index-only | 2 | yes | index_body 2, index_callees 1, index_callers 1, index_context 1, index_search 4 | 0 | 0 | 12 / 15231 / 118271 / 1693 | 0.005 | 15 | 10 |
| q-branche-premier-message | tokensave | 1 | yes | Grep 2, tokensave_body 1, tokensave_callees 1, tokensave_callers 2, tokensave_context 1, tokensave_read 1, tokensave_search 2 | 2 | 0 | 14 / 23760 / 180630 / 2204 | 0.008 | 22 | 11 |
| q-branche-premier-message | tokensave | 2 | yes | Grep 2, tokensave_body 2, tokensave_context 1, tokensave_search 3 | 2 | 0 | 12 / 15143 / 143937 / 1718 | 0.005 | 15 | 9 |
| q-reprise-limite | grep | 1 | yes | Grep 4, Read 3 | 0 | 2 | 14 / 16827 / 92024 / 1827 | 0.005 | 18 | 8 |
| q-reprise-limite | grep | 2 | yes | Grep 3, Read 1 | 0 | 0 | 10 / 10210 / 58340 / 1375 | 0.003 | 14 | 5 |
| q-reprise-limite | index-hook | 1 | yes | Grep 1, Read 3, index_callers 3, index_context 1, index_search 1 | 1 | 2 | 12 / 9720 / 105774 / 1698 | 0.004 | 30 | 10 |
| q-reprise-limite | index-hook | 2 | yes | Grep 1, index_body 1, index_callers 1, index_search 1 | 1 | 0 | 6 / 8351 / 47501 / 905 | 0.003 | 6 | 5 |
| q-reprise-limite | index-only | 1 | yes | index_body 2, index_callers 2, index_context 1, index_search 1 | 0 | 0 | 8 / 25144 / 55605 / 1308 | 0.006 | 11 | 7 |
| q-reprise-limite | index-only | 2 | yes | index_body 1, index_callers 3, index_context 1, index_search 2 | 0 | 0 | 8 / 14572 / 72479 / 1386 | 0.004 | 12 | 8 |
| q-reprise-limite | tokensave | 1 | yes | Grep 3, tokensave_callers 2, tokensave_context 1, tokensave_read 2 | 3 | 0 | 12 / 34695 / 137030 / 1598 | 0.009 | 14 | 9 |
| q-reprise-limite | tokensave | 2 | yes | Grep 2, tokensave_context 1, tokensave_read 2 | 2 | 0 | 8 / 16028 / 88081 / 1420 | 0.005 | 13 | 6 |
| q-zoom-borne | grep | 1 | yes | Grep 4, Read 1 | 0 | 0 | 8 / 8106 / 35898 / 1549 | 0.003 | 10 | 6 |
| q-zoom-borne | grep | 2 | yes | Grep 3, Read 3 | 0 | 1 | 10 / 10972 / 57128 / 1797 | 0.004 | 17 | 7 |
| q-zoom-borne | index-hook | 1 | yes | Grep 2, Read 2, index_search 1 | 1 | 0 | 10 / 9173 / 79847 / 1579 | 0.003 | 31 | 6 |
| q-zoom-borne | index-hook | 2 | yes | Grep 1, Read 2, index_search 1 | 0 | 0 | 6 / 5075 / 44219 / 1242 | 0.002 | 8 | 5 |
| q-zoom-borne | index-only | 1 | yes | index_body 3, index_context 1, index_search 1 | 0 | 0 | 6 / 7625 / 38459 / 1311 | 0.003 | 7 | 6 |
| q-zoom-borne | index-only | 2 | yes | Grep 1, index_body 2, index_callers 1, index_search 1 | 1 | 0 | 6 / 3440 / 39679 / 1055 | 0.002 | 8 | 6 |
| q-zoom-borne | tokensave | 1 | yes | Grep 1, tokensave_body 3, tokensave_search 1 | 1 | 0 | 6 / 5500 / 49016 / 933 | 0.002 | 9 | 6 |
| q-zoom-borne | tokensave | 2 | yes | Grep 1, tokensave_context 1, tokensave_read 1, tokensave_search 1 | 1 | 0 | 8 / 7898 / 71767 / 1858 | 0.003 | 13 | 5 |
| q-lance-processus | grep | 1 | yes | Grep 3, Read 3 | 0 | 1 | 12 / 14621 / 78205 / 1971 | 0.005 | 20 | 7 |
| q-lance-processus | grep | 2 | yes | Grep 4, Read 4 | 0 | 2 | 14 / 13877 / 104879 / 2086 | 0.005 | 19 | 9 |
| q-lance-processus | index-hook | 1 | yes | Grep 3, Read 1, index_callees 3, index_context 1, index_search 1 | 3 | 0 | 14 / 16182 / 131802 / 2078 | 0.006 | 17 | 10 |
| q-lance-processus | index-hook | 2 | yes | Grep 4, Read 2, index_body 1, index_callers 4, index_context 2, index_search 1 | 4 | 1 | 22 / 19214 / 273070 / 3679 | 0.008 | 22 | 15 |
| q-lance-processus | index-only | 1 | yes | index_body 5, index_callers 5, index_context 3, index_search 4 | 0 | 0 | 24 / 24016 / 276211 / 2889 | 0.009 | 22 | 18 |
| q-lance-processus | index-only | 2 | yes | Grep 1, index_body 1, index_callees 3, index_callers 3, index_context 1, index_search 1 | 1 | 0 | 16 / 16300 / 142791 / 2085 | 0.006 | 13 | 11 |
| q-lance-processus | tokensave | 1 | yes | tokensave_body 4, tokensave_callers 1, tokensave_context 1 | 0 | 0 | 8 / 11197 / 70734 / 1449 | 0.004 | 12 | 7 |
| q-lance-processus | tokensave | 2 | yes | Grep 2, tokensave_context 1, tokensave_read 3 | 2 | 0 | 10 / 8428 / 98511 / 1490 | 0.003 | 16 | 7 |
| q-ticket-termine | grep | 1 | yes | Grep 5, Read 1 | 0 | 0 | 12 / 17901 / 74244 / 1446 | 0.005 | 14 | 7 |
| q-ticket-termine | grep | 2 | yes | Grep 7, Read 3 | 0 | 2 | 14 / 17106 / 103220 / 2374 | 0.006 | 35 | 11 |
| q-ticket-termine | index-hook | 1 | yes | Grep 4, Read 1, index_context 1, index_search 1 | 4 | 0 | 14 / 15943 / 123131 / 2105 | 0.005 | 19 | 8 |
| q-ticket-termine | index-hook | 2 | yes | index_body 1, index_context 1, index_search 5 | 0 | 0 | 10 / 11850 / 83478 / 1486 | 0.004 | 14 | 8 |
| q-ticket-termine | index-only | 1 | yes | Read 1, index_callees 1, index_context 1, index_search 6 | 0 | 0 | 12 / 24290 / 119030 / 1878 | 0.007 | 14 | 10 |
| q-ticket-termine | index-only | 2 | yes | Read 2, index_callees 1, index_callers 1, index_context 1, index_search 4 | 0 | 1 | 14 / 18564 / 147744 / 1626 | 0.006 | 16 | 10 |
| q-ticket-termine | tokensave | 1 | yes | Grep 3, tokensave_context 1, tokensave_read 1, tokensave_search 1 | 3 | 0 | 10 / 12518 / 103834 / 1492 | 0.004 | 14 | 7 |
| q-ticket-termine | tokensave | 2 | yes | tokensave_context 1, tokensave_read 1, tokensave_search 4 | 0 | 0 | 10 / 12954 / 94995 / 1379 | 0.004 | 14 | 7 |
| q-langue-defaut | grep | 1 | yes | Grep 3, Read 2 | 0 | 0 | 8 / 11024 / 43186 / 1401 | 0.003 | 10 | 6 |
| q-langue-defaut | grep | 2 | yes | Grep 2, Read 2 | 0 | 0 | 6 / 7886 / 30634 / 1026 | 0.002 | 6 | 5 |
| q-langue-defaut | index-hook | 1 | yes | index_body 3, index_callers 1, index_context 1, index_search 2 | 0 | 0 | 12 / 8109 / 96843 / 1072 | 0.003 | 11 | 8 |
| q-langue-defaut | index-hook | 2 | no (the answer misses what was asked) | Grep 2, index_context 1 | 2 | 0 | 6 / 2941 / 44355 / 675 | 0.001 | 8 | 4 |
| q-langue-defaut | index-only | 1 | yes | index_body 2, index_callers 2, index_context 1, index_search 1 | 0 | 0 | 10 / 9248 / 74444 / 983 | 0.003 | 8 | 7 |
| q-langue-defaut | index-only | 2 | yes | Glob 1, Grep 1, Read 1, index_body 3, index_context 1, index_search 8 | 1 | 0 | 22 / 12959 / 189982 / 2093 | 0.006 | 24 | 16 |
| q-langue-defaut | tokensave | 1 | no (the answer misses what was asked) | Grep 1, tokensave_context 1, tokensave_read 1 | 1 | 0 | 6 / 4186 / 47745 / 879 | 0.002 | 5 | 4 |
| q-langue-defaut | tokensave | 2 | no (the answer misses what was asked) | Grep 1, Read 1, tokensave_body 1, tokensave_context 1 | 1 | 0 | 8 / 3670 / 67946 / 1013 | 0.002 | 9 | 5 |
| q-permission-plugin | grep | 1 | yes | Grep 3, Read 1 | 0 | 0 | 10 / 7394 / 45997 / 1082 | 0.002 | 9 | 5 |
| q-permission-plugin | grep | 2 | yes | Grep 5, Read 3 | 0 | 1 | 12 / 11807 / 82426 / 1848 | 0.004 | 18 | 9 |
| q-permission-plugin | index-hook | 1 | yes | index_body 2, index_search 3 | 0 | 0 | 10 / 5036 / 71303 / 909 | 0.002 | 7 | 6 |
| q-permission-plugin | index-hook | 2 | yes | index_body 2, index_search 2 | 0 | 0 | 8 / 2070 / 58445 / 717 | 0.001 | 11 | 5 |
| q-permission-plugin | index-only | 1 | yes | index_body 3, index_callees 1, index_callers 1, index_search 5 | 0 | 0 | 16 / 8434 / 116480 / 1367 | 0.004 | 9 | 11 |
| q-permission-plugin | index-only | 2 | yes | index_body 2, index_callees 1, index_search 2 | 0 | 0 | 8 / 2916 / 53855 / 838 | 0.002 | 9 | 6 |
| q-permission-plugin | tokensave | 1 | yes | Grep 2, tokensave_callers 4, tokensave_context 1, tokensave_read 4, tokensave_search 3 | 2 | 0 | 18 / 19110 / 208319 / 2407 | 0.007 | 22 | 15 |
| q-permission-plugin | tokensave | 2 | yes | tokensave_body 2, tokensave_callers 4, tokensave_context 1, tokensave_grep_placeholder_never 1, tokensave_read 4, tokensave_search 8 | 0 | 0 | 26 / 18744 / 323655 / 2898 | 0.008 | 28 | 21 |
| q-store-controles | grep | 1 | yes | Glob 1, Grep 6, Read 2 | 0 | 0 | 10 / 14858 / 57397 / 2353 | 0.005 | 16 | 10 |
| q-store-controles | grep | 2 | yes | Glob 1, Grep 3, Read 2 | 0 | 0 | 8 / 12847 / 47140 / 1672 | 0.004 | 10 | 7 |
| q-store-controles | index-hook | 1 | yes | Grep 2, Read 1, index_context 1 | 2 | 0 | 8 / 13918 / 67825 / 1647 | 0.004 | 13 | 5 |
| q-store-controles | index-hook | 2 | yes | Grep 3, Read 1, index_body 1, index_context 1 | 3 | 0 | 10 / 9238 / 86725 / 1948 | 0.004 | 15 | 7 |
| q-store-controles | index-only | 1 | yes | index_body 2, index_callers 1, index_context 1, index_search 2 | 0 | 0 | 10 / 10458 / 69366 / 1286 | 0.003 | 14 | 7 |
| q-store-controles | index-only | 2 | yes | Glob 1, index_body 3, index_callers 1, index_context 1, index_search 1 | 0 | 0 | 12 / 10176 / 96472 / 1666 | 0.004 | 17 | 8 |
| q-store-controles | tokensave | 1 | yes | Grep 3, Read 1, tokensave_context 1 | 3 | 0 | 12 / 16075 / 125775 / 1794 | 0.005 | 18 | 6 |
| q-store-controles | tokensave | 2 | yes | Glob 1, Grep 2, tokensave_callers 2, tokensave_context 1, tokensave_read 4 | 2 | 0 | 12 / 21595 / 145036 / 1815 | 0.007 | 11 | 11 |
| q-sauvegarde-etat | grep | 1 | yes | Grep 2, Read 2 | 0 | 1 | 8 / 9389 / 36603 / 1374 | 0.003 | 10 | 5 |
| q-sauvegarde-etat | grep | 2 | yes | Grep 2, Read 2 | 0 | 1 | 8 / 6752 / 39706 / 1522 | 0.003 | 10 | 5 |
| q-sauvegarde-etat | index-hook | 1 | yes | Grep 3, Read 3, index_callers 1, index_context 1, index_search 1 | 3 | 2 | 12 / 13386 / 106702 / 2684 | 0.005 | 16 | 10 |
| q-sauvegarde-etat | index-hook | 2 | yes | Grep 2, Read 3, index_context 1, index_search 3 | 2 | 2 | 16 / 10943 / 157505 / 2583 | 0.005 | 19 | 10 |
| q-sauvegarde-etat | index-only | 1 | yes | Grep 1, Read 3, index_body 4, index_callers 2, index_context 2, index_search 5 | 1 | 2 | 26 / 14189 / 229399 / 2935 | 0.007 | 29 | 18 |
| q-sauvegarde-etat | index-only | 2 | yes | Read 2, index_body 4, index_callers 4, index_context 1, index_search 8 | 0 | 1 | 24 / 14252 / 227900 / 3434 | 0.007 | 22 | 20 |
| q-sauvegarde-etat | tokensave | 1 | yes | Grep 2, Read 2, tokensave_body 4, tokensave_context 1, tokensave_search 2 | 2 | 1 | 16 / 10707 / 154802 / 2375 | 0.005 | 14 | 12 |
| q-sauvegarde-etat | tokensave | 2 | yes | Grep 1, tokensave_context 2, tokensave_read 3, tokensave_search 3 | 1 | 0 | 16 / 8189 / 153279 / 2115 | 0.004 | 20 | 10 |
| q-worktree-cree | grep | 1 | yes | Grep 4, Read 5 | 0 | 3 | 18 / 19051 / 139816 / 2307 | 0.006 | 14 | 10 |
| q-worktree-cree | grep | 2 | yes | Grep 3, Read 2 | 0 | 0 | 10 / 12347 / 64993 / 1522 | 0.004 | 12 | 6 |
| q-worktree-cree | index-hook | 1 | yes | Grep 3, Read 3, index_callees 1, index_callers 1, index_context 1, index_search 1 | 3 | 1 | 14 / 17179 / 146972 / 2351 | 0.006 | 26 | 11 |
| q-worktree-cree | index-hook | 2 | yes | Grep 4, index_body 3, index_search 3 | 4 | 0 | 14 / 10833 / 131766 / 1892 | 0.004 | 21 | 11 |
| q-worktree-cree | index-only | 1 | yes | Grep 1, index_body 1, index_callers 2, index_search 4 | 1 | 0 | 12 / 10245 / 88958 / 1138 | 0.004 | 9 | 9 |
| q-worktree-cree | index-only | 2 | yes | Grep 1, index_body 1, index_callers 2, index_context 1, index_search 3 | 1 | 0 | 10 / 7287 / 73913 / 1398 | 0.003 | 13 | 9 |
| q-worktree-cree | tokensave | 1 | yes | Grep 1, tokensave_body 2, tokensave_callers 1, tokensave_context 1, tokensave_read 3, tokensave_search 2 | 1 | 0 | 14 / 17868 / 165957 / 1949 | 0.006 | 11 | 11 |
| q-worktree-cree | tokensave | 2 | yes | Grep 2, tokensave_body 1, tokensave_callees 1, tokensave_callers 1, tokensave_context 1, tokensave_read 1 | 2 | 0 | 10 / 16025 / 111917 / 1542 | 0.005 | 15 | 8 |
| q-quota-compte | grep | 1 | yes | Grep 2, Read 1 | 0 | 0 | 8 / 11152 / 38582 / 973 | 0.003 | 6 | 4 |
| q-quota-compte | grep | 2 | yes | Grep 2, Read 2 | 0 | 0 | 8 / 13843 / 49665 / 1198 | 0.004 | 12 | 5 |
| q-quota-compte | index-hook | 1 | yes | index_callees 1, index_callers 1, index_context 1, index_search 1 | 0 | 0 | 6 / 10953 / 42949 / 977 | 0.003 | 8 | 5 |
| q-quota-compte | index-hook | 2 | yes | Grep 2, index_body 1, index_callers 1, index_search 1 | 2 | 0 | 8 / 12040 / 73966 / 962 | 0.004 | 9 | 6 |
| q-quota-compte | index-only | 1 | yes | index_body 1, index_callers 1, index_context 1, index_search 2 | 0 | 0 | 8 / 10094 / 59310 / 1075 | 0.003 | 12 | 6 |
| q-quota-compte | index-only | 2 | yes | index_body 1, index_callers 1, index_context 1, index_search 2 | 0 | 0 | 8 / 8188 / 63554 / 1024 | 0.003 | 10 | 6 |
| q-quota-compte | tokensave | 1 | yes | tokensave_body 1, tokensave_callers 3, tokensave_context 1, tokensave_read 1, tokensave_search 2 | 0 | 0 | 14 / 9544 / 130889 / 1352 | 0.004 | 15 | 9 |
| q-quota-compte | tokensave | 2 | yes | Grep 2, tokensave_body 1, tokensave_callers 2, tokensave_context 1, tokensave_read 2, tokensave_search 1 | 2 | 0 | 12 / 20036 / 143208 / 1701 | 0.006 | 15 | 10 |
| q-cree-agent | grep | 1 | yes | Grep 2, Read 2 | 0 | 0 | 8 / 9696 / 39294 / 931 | 0.003 | 8 | 5 |
| q-cree-agent | grep | 2 | yes | Grep 2, Read 2 | 0 | 0 | 6 / 9229 / 32475 / 913 | 0.003 | 10 | 5 |
| q-cree-agent | index-hook | 1 | yes | index_context 1 | 0 | 0 | 4 / 5131 / 25000 / 703 | 0.002 | 7 | 2 |
| q-cree-agent | index-hook | 2 | yes | index_context 1, index_search 1 | 0 | 0 | 4 / 3005 / 27866 / 802 | 0.001 | 8 | 3 |
| q-cree-agent | index-only | 1 | yes | index_context 1 | 0 | 0 | 4 / 5248 / 22447 / 637 | 0.002 | 5 | 2 |
| q-cree-agent | index-only | 2 | yes | index_context 1, index_search 1 | 0 | 0 | 4 / 3111 / 25312 / 620 | 0.001 | 6 | 3 |
| q-cree-agent | tokensave | 1 | yes | tokensave_context 1, tokensave_read 2, tokensave_search 3 | 0 | 0 | 8 / 7782 / 67199 / 1111 | 0.003 | 7 | 7 |
| q-cree-agent | tokensave | 2 | yes | tokensave_context 1, tokensave_search 1 | 0 | 0 | 6 / 5961 / 52655 / 843 | 0.002 | 8 | 3 |
| c-zoom-225 | grep | 1 | yes | Edit 2, Grep 2, Read 3 | 0 | 2 | 10 / 9349 / 53481 / 2068 | 0.003 | 17 | 8 |
| c-zoom-225 | grep | 2 | yes | Edit 1, Grep 3, Read 2 | 0 | 1 | 10 / 6025 / 56065 / 1410 | 0.002 | 17 | 7 |
| c-zoom-225 | index-hook | 1 | yes | Edit 2, Grep 2, Read 3, index_body 1, index_search 1 | 2 | 2 | 10 / 8797 / 78089 / 2123 | 0.004 | 15 | 10 |
| c-zoom-225 | index-hook | 2 | yes | Edit 2, Grep 2, Read 4, index_search 1 | 2 | 3 | 10 / 7957 / 85434 / 2500 | 0.004 | 25 | 10 |
| c-zoom-225 | index-only | 1 | yes | Edit 1, Grep 1, Read 2, index_body 1, index_search 3 | 1 | 1 | 10 / 7845 / 70036 / 1570 | 0.003 | 13 | 9 |
| c-zoom-225 | index-only | 2 | yes | Edit 1, Read 3, index_search 3 | 0 | 2 | 10 / 12945 / 95934 / 1729 | 0.004 | 14 | 8 |
| c-zoom-225 | tokensave | 1 | yes | Edit 1, Grep 3, Read 2, tokensave_search 1 | 3 | 1 | 10 / 9742 / 96476 / 1671 | 0.004 | 14 | 8 |
| c-zoom-225 | tokensave | 2 | yes | Edit 2, Grep 3, tokensave_read 3, tokensave_search 1 | 3 | 0 | 12 / 10463 / 125317 / 2102 | 0.004 | 20 | 10 |
| c-sauvegarde-250 | grep | 1 | yes | Edit 1, Grep 3, Read 2 | 0 | 0 | 10 / 11645 / 56340 / 1814 | 0.004 | 12 | 7 |
| c-sauvegarde-250 | grep | 2 | yes | Edit 2, Grep 6, Read 1 | 0 | 0 | 12 / 10332 / 77657 / 2544 | 0.004 | 30 | 10 |
| c-sauvegarde-250 | index-hook | 1 | yes | Edit 1, Grep 3, Read 1, index_search 1 | 2 | 0 | 12 / 13534 / 116666 / 1848 | 0.005 | 17 | 7 |
| c-sauvegarde-250 | index-hook | 2 | yes | Edit 1, Grep 4, Read 1 | 0 | 0 | 10 / 8938 / 90233 / 1647 | 0.004 | 19 | 7 |
| c-sauvegarde-250 | index-only | 1 | yes | Edit 1, Grep 1, Read 1, index_context 1, index_search 4 | 1 | 0 | 12 / 10128 / 89216 / 1865 | 0.004 | 15 | 9 |
| c-sauvegarde-250 | index-only | 2 | yes | Edit 1, Grep 1, Read 2, index_body 1, index_callers 2, index_search 9 | 1 | 1 | 24 / 20034 / 287797 / 2792 | 0.008 | 37 | 17 |
| c-sauvegarde-250 | tokensave | 1 | yes | Edit 1, Grep 3, Read 1, tokensave_context 1, tokensave_read 3, tokensave_search 1 | 3 | 0 | 14 / 19418 / 169016 / 2805 | 0.007 | 21 | 11 |
| c-sauvegarde-250 | tokensave | 2 | yes | Edit 1, Grep 12, tokensave_context 1, tokensave_read 4, tokensave_search 2 | 12 | 0 | 34 / 37102 / 576691 / 5943 | 0.016 | 68 | 21 |
| c-marge-reprise | grep | 1 | yes | Edit 2, Grep 3, Read 2 | 0 | 0 | 12 / 18574 / 94507 / 1972 | 0.006 | 15 | 8 |
| c-marge-reprise | grep | 2 | yes | Edit 1, Grep 2, Read 1 | 0 | 0 | 10 / 11756 / 71019 / 1165 | 0.004 | 10 | 5 |
| c-marge-reprise | index-hook | 1 | yes | Edit 2, Grep 1, Read 2, index_callers 1, index_context 1, index_search 2 | 1 | 0 | 14 / 15678 / 138834 / 2195 | 0.006 | 19 | 10 |
| c-marge-reprise | index-hook | 2 | yes | Edit 2, Grep 2, Read 2, index_context 1, index_search 2 | 2 | 0 | 14 / 16033 / 151387 / 2602 | 0.006 | 29 | 10 |
| c-marge-reprise | index-only | 1 | yes | Edit 1, Read 1, index_body 3, index_context 1, index_search 2 | 0 | 0 | 14 / 7904 / 105050 / 1135 | 0.003 | 9 | 9 |
| c-marge-reprise | index-only | 2 | yes | Edit 2, Glob 1, Read 3, index_search 3 | 0 | 1 | 14 / 7224 / 113762 / 1690 | 0.003 | 26 | 10 |
| c-marge-reprise | tokensave | 1 | yes | Edit 1, Grep 1, Read 1, tokensave_context 1 | 1 | 0 | 10 / 11534 / 103192 / 1108 | 0.004 | 12 | 5 |
| c-marge-reprise | tokensave | 2 | yes | Edit 2, Grep 3, Read 2, tokensave_context 1, tokensave_search 1 | 3 | 0 | 16 / 19440 / 199181 / 2498 | 0.007 | 18 | 10 |
| c-locale-fra | grep | 1 | yes | Edit 1, Grep 2, Read 1 | 0 | 0 | 10 / 10360 / 55194 / 1197 | 0.003 | 13 | 5 |
| c-locale-fra | grep | 2 | yes | Edit 2, Grep 3, Read 2 | 0 | 1 | 10 / 9216 / 60953 / 2527 | 0.004 | 21 | 8 |
| c-locale-fra | index-hook | 1 | yes | Edit 1, Grep 1, Read 1, index_body 2, index_callers 2, index_search 1 | 1 | 0 | 12 / 10326 / 104047 / 1473 | 0.004 | 9 | 9 |
| c-locale-fra | index-hook | 2 | yes | Edit 2, Grep 1, Read 2, index_search 1 | 1 | 0 | 8 / 5351 / 62244 / 1961 | 0.003 | 15 | 7 |
| c-locale-fra | index-only | 1 | yes | Edit 1, Read 1, index_body 1, index_search 2 | 0 | 0 | 10 / 9794 / 78014 / 1355 | 0.003 | 13 | 6 |
| c-locale-fra | index-only | 2 | yes | Edit 2, Grep 1, Read 1, index_search 1 | 1 | 0 | 8 / 10862 / 62039 / 1743 | 0.004 | 14 | 6 |
| c-locale-fra | tokensave | 1 | yes | Edit 2, Grep 4, Read 2, tokensave_context 1 | 4 | 1 | 12 / 20877 / 135693 / 2576 | 0.007 | 24 | 10 |
| c-locale-fra | tokensave | 2 | yes | Edit 2, Grep 1, Read 2, tokensave_context 1, tokensave_search 1 | 1 | 0 | 12 / 7801 / 108344 / 2246 | 0.004 | 18 | 8 |
| c-ansi-noir | grep | 1 | yes | Edit 1, Grep 1, Read 1 | 0 | 0 | 8 / 5131 / 33782 / 818 | 0.002 | 8 | 4 |
| c-ansi-noir | grep | 2 | yes | Edit 1, Grep 2, Read 1 | 0 | 0 | 10 / 2801 / 47706 / 997 | 0.002 | 16 | 5 |
| c-ansi-noir | index-hook | 1 | yes | Edit 1, Grep 2, Read 2 | 0 | 0 | 12 / 6085 / 88948 / 1120 | 0.003 | 10 | 6 |
| c-ansi-noir | index-hook | 2 | yes | Edit 1, Grep 2, Read 1 | 0 | 0 | 8 / 2529 / 59489 / 1097 | 0.002 | 15 | 5 |
| c-ansi-noir | index-only | 1 | yes | Edit 1, Grep 2, Read 1, index_search 2 | 0 | 0 | 10 / 8694 / 71257 / 1056 | 0.003 | 6 | 7 |
| c-ansi-noir | index-only | 2 | yes | Edit 1, Grep 2, Read 1, index_search 2 | 0 | 0 | 10 / 5848 / 74346 / 1212 | 0.003 | 13 | 7 |
| c-ansi-noir | tokensave | 1 | yes | Edit 1, tokensave_read 1, tokensave_search 2 | 0 | 0 | 8 / 7470 / 71817 / 1174 | 0.003 | 11 | 5 |
| c-ansi-noir | tokensave | 2 | yes | Edit 1, Grep 1, Read 1 | 0 | 0 | 8 / 2220 / 66342 / 785 | 0.002 | 8 | 4 |

#### Completed after the index

The runs that called the index, then searched or read the files anyway (Grep, Glob, Read after its first call).

| Task | Mode | # | Index calls | Grep / Glob / Read after |
|---|---|---|---|---|
| q-reprise-limite | index-hook | 1 | 5 | 1 / 0 / 3 |
| q-reprise-limite | tokensave | 1 | 5 | 3 / 0 / 0 |
| q-ticket-termine | index-hook | 1 | 2 | 4 / 0 / 1 |
| q-ticket-termine | tokensave | 1 | 3 | 3 / 0 / 0 |
| q-ticket-termine | index-only | 1 | 8 | 0 / 0 / 1 |
| c-sauvegarde-250 | index-hook | 1 | 1 | 2 / 0 / 1 |
| c-sauvegarde-250 | index-only | 1 | 5 | 1 / 0 / 1 |
| c-sauvegarde-250 | tokensave | 1 | 5 | 3 / 0 / 1 |
| q-modeles-harness | index-only | 1 | 2 | 1 / 0 / 0 |
| q-modeles-harness | tokensave | 1 | 3 | 2 / 0 / 0 |
| q-modeles-harness | index-hook | 1 | 2 | 2 / 0 / 1 |
| q-compte-limite | index-hook | 1 | 5 | 2 / 0 / 0 |
| q-compte-limite | index-only | 1 | 4 | 1 / 0 / 0 |
| q-compte-limite | tokensave | 1 | 2 | 2 / 0 / 1 |
| q-mcp-codex | index-hook | 1 | 5 | 1 / 0 / 1 |
| q-mcp-codex | index-only | 1 | 12 | 1 / 0 / 0 |
| q-mcp-codex | tokensave | 1 | 3 | 6 / 0 / 1 |
| q-branche-premier-message | index-hook | 1 | 1 | 2 / 0 / 1 |
| q-branche-premier-message | tokensave | 1 | 8 | 2 / 0 / 0 |
| q-zoom-borne | tokensave | 1 | 4 | 1 / 0 / 0 |
| q-zoom-borne | index-hook | 1 | 1 | 1 / 0 / 2 |
| q-langue-defaut | tokensave | 1 | 2 | 1 / 0 / 0 |
| q-lance-processus | index-hook | 1 | 5 | 3 / 0 / 1 |
| q-permission-plugin | tokensave | 1 | 12 | 2 / 0 / 0 |
| q-store-controles | index-hook | 1 | 1 | 2 / 0 / 1 |
| q-store-controles | tokensave | 1 | 1 | 3 / 0 / 1 |
| q-sauvegarde-etat | tokensave | 1 | 7 | 2 / 0 / 2 |
| q-sauvegarde-etat | index-hook | 1 | 3 | 3 / 0 / 3 |
| q-sauvegarde-etat | index-only | 1 | 13 | 1 / 0 / 3 |
| q-worktree-cree | index-only | 1 | 7 | 1 / 0 / 0 |
| q-worktree-cree | tokensave | 1 | 9 | 1 / 0 / 0 |
| q-worktree-cree | index-hook | 1 | 4 | 3 / 0 / 3 |
| c-zoom-225 | index-only | 1 | 4 | 1 / 0 / 2 |
| c-zoom-225 | tokensave | 1 | 1 | 3 / 0 / 2 |
| c-zoom-225 | index-hook | 1 | 2 | 2 / 0 / 3 |
| c-marge-reprise | index-only | 1 | 6 | 0 / 0 / 1 |
| c-marge-reprise | tokensave | 1 | 1 | 1 / 0 / 1 |
| c-locale-fra | index-only | 1 | 3 | 0 / 0 / 1 |
| c-marge-reprise | index-hook | 1 | 4 | 1 / 0 / 2 |
| c-ansi-noir | index-only | 1 | 2 | 0 / 0 / 1 |
| c-locale-fra | index-hook | 1 | 5 | 1 / 0 / 1 |
| c-locale-fra | tokensave | 1 | 1 | 4 / 0 / 2 |
| q-modeles-harness | index-hook | 2 | 2 | 1 / 0 / 0 |
| q-compte-limite | index-hook | 2 | 4 | 1 / 0 / 0 |
| q-compte-limite | tokensave | 2 | 4 | 1 / 0 / 1 |
| q-mcp-codex | tokensave | 2 | 3 | 2 / 0 / 2 |
| q-mcp-codex | index-hook | 2 | 2 | 4 / 0 / 3 |
| q-branche-premier-message | tokensave | 2 | 6 | 2 / 0 / 0 |
| q-zoom-borne | index-only | 2 | 4 | 1 / 0 / 0 |
| q-reprise-limite | index-hook | 2 | 3 | 1 / 0 / 0 |
| q-zoom-borne | index-hook | 2 | 1 | 0 / 0 / 2 |
| q-lance-processus | index-only | 2 | 9 | 1 / 0 / 0 |
| q-reprise-limite | tokensave | 2 | 3 | 2 / 0 / 0 |
| q-zoom-borne | tokensave | 2 | 3 | 1 / 0 / 0 |
| q-ticket-termine | index-only | 2 | 7 | 0 / 0 / 2 |
| q-lance-processus | index-hook | 2 | 8 | 4 / 0 / 2 |
| q-lance-processus | tokensave | 2 | 4 | 2 / 0 / 0 |
| q-langue-defaut | index-only | 2 | 12 | 1 / 1 / 1 |
| q-langue-defaut | index-hook | 2 | 1 | 2 / 0 / 0 |
| q-langue-defaut | tokensave | 2 | 2 | 1 / 0 / 1 |
| q-store-controles | index-only | 2 | 6 | 0 / 1 / 0 |
| q-store-controles | index-hook | 2 | 2 | 3 / 0 / 1 |
| q-sauvegarde-etat | index-only | 2 | 17 | 0 / 0 / 2 |
| q-sauvegarde-etat | index-hook | 2 | 4 | 2 / 0 / 3 |
| q-store-controles | tokensave | 2 | 7 | 2 / 1 / 0 |
| q-worktree-cree | index-only | 2 | 7 | 1 / 0 / 0 |
| q-worktree-cree | index-hook | 2 | 6 | 4 / 0 / 0 |
| q-sauvegarde-etat | tokensave | 2 | 8 | 1 / 0 / 0 |
| q-quota-compte | index-hook | 2 | 3 | 2 / 0 / 0 |
| c-zoom-225 | index-only | 2 | 3 | 0 / 0 / 3 |
| q-worktree-cree | tokensave | 2 | 5 | 2 / 0 / 0 |
| q-quota-compte | tokensave | 2 | 7 | 2 / 0 / 0 |
| c-zoom-225 | index-hook | 2 | 1 | 2 / 0 / 4 |
| c-sauvegarde-250 | index-only | 2 | 12 | 1 / 0 / 2 |
| c-zoom-225 | tokensave | 2 | 4 | 3 / 0 / 0 |
| c-marge-reprise | index-only | 2 | 3 | 0 / 1 / 3 |
| c-marge-reprise | index-hook | 2 | 3 | 2 / 0 / 2 |
| c-locale-fra | index-only | 2 | 1 | 1 / 0 / 1 |
| c-locale-fra | index-hook | 2 | 1 | 1 / 0 / 2 |
| c-ansi-noir | index-only | 2 | 2 | 0 / 0 / 1 |
| c-sauvegarde-250 | tokensave | 2 | 7 | 12 / 0 / 0 |
| c-marge-reprise | tokensave | 2 | 2 | 3 / 0 / 2 |
| c-locale-fra | tokensave | 2 | 2 | 1 / 0 / 2 |

#### Checked again

The runs whose verdict changed with a check widened after their answers were read.

| Task | Mode | # | At the run | Now | Why |
|---|---|---|---|---|---|
| q-store-controles | grep | 1 | wrong | right | checkArchive de checks.ts, que runChecks appelle, est juste (relu le 2026-10-30) |
| q-quota-compte | tokensave | 2 | wrong | right | refresh_usage relit le quota de chaque compte (relu le 2026-10-30) |
