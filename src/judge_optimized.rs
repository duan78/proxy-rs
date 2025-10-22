use hyper::{Request, StatusCode};
use hyper_util::client::legacy::Client;
use hyper_util::client::legacy::connect::HttpConnector;
use hyper_tls::HttpsConnector;
use http_body_util::{BodyExt, Empty};
use rand::{Rng, thread_rng};
use std::{collections::BTreeMap, sync::Arc, time::Duration};
use tokio::{sync::RwLock, time::timeout};
use url::Url;

use crate::{resolver::Resolver, utils::http::random_useragent};

// Judges ultra-rapides et fiables
const HIGH_PERFORMANCE_JUDGES: &[&str] = &[
    // Judges HTTP ultra-rapides (< 500ms)
    "http://httpbin.org/ip",                    // Simple et rapide
    "https://httpbin.org/ip",                   // HTTPS rapide
    "http://ifconfig.me/ip",                    // Service IP dédié
    "http://icanhazip.com",                     // Service IP minimaliste
    "http://ident.me",                          // Service IP ultra-light
    "http://myexternalip.com/raw",              // IP brute, pas de HTML
    "http://ipecho.net/plain",                  // Texte pur

    // Judges avec environnement (pour anonymat)
    "http://httpbin.org/get?show_env",          // Variables d'environnement
    "https://httpbin.org/get?show_env",         // HTTPS avec environnement
    "http://httpheader.net/azenv.php",          // Headers complets
    "https://httpheader.net/azenv.php",         // HTTPS headers

    // Judges spécialisés (backup)
    "http://azenv.net/",                        // Judge spécialisé
    "http://proxyjudge.us/azenv.php",          // Judge proxy dédié
    "https://www.proxyjudge.info/azenv.php",    // HTTPS judge
];

// Judges SMTP pour CONNECT:25
const SMTP_JUDGES: &[&str] = &[
    "smtp://smtp.gmail.com:587",
    "smtp://smtp.gmail.com:465",
    "smtp://aspmx.l.google.com:25",
    "smtp://mail.protonmail.ch:587",
];

// Cache pour les résultats de judges
pub type JudgeCache = Arc<RwLock<std::collections::HashMap<String, JudgeInfo>>>;

#[derive(Debug, Clone)]
pub struct JudgeInfo {
    pub url: String,
    pub host: String,
    pub scheme: String,
    pub ip_address: Option<String>,
    pub is_working: bool,
    pub response_time: Duration,
    pub success_rate: f64,
    pub last_checked: std::time::Instant,
    pub marks: BTreeMap<String, usize>,
}

impl JudgeInfo {
    pub fn new(url: &str) -> Self {
        let url = Url::parse(url).unwrap_or_else(|e| {
            log::error!("Failed to parse judge URL '{}': {}", url, e);
            panic!("Invalid judge URL format: {}", url);
        });

        let mut marks = BTreeMap::new();
        marks.insert("via".to_string(), 0);
        marks.insert("proxy".to_string(), 0);

        JudgeInfo {
            url: url.to_string(),
            scheme: url.scheme().to_uppercase(),
            host: url.host_str().unwrap_or("unknown").to_string(),
            ip_address: None,
            is_working: false,
            response_time: Duration::from_millis(1000), // Default 1s
            success_rate: 0.0,
            last_checked: std::time::Instant::now(),
            marks,
        }
    }

    pub fn health_score(&self) -> f64 {
        if !self.is_working {
            return 0.0;
        }

        // Score basé sur le temps de réponse et le taux de succès
        let time_score = (1000.0 / self.response_time.as_millis() as f64).min(10.0);
        let reliability_score = self.success_rate;

        time_score * reliability_score
    }
}

// Manager pour les judges optimisés
pub struct OptimizedJudgeManager {
    cache: JudgeCache,
    http_judges: Vec<JudgeInfo>,
    smtp_judges: Vec<JudgeInfo>,
    client_pool: Vec<Client<HttpsConnector<HttpConnector>, Empty<bytes::Bytes>>>,
}

