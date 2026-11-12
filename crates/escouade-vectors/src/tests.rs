//! The embeddings and the index, on the fake model; the real ones on a model named by `ESCOUADE_TEST_REAL_VECTORS`.

use super::embed::{Embedder, Kind, Onnx, Prefixes};
use super::eval::{PASSAGES, QUERIES};
use std::path::PathBuf;
use std::time::Instant;

/// Quality of a model on the set: the rank of the right passage for each question.
fn measure(e: &dyn Embedder) -> (f32, f32, f32, f64) {
    let started = Instant::now();
    let texts: Vec<String> = PASSAGES.iter().map(|p| p.1.to_string()).collect();
    let docs = e.embed(&texts, Kind::Passage).unwrap();
    let secs = started.elapsed().as_secs_f64();
    let qs: Vec<String> = QUERIES.iter().map(|q| q.0.to_string()).collect();
    let qv = e.embed(&qs, Kind::Query).unwrap();
    let (mut r1, mut r3, mut mrr) = (0f32, 0f32, 0f32);
    for (q, (_, want)) in qv.iter().zip(QUERIES) {
        let mut order: Vec<usize> = (0..docs.len()).collect();
        order.sort_by(|&a, &b| {
            super::embed::dot(q, &docs[b]).total_cmp(&super::embed::dot(q, &docs[a]))
        });
        let rank = order.iter().position(|&i| PASSAGES[i].0 == *want).unwrap();
        r1 += (rank == 0) as u8 as f32;
        r3 += (rank < 3) as u8 as f32;
        mrr += 1.0 / (rank + 1) as f32;
    }
    let n = QUERIES.len() as f32;
    (r1 / n, r3 / n, mrr / n, PASSAGES.len() as f64 / secs)
}

#[test]
#[ignore = "needs models: ESCOUADE_TEST_REAL_VECTORS_CANDIDATES=<folder of model folders>"]
fn candidates_compared() {
    let Ok(root) = std::env::var("ESCOUADE_TEST_REAL_VECTORS_CANDIDATES") else {
        return;
    };
    for entry in std::fs::read_dir(PathBuf::from(root)).unwrap() {
        let dir = entry.unwrap().path();
        let name = dir.file_name().unwrap().to_string_lossy().to_string();
        let prefixes = if name.contains("e5") {
            Prefixes {
                query: "query: ",
                passage: "passage: ",
            }
        } else {
            Prefixes {
                query: "",
                passage: "",
            }
        };
        let started = Instant::now();
        let e = Onnx::load(
            &name,
            &dir.join("model_int8.onnx"),
            &dir.join("tokenizer.json"),
            prefixes,
        )
        .unwrap();
        let load = started.elapsed();
        let (r1, r3, mrr, per_s) = measure(&e);
        eprintln!("{name}: dim {} load {load:?} recall@1 {r1:.2} recall@3 {r3:.2} mrr {mrr:.3} {per_s:.1} passages/s", e.dim());
    }
}

mod fake {
    use super::super::chunk::{chunks, Format};
    use super::super::embed::{dot, Embedder, Fake, Kind};
    use super::super::index::{Index, Source};
    use std::sync::Arc;

    fn index() -> Index {
        Index::in_memory(Arc::new(Fake)).unwrap()
    }

    #[test]
    fn the_fake_model_is_deterministic_and_unit_length() {
        let a = Fake
            .embed(&["Le cache de prompt expire".into()], Kind::Passage)
            .unwrap();
        let b = Fake
            .embed(&["Le cache de prompt expire".into()], Kind::Query)
            .unwrap();
        assert_eq!(a, b);
        assert!((dot(&a[0], &a[0]) - 1.0).abs() < 1e-5);
        let c = Fake
            .embed(&["tout autre sujet".into()], Kind::Passage)
            .unwrap();
        assert!(dot(&a[0], &c[0]) < 0.2);
    }

    #[test]
    fn markdown_is_cut_at_its_headings_and_keeps_their_path() {
        let md = "intro\n\n# A\n\nun\n\n## B\n\ndeux\n\n```\n# not a heading\n\nstill code\n```\n\n# C\n\ntrois\n";
        let c = chunks(md, Format::Markdown);
        let titles: Vec<&str> = c.iter().map(|c| c.title.as_str()).collect();
        assert_eq!(titles, ["", "A", "A › B", "C"]);
        assert!(c[2].text.contains("# not a heading\n\nstill code"));
        assert!(c[2].embedded().starts_with("A › B\n"));
    }

