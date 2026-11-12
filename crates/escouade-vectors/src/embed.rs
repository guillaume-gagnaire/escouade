//! The embeddings: a text becomes a unit vector whose direction is its meaning. `Onnx` runs a multilingual model with
//! ONNX Runtime on this machine; `Fake` is the deterministic model of the tests.

use anyhow::{anyhow, bail, Context, Result};
use escouade_voice_local::ort;
use parking_lot::Mutex;
use std::path::Path;

/// What a text is for: some models are trained with a different prefix on what is searched and on what is searched in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Query,
    Passage,
}

/// Something that turns texts into unit vectors of `dim()` numbers. The vectors of two texts of the same meaning, in
/// any of the languages the model knows, are close (a dot product near 1).
pub trait Embedder: Send + Sync {
    /// The model's name, kept with the vectors it made: another model's vectors are never mixed with these.
    fn id(&self) -> &str;
    fn dim(&self) -> usize;
    fn embed(&self, texts: &[String], kind: Kind) -> Result<Vec<Vec<f32>>>;
}

/// Brought to a length of 1 (a dot product is then a cosine).
pub fn normalize(v: &mut [f32]) {
    let n = v.iter().map(|x| x * x).sum::<f32>().sqrt();
    if n > 1e-12 {
        v.iter_mut().for_each(|x| *x /= n);
    }
}

pub fn dot(a: &[f32], b: &[f32]) -> f32 {
    a.iter().zip(b).map(|(x, y)| x * y).sum()
}

/// What differs from one model to the next.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Prefixes {
    pub query: &'static str,
    pub passage: &'static str,
}

/// The longest text the model reads, in tokens (the rest is cut): XLM-R's models hold 512.
const MAX_TOKENS: usize = 512;
/// How many texts go through the model together: a batch pads every text to the longest, so a big one costs more than
/// it brings.
const BATCH: usize = 16;

pub struct Onnx {
    id: String,
    dim: usize,
    prefixes: Prefixes,
    tokenizer: tokenizers::Tokenizer,
    session: Mutex<ort::session::Session>,
    token_types: bool,
}

impl Onnx {
    /// The model `model` (ONNX) and its tokenizer (`tokenizer.json`), loaded. ONNX Runtime is made ready first.
    pub fn load(id: &str, model: &Path, tokenizer: &Path, prefixes: Prefixes) -> Result<Self> {
        escouade_voice_local::speech::piper::runtime()?;
        let mut tok = tokenizers::Tokenizer::from_file(tokenizer)
            .map_err(|e| anyhow!("{}: {e}", tokenizer.display()))?;
        tok.with_truncation(Some(tokenizers::TruncationParams {
            max_length: MAX_TOKENS,
            ..Default::default()
        }))
        .map_err(|e| anyhow!("{e}"))?;
        tok.with_padding(Some(tokenizers::PaddingParams {
            strategy: tokenizers::PaddingStrategy::BatchLongest,
            ..Default::default()
        }));
        let session = ort::session::Session::builder()
            .map_err(|e| anyhow!("{e}"))?
            .commit_from_file(model)
            .map_err(|e| anyhow!("{}: {e}", model.display()))?;
        let token_types = session
            .inputs()
            .iter()
            .any(|i| i.name() == "token_type_ids");
        let mut me = Self {
            id: id.into(),
            dim: 0,
            prefixes,
            tokenizer: tok,
            session: Mutex::new(session),
            token_types,
        };
        me.dim = me
            .embed(&["test".into()], Kind::Passage)?
            .first()
            .map_or(0, Vec::len);
        if me.dim == 0 {
            bail!("the model gave no vector");
        }
        Ok(me)
    }

