use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

// ── On-device ML models — pure Rust math, zero deps ──

/// Feature vector for a story
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StoryFeatures {
    pub files_changed: f64,
    pub lines_added: f64,
    pub lines_removed: f64,
    pub tests_run: f64,
    pub complexity_score: f64, // 0=simple, 1=moderate, 2=complex, 3=critical
}

impl StoryFeatures {
    pub fn to_vec(&self) -> Vec<f64> {
        vec![1.0, self.files_changed, self.lines_added, self.lines_removed, self.tests_run, self.complexity_score]
    }
}

// ── Linear Regression (OLS) for time estimation ──

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LinearRegressionModel {
    pub weights: Vec<f64>, // [intercept, w1, w2, w3, w4, w5]
}

impl LinearRegressionModel {
    pub fn predict(&self, features: &StoryFeatures) -> f64 {
        let x = features.to_vec();
        x.iter().zip(self.weights.iter()).map(|(xi, wi)| xi * wi).sum()
    }

    /// Train via OLS: w = (X^T X)^-1 X^T y
    pub fn train(samples: &[(StoryFeatures, f64)]) -> Option<Self> {
        if samples.len() < 6 { return None; } // Need more samples than features

        let n = samples.len();
        let p = 6; // intercept + 5 features
        let x: Vec<Vec<f64>> = samples.iter().map(|(f, _)| f.to_vec()).collect();
        let y: Vec<f64> = samples.iter().map(|(_, target)| *target).collect();

        // X^T X (p x p matrix)
        let mut xtx = vec![vec![0.0; p]; p];
        for i in 0..p {
            for j in 0..p {
                for k in 0..n {
                    xtx[i][j] += x[k][i] * x[k][j];
                }
            }
        }

        // X^T y (p vector)
        let mut xty = vec![0.0; p];
        for i in 0..p {
            for k in 0..n {
                xty[i] += x[k][i] * y[k];
            }
        }

        // Ridge regularization: add small lambda to diagonal for numerical stability
        for i in 0..p {
            xtx[i][i] += 0.001;
        }

        // Invert (X^T X + λI) via Gauss-Jordan
        let inv = invert_matrix(&xtx)?;

        // w = inv * X^T y
        let mut weights = vec![0.0; p];
        for i in 0..p {
            for j in 0..p {
                weights[i] += inv[i][j] * xty[j];
            }
        }

        Some(Self { weights })
    }
}

// ── Naive Bayes Classifier for story categorization ──

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NaiveBayesModel {
    pub categories: Vec<String>,
    pub category_counts: Vec<usize>,
    pub word_counts: HashMap<String, Vec<usize>>, // word → counts per category
    pub total_words_per_category: Vec<usize>,
    pub vocabulary_size: usize,
}

impl NaiveBayesModel {
    pub fn predict(&self, tokens: &[String]) -> String {
        let total: usize = self.category_counts.iter().sum();
        let mut best_score = f64::NEG_INFINITY;
        let mut best_cat = 0;

        for (i, _cat) in self.categories.iter().enumerate() {
            let mut log_prob = (self.category_counts[i] as f64 / total as f64).ln();
            let denom = (self.total_words_per_category[i] + self.vocabulary_size) as f64;

            for token in tokens {
                let count = self.word_counts.get(token)
                    .map(|c| c[i])
                    .unwrap_or(0);
                log_prob += ((count as f64 + 1.0) / denom).ln(); // Laplace smoothing
            }

            if log_prob > best_score {
                best_score = log_prob;
                best_cat = i;
            }
        }

        self.categories[best_cat].clone()
    }

    pub fn train(samples: &[(Vec<String>, String)], categories: &[&str]) -> Option<Self> {
        if samples.len() < 5 { return None; }

        let cats: Vec<String> = categories.iter().map(|s| s.to_string()).collect();
        let n = cats.len();
        let mut cat_counts = vec![0usize; n];
        let mut word_counts: HashMap<String, Vec<usize>> = HashMap::new();
        let mut total_words = vec![0usize; n];

        for (tokens, label) in samples {
            let idx = cats.iter().position(|c| c == label).unwrap_or(0);
            cat_counts[idx] += 1;
            for token in tokens {
                let entry = word_counts.entry(token.clone()).or_insert_with(|| vec![0; n]);
                entry[idx] += 1;
                total_words[idx] += 1;
            }
        }

        Some(Self {
            categories: cats,
            category_counts: cat_counts,
            vocabulary_size: word_counts.len(),
            word_counts,
            total_words_per_category: total_words,
        })
    }
}

// ── Decision Stump for conflict prediction ──

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecisionStumpModel {
    pub feature_index: usize,
    pub threshold: f64,
    pub probability_above: f64,
    pub probability_below: f64,
}