impl OptimizedJudgeManager {
    pub fn new() -> Self {
        let http_judges: Vec<JudgeInfo> = HIGH_PERFORMANCE_JUDGES
            .iter()
            .map(|url| JudgeInfo::new(url))
            .collect();

        let smtp_judges: Vec<JudgeInfo> = SMTP_JUDGES
            .iter()
            .map(|url| JudgeInfo::new(url))
            .collect();

        // Pool de clients HTTP réutilisables
        let client_pool = (0..5).map(|_| {
            let connector = hyper_tls::native_tls::TlsConnector::builder()
                .danger_accept_invalid_certs(true)
                .danger_accept_invalid_hostnames(true)
                .build()
                .map(|tls| {
                    let mut http = HttpConnector::new();
                    http.enforce_http(false);
                    http.set_keepalive(Some(Duration::from_secs(30)));
                    HttpsConnector::from((http, tls.into()))
                })
                .unwrap();

            Client::builder(hyper_util::rt::TokioExecutor::new())
                .pool_idle_timeout(Duration::from_secs(30))
                .pool_max_idle_per_host(5)
                .build::<_, Empty<bytes::Bytes>>(connector)
        }).collect();

        Self {
            cache: Arc::new(RwLock::new(std::collections::HashMap::new())),
            http_judges,
            smtp_judges,
            client_pool,
        }
    }

    // Pré-test rapide de tous les judges
    pub async fn pretest_judges(&mut self, real_ext_ip: &str) {
        log::info!("🚀 Pré-test des judges haute performance...");

        let mut tasks = Vec::new();

        // Test parallèle des judges HTTP
        for judge in &mut self.http_judges {
            let client_idx = thread_rng().gen_range(0..self.client_pool.len());
            let client = self.client_pool[client_idx].clone();
            let ip = real_ext_ip.to_string();
            let mut judge_clone = judge.clone();

            tasks.push(tokio::spawn(async move {
                Self::test_judge_fast(&mut judge_clone, &client, &ip).await;
                judge_clone
            }));
        }

        // Test rapide des judges SMTP
        for judge in &mut self.smtp_judges {
            let mut judge_clone = judge.clone();

            tasks.push(tokio::spawn(async move {
                // Les judges SMTP sont considérés comme working par défaut
                judge_clone.is_working = true;
                judge_clone.response_time = Duration::from_millis(100);
                judge_clone.success_rate = 1.0;
                judge_clone
            }));
        }

        // Collecter les résultats
        let results = futures_util::future::join_all(tasks).await;

        // Mettre à jour les judges avec les résultats
        for (i, result) in results.into_iter().enumerate() {
            if let Ok(checked_judge) = result {
                if i < self.http_judges.len() {
                    self.http_judges[i] = checked_judge;
                } else {
                    let smtp_idx = i - self.http_judges.len();
                    if smtp_idx < self.smtp_judges.len() {
                        self.smtp_judges[smtp_idx] = checked_judge;
                    }
                }
            }
        }

        // Trier par performance
        self.http_judges.sort_by(|a, b| b.health_score().partial_cmp(&a.health_score()).unwrap_or(std::cmp::Ordering::Equal));

        let working_http = self.http_judges.iter().filter(|j| j.is_working).count();
        let working_smtp = self.smtp_judges.iter().filter(|j| j.is_working).count();

        log::info!("✅ Judges pré-testés: {} HTTP + {} SMTP working", working_http, working_smtp);

        if working_http == 0 {
            log::warn!("⚠️  Aucun judge HTTP disponible - utilisation en mode dégradé");
        }
    }