    #[test]
    fn text_is_cut_at_paragraphs_joined_up_to_a_size_and_long_ones_are_cut() {
        let small = "un\n\ndeux\n\ntrois";
        assert_eq!(chunks(small, Format::Text).len(), 1);
        let para = "mot ".repeat(200);
        let text = format!("{para}\n\n{para}\n\n{para}");
        let c = chunks(&text, Format::Text);
        assert!(c.len() >= 3);
        assert!(c.iter().all(|c| c.text.chars().count() <= 1400));
        let huge = "x".repeat(5000);
        let c = chunks(&huge, Format::Code);
        assert_eq!(c.len(), 4);
        assert!(c.iter().all(|c| c.text.chars().count() <= 1400));
        assert!(chunks("  \n\n ", Format::Text).is_empty());
    }

    #[test]
    fn the_format_comes_from_the_name() {
        assert_eq!(Format::of_path("docs/a.md"), Format::Markdown);
        assert_eq!(Format::of_path("notes.txt"), Format::Text);
        assert_eq!(Format::of_path("src/main.rs"), Format::Code);
    }

    #[test]
    fn search_finds_the_nearest_within_the_sources_asked_for() {
        let mut ix = index();
        ix.put(
            Source::Spec,
            "spec.md",
            "# Cache\n\nLe cache de prompt expire après cinq minutes.\n",
        )
        .unwrap();
        ix.put(
            Source::Memory,
            "m/cache.md",
            "Le cache de prompt coûte cher quand un agent dort.\n",
        )
        .unwrap();
        ix.put(
            Source::Decision,
            "d/proxy.md",
            "Le proxy vient des réglages de l'application.\n",
        )
        .unwrap();
        let hits = ix.search("cache de prompt", 5, &[]).unwrap();
        assert_eq!(hits.len(), 3);
        assert_ne!(hits[0].source, Source::Decision);
        assert!(hits[0].score > hits[2].score);
        let only = ix.search("cache de prompt", 5, &[Source::Memory]).unwrap();
        assert_eq!(only.len(), 1);
        assert_eq!(only[0].path, "m/cache.md");
        assert!(ix.search("cache", 5, &[Source::Code]).unwrap().is_empty());
        assert_eq!(ix.search("cache de prompt", 1, &[]).unwrap().len(), 1);
    }

    #[test]
    fn an_unchanged_text_costs_nothing_and_a_changed_one_only_its_new_pieces() {
        let mut ix = index();
        let v1 = "# A\n\nalpha beta gamma\n\n# B\n\ndelta epsilon zeta\n";
        let p = ix.put(Source::Document, "a.md", v1).unwrap();
        assert_eq!((p.unchanged, p.chunks, p.embedded), (false, 2, 2));
        let p = ix.put(Source::Document, "a.md", v1).unwrap();
        assert_eq!((p.unchanged, p.chunks, p.embedded), (true, 2, 0));
        let v2 = "# A\n\nalpha beta gamma\n\n# B\n\ndelta epsilon zeta eta theta\n";
        let p = ix.put(Source::Document, "a.md", v2).unwrap();
        assert_eq!((p.unchanged, p.chunks, p.embedded), (false, 2, 1));
        assert_eq!(ix.stats().unwrap().chunks, 2);
        let hits = ix.search("theta", 1, &[]).unwrap();
        assert!(hits[0].text.contains("theta"));
    }

    #[test]
    fn a_removed_or_vanished_document_is_forgotten_by_both_searches() {
        let mut ix = index();
        ix.put(Source::Document, "a.md", "licorne arc-en-ciel\n")
            .unwrap();
        ix.put(Source::Document, "b.md", "dragon volcan\n").unwrap();
        ix.put(Source::Code, "c.rs", "fn dragon() {}\n").unwrap();
        assert!(ix.remove(Source::Document, "a.md").unwrap());
        assert!(!ix.remove(Source::Document, "a.md").unwrap());
        assert!(ix
            .search_hybrid("licorne", 5, &[])
            .unwrap()
            .iter()
            .all(|h| h.path != "a.md"));
        assert_eq!(ix.retain(Source::Document, &[]).unwrap(), 1);
        assert_eq!(ix.stats().unwrap().documents, 1);
        assert_eq!(ix.documents(Source::Code).unwrap().len(), 1);
        assert_eq!(ix.documents(Source::Document).unwrap().len(), 0);
    }