impl DecisionStumpModel {
    pub fn predict(&self, features: &[f64]) -> f64 {
        if features.get(self.feature_index).copied().unwrap_or(0.0) >= self.threshold {
            self.probability_above
        } else {
            self.probability_below
        }
    }

    pub fn train(samples: &[(Vec<f64>, bool)]) -> Option<Self> {
        if samples.len() < 5 { return None; }

        let n_features = samples[0].0.len();
        let mut best_gain = 0.0;
        let mut best = Self { feature_index: 0, threshold: 0.0, probability_above: 0.5, probability_below: 0.5 };

        for fi in 0..n_features {
            let mut values: Vec<f64> = samples.iter().map(|(f, _)| f[fi]).collect();
            values.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
            values.dedup();

            for threshold in &values {
                let (mut above_pos, mut above_neg, mut below_pos, mut below_neg) = (0, 0, 0, 0);
                for (features, label) in samples {
                    if features[fi] >= *threshold {
                        if *label { above_pos += 1; } else { above_neg += 1; }
                    } else {
                        if *label { below_pos += 1; } else { below_neg += 1; }
                    }
                }

                let gain = info_gain(above_pos, above_neg, below_pos, below_neg, samples.len());
                if gain > best_gain {
                    best_gain = gain;
                    let a_total = (above_pos + above_neg) as f64;
                    let b_total = (below_pos + below_neg) as f64;
                    best = Self {
                        feature_index: fi,
                        threshold: *threshold,
                        probability_above: if a_total > 0.0 { above_pos as f64 / a_total } else { 0.5 },
                        probability_below: if b_total > 0.0 { below_pos as f64 / b_total } else { 0.5 },
                    };
                }
            }
        }

        Some(best)
    }
}

// ── ML Trainer — trains all models from LearningRecords ──

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TrainedModels {
    pub time_model: Option<LinearRegressionModel>,
    pub category_model: Option<NaiveBayesModel>,
    pub conflict_model: Option<DecisionStumpModel>,
    pub trained_at: Option<String>,
    pub sample_count: usize,
}

const CATEGORIES: &[&str] = &[
    "backend_rust", "frontend_react", "ios_swift", "testing",
    "db_migration", "devops", "docs", "general",
];

impl TrainedModels {
    /// Train all models from learning records
    pub fn train_from_records(records: &[(StoryFeatures, f64, Vec<String>, String, bool)]) -> Self {
        let sample_count = records.len();
        if sample_count < 10 {
            return Self { sample_count, ..Default::default() };
        }

        // Time regression
        let time_samples: Vec<(StoryFeatures, f64)> = records.iter()
            .map(|(f, dur, _, _, _)| (f.clone(), *dur))
            .collect();
        let time_model = LinearRegressionModel::train(&time_samples);

        // Category classification
        let cat_samples: Vec<(Vec<String>, String)> = records.iter()
            .map(|(_, _, tokens, cat, _)| (tokens.clone(), cat.clone()))
            .collect();
        let category_model = NaiveBayesModel::train(&cat_samples, CATEGORIES);

        // Conflict prediction (pairs from same session)
        // Simplified: use individual records as "this story had conflicts" signal
        let conflict_samples: Vec<(Vec<f64>, bool)> = records.iter()
            .map(|(f, _, _, _, had_conflict)| {
                (vec![f.files_changed, f.lines_added, f.tests_run, f.complexity_score], *had_conflict)
            })
            .collect();
        let conflict_model = DecisionStumpModel::train(&conflict_samples);

        Self {
            time_model,
            category_model,
            conflict_model,
            trained_at: Some(chrono::Utc::now().to_rfc3339()),
            sample_count,
        }
    }

    /// Save to .crossroads/ml/models.json
    pub fn save(&self, project_path: &str) -> Result<(), String> {
        let dir = Path::new(project_path).join(".crossroads/ml");
        std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        let json = serde_json::to_string_pretty(self).map_err(|e| e.to_string())?;
        std::fs::write(dir.join("models.json"), json).map_err(|e| e.to_string())
    }

    /// Load from .crossroads/ml/models.json
    pub fn load(project_path: &str) -> Option<Self> {
        let path = Path::new(project_path).join(".crossroads/ml/models.json");
        let content = std::fs::read_to_string(path).ok()?;
        serde_json::from_str(&content).ok()
    }

    /// Predict story time (returns None if model not trained)
    pub fn predict_time(&self, features: &StoryFeatures) -> Option<f64> {
        self.time_model.as_ref().map(|m| m.predict(features))
    }

    /// Classify story category (returns None if model not trained)
    pub fn classify_story(&self, tokens: &[String]) -> Option<String> {
        self.category_model.as_ref().map(|m| m.predict(tokens))
    }

    /// Predict conflict probability (returns None if model not trained)
    pub fn predict_conflict(&self, features: &[f64]) -> Option<f64> {
        self.conflict_model.as_ref().map(|m| m.predict(features))
    }
}

