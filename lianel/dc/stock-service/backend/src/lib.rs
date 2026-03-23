//! Stock service – library. Minimal: health, status, /me (Keycloak SSO).

pub mod app;
pub mod auth;
pub mod config;
pub mod daily_strategy;
pub mod ibkr;
pub mod paper_trade;
pub mod today_cache;
pub mod watchlist;
