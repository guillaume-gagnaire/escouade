# Changelog

## 2.2.4

- A service out of reach (a name the DNS does not know, no connection, a delay passed) has an error of its own, typed as a 503: Escouade asks again later, and the message says to check the connection, the account’s address or the proxy (ESC-536).

## 2.2.3

- Its messages, README and documentation send to the settings pages of Escouade 2.2: Connected services and Advanced › Network (ESC-387).

## 2.2.2

- Its page has its documentation, in French and in English: install, use, permissions, troubleshooting and develop.

## 2.2.1

- Its page speaks of its module, not of a process.

## 2.2.0

- A WASM module (`run.wasm`), run in Escouade's host: no process of its own any more. It answers as it did. It has its own version, is
  published on the store and can be uninstalled.
- Moved to `plugins/bitbucket/` of Escouade's repository (it was `plugins/builtin/dev.escouade.bitbucket/`), with its own LICENSE
  (GPL-3.0-or-later) and this file. Its id does not change.