    // Test ultra-rapide d'un judge
    async fn test_judge_fast(judge: &mut JudgeInfo, client: &Client<HttpsConnector<HttpConnector>, Empty<bytes::Bytes>>, real_ext_ip: &str) -> bool {
        let start_time = std::time::Instant::now();

        let request = Request::builder()
            .uri(&judge.url)
            .header("User-Agent", random_useragent(false))
            .header("Accept", "*/*")
            .header("Connection", "keep-alive")
            .body(Empty::new())
            .unwrap();

        let task = timeout(
            Duration::from_millis(2000), // Timeout 2s maximum
            client.request(request),
        );

        match task.await {
            Ok(Ok(response)) => {
                if StatusCode::OK == response.status() {
                    if let Ok(body) = response.collect().await {
                        let body_bytes = body.to_bytes();
                        let body_str = String::from_utf8_lossy(&body_bytes);

                        let response_time = start_time.elapsed();
                        judge.response_time = response_time;

                        // Vérifie si le judge peut détecter l'IP réelle
                        judge.is_working = if judge.url.contains("/ip") {
                            // Pour les services IP simples, juste vérifier qu'on reçoit une réponse
                            body_str.trim().len() > 7 && body_str.contains('.')
                        } else {
                            // Pour les judges complets, vérifier la détection d'IP
                            body_str.to_lowercase().contains(&real_ext_ip.to_lowercase())
                        };

                        if judge.is_working {
                            judge.marks.insert("via".into(), body_str.matches("via").count());
                            judge.marks.insert("proxy".into(), body_str.matches("proxy").count());
                            judge.success_rate = 1.0;
                        }

                        log::debug!("Judge {} testé en {}ms - Working: {}",
                                   judge.host, response_time.as_millis(), judge.is_working);
                    }
                }
            }
            Ok(Err(err)) => {
                log::debug!("Judge {} erreur: {}", judge.host, err);
                judge.is_working = false;
            }
            Err(_) => {
                log::debug!("Judge {} timeout", judge.host);
                judge.is_working = false;
            }
        }

        judge.is_working
    }

    // Obtenir le meilleur judge disponible pour un protocole
    pub async fn get_best_judge(&self, protocol: &str) -> Option<&JudgeInfo> {
        match protocol.to_uppercase().as_str() {
            "HTTP" | "HTTPS" | "CONNECT:80" => {
                self.http_judges.iter().find(|j| j.is_working)
            }
            "SMTP" | "CONNECT:25" => {
                self.smtp_judges.iter().find(|j| j.is_working)
            }
            _ => None,
        }
    }

    // Obtenir plusieurs judges pour load balancing
    pub async fn get_working_judges(&self, protocol: &str, count: usize) -> Vec<&JudgeInfo> {
        let judges = match protocol.to_uppercase().as_str() {
            "HTTP" | "HTTPS" | "CONNECT:80" => &self.http_judges,
            "SMTP" | "CONNECT:25" => &self.smtp_judges,
            _ => return Vec::new(),
        };

        judges.iter()
            .filter(|j| j.is_working)
            .take(count)
            .collect()
    }

    // Statistiques des judges
    pub fn get_stats(&self) -> JudgeStats {
        let http_working = self.http_judges.iter().filter(|j| j.is_working).count();
        let smtp_working = self.smtp_judges.iter().filter(|j| j.is_working).count();

        let avg_response_time = if http_working > 0 {
            let total: Duration = self.http_judges.iter()
                .filter(|j| j.is_working)
                .map(|j| j.response_time)
                .sum();
            total / http_working as u32
        } else {
            Duration::from_millis(0)
        };

        JudgeStats {
            http_total: self.http_judges.len(),
            http_working,
            smtp_total: self.smtp_judges.len(),
            smtp_working,
            avg_response_time_ms: avg_response_time.as_millis(),
        }
    }
}

#[derive(Debug)]
pub struct JudgeStats {
    pub http_total: usize,
    pub http_working: usize,
    pub smtp_total: usize,
    pub smtp_working: usize,
    pub avg_response_time_ms: u128,
}

impl std::fmt::Display for JudgeStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Judges: HTTP {}/{} ({}ms avg) | SMTP {}/{}",
               self.http_working, self.http_total, self.avg_response_time_ms,
               self.smtp_working, self.smtp_total)
    }
}

// Trait d'extension pour les slices
trait ChooseRandom<T> {
    fn choose<R: rand::Rng + ?Sized>(&self, rng: &mut R) -> Option<&T>;
}

impl<T> ChooseRandom<T> for [T] {
    fn choose<R: rand::Rng + ?Sized>(&self, rng: &mut R) -> Option<&T> {
        if self.is_empty() {
            None
        } else {
            Some(&self[rng.gen_range(0..self.len())])
        }
    }
}