//! The page of the browser's client (BC-1, ESC-704, `docs/REMOTE.md` « Le client navigateur »), embedded in
//! `escouade-server` alone: its `main` hands it to the core (`escouade_lib::server_main_with`); the desktop app never
//! names this crate, so its binary does not carry it.
//!
//! `build.rs` reads the folder `ESCOUADE_WEB_DIST` names (`dist-web/`, built by `npm run build:web`; a relative path is
//! the repository's): `index.html`, `assets/` and what Vite copies of `public/` (What's up ?, the tour's images, the
//! logo), each file compressed by gzip. Without the variable, no page ([`FILES`] is empty): a build of development or
//! of the tests, which `escouade-server --browser` says it cannot serve. The release sets it
//! (`.github/workflows/release.yml`), and `scripts/server-package.sh` refuses a server without the page.

/// A file of the page: its path in `dist-web/` (`index.html`, `assets/<name>`, `whats-up/whats-up.js`…), and its bytes
/// compressed by gzip.
pub type File = (&'static str, &'static [u8]);

include!(concat!(env!("OUT_DIR"), "/page.rs"));

#[cfg(test)]
mod tests {
    #[test]
    fn the_files_are_index_html_first_then_names_served_as_they_are() {
        if let Some((first, _)) = super::FILES.first() {
            assert_eq!(*first, "index.html");
        }
        for (name, gzip) in super::FILES {
            assert!(
                !name.is_empty()
                    && name
                        .split('/')
                        .all(|s| !s.is_empty() && !s.starts_with('.') && !s.contains(['\\', '%'])),
                "{name}"
            );
            // gzip's magic number.
            assert_eq!(&gzip[..2], &[0x1f, 0x8b], "{name}");
        }
        assert_eq!(super::MARK.is_empty(), super::FILES.is_empty());
    }
}