    #[test]
    fn the_hybrid_search_finds_a_rare_word_and_respects_the_sources() {
        let mut ix = index();
        ix.put(
            Source::Spec,
            "s.md",
            "Le serveur répond sur le port quarante-deux.\n",
        )
        .unwrap();
        ix.put(Source::Code, "x.rs", "const ZXQ_LIMIT: u32 = 3; // zxq\n")
            .unwrap();
        let hits = ix.search_hybrid("zxq", 3, &[]).unwrap();
        assert_eq!(hits[0].path, "x.rs");
        let hits = ix.search_hybrid("zxq", 3, &[Source::Spec]).unwrap();
        assert!(hits.iter().all(|h| h.source == Source::Spec));
        assert!(ix.search_hybrid("?!", 3, &[]).unwrap().len() <= 3);
    }

    #[test]
    fn an_index_made_by_another_model_is_emptied() {
        struct Other;
        impl Embedder for Other {
            fn id(&self) -> &str {
                "other"
            }
            fn dim(&self) -> usize {
                256
            }
            fn embed(&self, t: &[String], k: Kind) -> anyhow::Result<Vec<Vec<f32>>> {
                Fake.embed(t, k)
            }
        }
        let dir = crate::test_dir("vectors-model-change");
        let file = dir.join("v.db");
        let mut ix = Index::open(&file, Arc::new(Fake)).unwrap();
        ix.put(Source::Spec, "s.md", "du texte\n").unwrap();
        drop(ix);
        let ix = Index::open(&file, Arc::new(Fake)).unwrap();
        assert_eq!(ix.stats().unwrap().documents, 1);
        drop(ix);
        let ix = Index::open(&file, Arc::new(Other)).unwrap();
        assert_eq!(ix.stats().unwrap().documents, 0);
    }
}

mod download {
    use super::super::embed::Prefixes;
    use super::super::model::{File, Spec, Store};
    use sha2::{Digest, Sha256};
    use std::sync::atomic::AtomicBool;
    use std::sync::{Arc, Mutex};
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    /// A server of one body at `/f.bin`. The first answer is cut after `cut` bytes when it is given; a `Range` is
    /// honoured unless `ignore_range`. The `Range` headers it received are kept.
    async fn serve(
        body: Vec<u8>,
        cut: Option<usize>,
        ignore_range: bool,
    ) -> (String, Arc<Mutex<Vec<String>>>) {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        let seen = Arc::new(Mutex::new(Vec::new()));
        let log = seen.clone();
        tokio::spawn(async move {
            let mut first = true;
            loop {
                let (mut s, _) = listener.accept().await.unwrap();
                let mut buf = vec![0u8; 4096];
                let n = s.read(&mut buf).await.unwrap();
                let req = String::from_utf8_lossy(&buf[..n]).to_string();
                let range = req.lines().find_map(|l| {
                    l.to_lowercase()
                        .strip_prefix("range: bytes=")
                        .map(|r| r.trim_end_matches('-').to_string())
                });
                log.lock().unwrap().push(range.clone().unwrap_or_default());
                let from = if ignore_range {
                    None
                } else {
                    range.and_then(|r| r.trim().parse::<usize>().ok())
                };
                let part = &body[from.unwrap_or(0)..];
                let head = match from {
                    Some(f) => format!(
                        "HTTP/1.1 206 Partial Content\r\nContent-Length: {}\r\nContent-Range: bytes {f}-{}/{}\r\nConnection: close\r\n\r\n",
                        part.len(),
                        body.len() - 1,
                        body.len()
                    ),
                    None => format!("HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", part.len()),
                };
                s.write_all(head.as_bytes()).await.unwrap();
                let send = if first {
                    cut.map_or(part.len(), |c| c.min(part.len()))
                } else {
                    part.len()
                };
                first = false;
                let _ = s.write_all(&part[..send]).await;
                let _ = s.shutdown().await;
            }
        });
        (url, seen)
    }

