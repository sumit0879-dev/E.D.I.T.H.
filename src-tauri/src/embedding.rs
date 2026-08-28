// Pure Rust semantic-like embedding using TF-IDF style approach
// No C++ runtime dependencies - avoids MSVC CRT mismatch

use std::collections::HashMap;

/// Generate a deterministic embedding vector from text using character n-gram hashing.
/// This is a fast, pure-Rust alternative to ONNX-based embeddings.
/// For production semantic search, replace with an HTTP call to an embedding API.
pub fn embed_text_hash(text: &str) -> Vec<f32> {
    const DIM: usize = 384;
    let mut vec = vec![0.0f32; DIM];
    
    let text_lower = text.to_lowercase();
    let words: Vec<&str> = text_lower.split_whitespace().collect();
    
    if words.is_empty() {
        return vec;
    }
    
    // Word unigrams
    let mut word_counts: HashMap<&str, usize> = HashMap::new();
    for w in &words {
        *word_counts.entry(w).or_insert(0) += 1;
    }
    
    for (word, count) in &word_counts {
        let tf = (*count as f32) / (words.len() as f32);
        // Hash word into multiple positions using different hash seeds
        for seed in 0u64..6 {
            let h = fnv_hash(word.as_bytes(), seed);
            let pos = (h as usize) % DIM;
            let val = (fnv_hash(word.as_bytes(), seed + 100) as f32 / u64::MAX as f32) * 2.0 - 1.0;
            vec[pos] += val * tf;
        }
    }
    
    // Character trigrams for subword information
    for word in &words {
        let chars: Vec<char> = word.chars().collect();
        for i in 0..chars.len().saturating_sub(2) {
            let trigram: String = chars[i..i+3].iter().collect();
            let h = fnv_hash(trigram.as_bytes(), 42);
            let pos = (h as usize) % DIM;
            let val = (fnv_hash(trigram.as_bytes(), 99) as f32 / u64::MAX as f32) * 2.0 - 1.0;
            vec[pos] += val * 0.3;
        }
    }
    
    // Positional encoding
    for (i, word) in words.iter().enumerate() {
        let h = fnv_hash(word.as_bytes(), i as u64 + 200);
        let pos = (h as usize) % DIM;
        let position_weight = 1.0 / (1.0 + i as f32).ln().max(1.0);
        vec[pos] += position_weight * 0.1;
    }
    
    // L2 normalize
    let norm: f32 = vec.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm > 1e-9 {
        for x in &mut vec {
            *x /= norm;
        }
    }
    
    vec
}

fn fnv_hash(data: &[u8], seed: u64) -> u64 {
    let mut hash: u64 = 14695981039346656037u64.wrapping_add(seed.wrapping_mul(2654435761));
    for &byte in data {
        hash ^= byte as u64;
        hash = hash.wrapping_mul(1099511628211);
    }
    hash
}

/// Cosine similarity between two vectors
pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() {
        return 0.0;
    }
    let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm_a < 1e-9 || norm_b < 1e-9 {
        return 0.0;
    }
    dot / (norm_a * norm_b)
}