    fn run(&self, texts: &[String]) -> Result<Vec<Vec<f32>>> {
        use ort::value::Tensor;
        let enc = self
            .tokenizer
            .encode_batch(texts.to_vec(), true)
            .map_err(|e| anyhow!("{e}"))?;
        let b = enc.len();
        let n = enc.first().map_or(0, |e| e.get_ids().len());
        let ids: Vec<i64> = enc
            .iter()
            .flat_map(|e| e.get_ids().iter().map(|&x| x as i64))
            .collect();
        let mask: Vec<i64> = enc
            .iter()
            .flat_map(|e| e.get_attention_mask().iter().map(|&x| x as i64))
            .collect();
        let shape = [b, n];
        let mut inputs = ort::inputs![
            "input_ids" => Tensor::<i64>::from_array((shape, ids.into_boxed_slice()))?,
            "attention_mask" => Tensor::<i64>::from_array((shape, mask.clone().into_boxed_slice()))?,
        ];
        if self.token_types {
            inputs.push((
                "token_type_ids".into(),
                Tensor::<i64>::from_array((shape, vec![0i64; b * n].into_boxed_slice()))?.into(),
            ));
        }
        let mut session = self.session.lock();
        let outputs = session.run(inputs).map_err(|e| anyhow!("{e}"))?;
        let (dims, data) = outputs[0]
            .try_extract_tensor::<f32>()
            .map_err(|e| anyhow!("{e}"))?;
        if dims.len() != 3 || dims[0] as usize != b || dims[1] as usize != n {
            bail!("unexpected output shape {dims:?}");
        }
        let d = dims[2] as usize;
        // Mean of the tokens that are not padding.
        Ok((0..b)
            .map(|i| {
                let mut v = vec![0f32; d];
                let mut count = 0f32;
                for t in 0..n {
                    if mask[i * n + t] == 0 {
                        continue;
                    }
                    count += 1.0;
                    let row = &data[(i * n + t) * d..(i * n + t + 1) * d];
                    v.iter_mut().zip(row).for_each(|(a, x)| *a += x);
                }
                v.iter_mut().for_each(|x| *x /= count.max(1.0));
                normalize(&mut v);
                v
            })
            .collect())
    }
}

impl Embedder for Onnx {
    fn id(&self) -> &str {
        &self.id
    }

    fn dim(&self) -> usize {
        self.dim
    }

    fn embed(&self, texts: &[String], kind: Kind) -> Result<Vec<Vec<f32>>> {
        let prefix = match kind {
            Kind::Query => self.prefixes.query,
            Kind::Passage => self.prefixes.passage,
        };
        // The longest first inside a batch of a similar length: less padding.
        let mut order: Vec<usize> = (0..texts.len()).collect();
        order.sort_by_key(|&i| texts[i].len());
        let mut out = vec![Vec::new(); texts.len()];
        for group in order.chunks(BATCH) {
            let batch: Vec<String> = group
                .iter()
                .map(|&i| format!("{prefix}{}", texts[i]))
                .collect();
            let vectors = self.run(&batch).context("embeddings")?;
            for (&i, v) in group.iter().zip(vectors) {
                out[i] = v;
            }
        }
        Ok(out)
    }
}

/// The model of the tests: the words of a text hashed into 256 dimensions, so that two texts that share words are
/// close and two that share none are not. Deterministic, instant, nothing to download. It knows no synonym.
pub struct Fake;

impl Fake {
    const DIM: usize = 256;

    fn word(w: &str) -> usize {
        // FNV-1a: the same on every machine and in every run.
        let mut h = 0xcbf29ce484222325u64;
        for b in w.bytes() {
            h = (h ^ b as u64).wrapping_mul(0x100000001b3);
        }
        (h % Self::DIM as u64) as usize
    }
}

impl Embedder for Fake {
    fn id(&self) -> &str {
        "fake"
    }

    fn dim(&self) -> usize {
        Self::DIM
    }

    fn embed(&self, texts: &[String], _: Kind) -> Result<Vec<Vec<f32>>> {
        Ok(texts
            .iter()
            .map(|t| {
                let mut v = vec![0f32; Self::DIM];
                for w in t
                    .to_lowercase()
                    .split(|c: char| !c.is_alphanumeric())
                    .filter(|w| w.chars().count() > 2)
                {
                    v[Self::word(w)] += 1.0;
                }
                normalize(&mut v);
                v
            })
            .collect())
    }
}