    fn spec(body: &[u8], bad_hash: bool) -> Spec {
        let sha = if bad_hash {
            "0".repeat(64)
        } else {
            format!("{:x}", Sha256::digest(body))
        };
        let files: &'static [File] = Box::leak(Box::new([File {
            name: "m.bin",
            url: "f.bin",
            size: body.len() as u64,
            sha256: Box::leak(sha.into_boxed_str()),
        }]));
        Spec {
            id: "t",
            license: "test",
            dim: 4,
            prefixes: Prefixes {
                query: "",
                passage: "",
            },
            files,
        }
    }

    fn body() -> Vec<u8> {
        (0..300_000u32).map(|i| (i % 251) as u8).collect()
    }

    #[tokio::test]
    async fn the_model_comes_with_its_progress_and_is_checked() {
        let body = body();
        let (url, _) = serve(body.clone(), None, false).await;
        let store = Store {
            dir: crate::test_dir("vectors-dl-ok"),
        };
        let spec = spec(&body, false);
        assert!(!store.has(&spec));
        let mut told = Vec::new();
        store
            .download(
                &reqwest::Client::new(),
                &url,
                &spec,
                &AtomicBool::new(false),
                |d, t| told.push((d, t)),
            )
            .await
            .unwrap();
        assert!(store.has(&spec) && store.verify(&spec));
        assert_eq!(std::fs::read(store.path(&spec.files[0])).unwrap(), body);
        assert_eq!(told.last(), Some(&(body.len() as u64, body.len() as u64)));
        assert!(told.windows(2).all(|w| w[0].0 <= w[1].0));
        // Already there: nothing asked of the server.
        store
            .download(
                &reqwest::Client::new(),
                "http://127.0.0.1:1",
                &spec,
                &AtomicBool::new(false),
                |_, _| {},
            )
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn a_cut_download_goes_on_from_where_it_stopped() {
        let body = body();
        let (url, seen) = serve(body.clone(), Some(100_000), false).await;
        let store = Store {
            dir: crate::test_dir("vectors-dl-resume"),
        };
        let spec = spec(&body, false);
        let client = reqwest::Client::new();
        assert!(store
            .download(&client, &url, &spec, &AtomicBool::new(false), |_, _| {})
            .await
            .is_err());
        assert!(!store.has(&spec));
        assert_eq!(store.have(&spec), 100_000);
        store
            .download(&client, &url, &spec, &AtomicBool::new(false), |_, _| {})
            .await
            .unwrap();
        assert!(store.verify(&spec));
        assert_eq!(seen.lock().unwrap().as_slice(), ["", "100000"]);
    }

    #[tokio::test]
    async fn a_server_that_ignores_the_range_makes_it_start_again() {
        let body = body();
        let (url, _) = serve(body.clone(), Some(50_000), true).await;
        let store = Store {
            dir: crate::test_dir("vectors-dl-norange"),
        };
        let spec = spec(&body, false);
        let client = reqwest::Client::new();
        assert!(store
            .download(&client, &url, &spec, &AtomicBool::new(false), |_, _| {})
            .await
            .is_err());
        store
            .download(&client, &url, &spec, &AtomicBool::new(false), |_, _| {})
            .await
            .unwrap();
        assert!(store.verify(&spec));
    }

    #[tokio::test]
    async fn what_is_not_the_model_is_not_kept() {
        let body = body();
        let (url, _) = serve(body.clone(), None, false).await;
        let store = Store {
            dir: crate::test_dir("vectors-dl-bad"),
        };
        let spec = spec(&body, true);
        let err = store
            .download(
                &reqwest::Client::new(),
                &url,
                &spec,
                &AtomicBool::new(false),
                |_, _| {},
            )
            .await
            .unwrap_err();
        assert!(err
            .downcast_ref::<escouade_voice_local::models::ChecksumMismatch>()
            .is_some());
        assert!(!store.has(&spec));
        assert_eq!(store.have(&spec), 0);
    }

    #[tokio::test]
    async fn a_download_called_off_keeps_its_start_for_later() {
        let body = body();
        let (url, _) = serve(body.clone(), None, false).await;
        let store = Store {
            dir: crate::test_dir("vectors-dl-cancel"),
        };
        let spec = spec(&body, false);
        let err = store
            .download(
                &reqwest::Client::new(),
                &url,
                &spec,
                &AtomicBool::new(true),
                |_, _| {},
            )
            .await
            .unwrap_err();
        assert!(err
            .downcast_ref::<escouade_voice_local::models::Cancelled>()
            .is_some());
        assert!(!store.has(&spec));
        store.delete(&spec).unwrap();
        assert_eq!(store.have(&spec), 0);
    }
}