// ── Math helpers ──

fn invert_matrix(m: &[Vec<f64>]) -> Option<Vec<Vec<f64>>> {
    let n = m.len();
    let mut aug: Vec<Vec<f64>> = m.iter().enumerate().map(|(i, row)| {
        let mut r = row.clone();
        r.extend((0..n).map(|j| if i == j { 1.0 } else { 0.0 }));
        r
    }).collect();

    for i in 0..n {
        let pivot = aug[i][i];
        if pivot.abs() < 1e-12 { return None; }
        for j in 0..2*n { aug[i][j] /= pivot; }
        for k in 0..n {
            if k == i { continue; }
            let factor = aug[k][i];
            for j in 0..2*n { aug[k][j] -= factor * aug[i][j]; }
        }
    }

    Some(aug.iter().map(|row| row[n..].to_vec()).collect())
}

fn entropy(pos: usize, neg: usize) -> f64 {
    let total = (pos + neg) as f64;
    if total == 0.0 { return 0.0; }
    let p = pos as f64 / total;
    let n = neg as f64 / total;
    let mut e = 0.0;
    if p > 0.0 { e -= p * p.log2(); }
    if n > 0.0 { e -= n * n.log2(); }
    e
}

fn info_gain(above_pos: usize, above_neg: usize, below_pos: usize, below_neg: usize, total: usize) -> f64 {
    let parent = entropy(above_pos + below_pos, above_neg + below_neg);
    let a_total = (above_pos + above_neg) as f64;
    let b_total = (below_pos + below_neg) as f64;
    let t = total as f64;
    parent - (a_total / t) * entropy(above_pos, above_neg) - (b_total / t) * entropy(below_pos, below_neg)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_linear_regression_simple() {
        // Independent features with noise to ensure non-singular matrix
        let samples: Vec<(StoryFeatures, f64)> = (1..=20).map(|i| {
            let f = i as f64;
            let noise = ((i * 7 + 3) % 5) as f64; // deterministic pseudo-noise
            (StoryFeatures {
                files_changed: f + noise,
                lines_added: f * 3.0 + noise * 2.0,
                lines_removed: noise * 5.0,
                tests_run: f * 1.5 + noise,
                complexity_score: (i % 4) as f64,
            }, 1000.0 + f * 100.0) // target grows with i
        }).collect();

        let model = LinearRegressionModel::train(&samples);
        assert!(model.is_some(), "Model should train successfully");
        let m = model.unwrap();
        // Prediction should be in reasonable range for i=10
        let pred = m.predict(&StoryFeatures { files_changed: 12.0, lines_added: 36.0, lines_removed: 10.0, tests_run: 18.0, complexity_score: 2.0 });
        assert!(pred > 500.0 && pred < 3000.0, "Prediction {} should be in reasonable range", pred);
    }

    #[test]
    fn test_naive_bayes() {
        let samples = vec![
            (vec!["rust".into(), "backend".into(), "api".into()], "backend_rust".into()),
            (vec!["rust".into(), "server".into()], "backend_rust".into()),
            (vec!["react".into(), "component".into(), "ui".into()], "frontend_react".into()),
            (vec!["react".into(), "form".into()], "frontend_react".into()),
            (vec!["test".into(), "unit".into()], "testing".into()),
            (vec!["test".into(), "integration".into()], "testing".into()),
        ];

        let model = NaiveBayesModel::train(&samples, CATEGORIES).unwrap();
        let pred = model.predict(&["rust".into(), "api".into()]);
        assert_eq!(pred, "backend_rust");
    }

    #[test]
    fn test_decision_stump() {
        let samples: Vec<(Vec<f64>, bool)> = vec![
            (vec![10.0, 5.0], true),
            (vec![8.0, 4.0], true),
            (vec![1.0, 0.0], false),
            (vec![2.0, 1.0], false),
            (vec![0.0, 0.0], false),
            (vec![9.0, 3.0], true),
        ];

        let model = DecisionStumpModel::train(&samples).unwrap();
        assert!(model.predict(&[10.0, 5.0]) > 0.5);
        assert!(model.predict(&[0.0, 0.0]) < 0.5);
    }

    #[test]
    fn test_trained_models_save_load() {
        let dir = format!("/tmp/xroads-ml-test-{}", uuid::Uuid::new_v4());
        std::fs::create_dir_all(&dir).unwrap();

        let models = TrainedModels {
            time_model: Some(LinearRegressionModel { weights: vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0] }),
            category_model: None,
            conflict_model: None,
            trained_at: Some("2026-03-29T12:00:00Z".into()),
            sample_count: 50,
        };

        models.save(&dir).unwrap();
        let loaded = TrainedModels::load(&dir).unwrap();
        assert_eq!(loaded.sample_count, 50);
        assert!(loaded.time_model.is_some());

        std::fs::remove_dir_all(&dir).ok();
    }
}