/// The real model, downloaded into `ESCOUADE_TEST_REAL_VECTORS` (kept there: the next run finds it and checks it), the
/// way the app does it, then the set of searches, the throughput and the incremental index on this repo's own docs.
/// Run by hand: 300 MB, and the CI has no model.
#[tokio::test]
#[ignore = "downloads the real model: ESCOUADE_TEST_REAL_VECTORS=<folder>"]
async fn the_real_model_downloads_and_searches_in_french_and_english() {
    use super::index::{Index, Source};
    use super::model::{Store, MODEL, MODEL_URL};
    use std::sync::{atomic::AtomicBool, Arc};
    let Ok(dir) = std::env::var("ESCOUADE_TEST_REAL_VECTORS") else {
        return;
    };
    let store = Store {
        dir: PathBuf::from(dir),
    };
    let started = Instant::now();
    let mut last = 0;
    store
        .download(
            &reqwest::Client::new(),
            MODEL_URL,
            &MODEL,
            &AtomicBool::new(false),
            |d, t| {
                if d * 10 / t != last {
                    last = d * 10 / t;
                    eprintln!("download {} %", d * 100 / t);
                }
            },
        )
        .await
        .unwrap();
    eprintln!(
        "model there in {:?} (verified: {})",
        started.elapsed(),
        store.verify(&MODEL)
    );
    assert!(store.verify(&MODEL));
    let started = Instant::now();
    let model = store.load(&MODEL).unwrap();
    eprintln!("loaded in {:?}, dim {}", started.elapsed(), model.dim());
    assert_eq!(model.dim(), MODEL.dim);
    let (r1, r3, mrr, per_s) = measure(&model);
    eprintln!("quality: recall@1 {r1:.2} recall@3 {r3:.2} mrr {mrr:.3}; {per_s:.1} passages/s");
    assert!(r3 >= 0.8 && mrr >= 0.7, "quality fell: {r3} {mrr}");

    // The repo's docs through the index: throughput, then incremental, then search with a filter.
    let model: Arc<dyn Embedder> = Arc::new(model);
    let mut ix = Index::in_memory(model).unwrap();
    let docs = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../docs");
    let started = Instant::now();
    let mut chunks = 0;
    for (name, src) in [("SPEC.md", Source::Spec), ("FACTORY.md", Source::Document)] {
        let text = std::fs::read_to_string(docs.join(name)).unwrap();
        chunks += ix.put(src, name, &text).unwrap().chunks;
    }
    let took = started.elapsed();
    eprintln!(
        "indexed {chunks} pieces in {took:?}: {:.1} pieces/s",
        chunks as f64 / took.as_secs_f64()
    );
    let text = std::fs::read_to_string(docs.join("FACTORY.md")).unwrap();
    let started = Instant::now();
    let p = ix
        .put(
            Source::Document,
            "FACTORY.md",
            &format!(
                "{text}

## Ajout

Une nouvelle phrase sur les chats.
"
            ),
        )
        .unwrap();
    eprintln!(
        "one section added: {} pieces read of {} in {:?}",
        p.embedded,
        p.chunks,
        started.elapsed()
    );
    assert!(p.embedded <= 2);
    let started = Instant::now();
    let hits = ix
        .search(
            "où sont rangés les vecteurs qui se recalculent ?",
            3,
            &[Source::Document],
        )
        .unwrap();
    eprintln!(
        "search in {:?}: {} › {}",
        started.elapsed(),
        hits[0].path,
        hits[0].title
    );
    assert!(hits.iter().all(|h| h.source == Source::Document));
}

/// The search over many pieces, with the fake model: how long the nearest of 768-number vectors takes to find.
#[test]
#[ignore = "a measure"]
fn search_time_over_many_pieces() {
    use super::index::{Index, Source};
    use std::sync::Arc;
    struct Wide;
    impl Embedder for Wide {
        fn id(&self) -> &str {
            "wide"
        }
        fn dim(&self) -> usize {
            768
        }
        fn embed(&self, t: &[String], _: Kind) -> anyhow::Result<Vec<Vec<f32>>> {
            Ok(t.iter()
                .map(|s| {
                    let mut seed = s
                        .bytes()
                        .fold(7u64, |h, b| h.wrapping_mul(31).wrapping_add(b as u64));
                    let mut v: Vec<f32> = (0..768)
                        .map(|_| {
                            seed = seed
                                .wrapping_mul(6364136223846793005)
                                .wrapping_add(1442695040888963407);
                            (seed >> 40) as f32 / 1e6 - 8.0
                        })
                        .collect();
                    super::embed::normalize(&mut v);
                    v
                })
                .collect())
        }
    }
    let mut ix = Index::in_memory(Arc::new(Wide)).unwrap();
    for n in [1_000, 10_000, 50_000] {
        let have = ix.stats().unwrap().documents;
        for i in have..n {
            ix.put(
                Source::Code,
                &format!("f{i}.rs"),
                &format!("fn f{i}() {{ {i} }}"),
            )
            .unwrap();
        }
        let started = Instant::now();
        for _ in 0..5 {
            ix.search("fn quelque chose", 10, &[]).unwrap();
        }
        eprintln!("{n} pieces: search {:?}", started.elapsed() / 5);
    }
}
