pub mod browser;

#[cfg(test)]
mod browser_tests;

pub use browser::{get_browser_definitions, BrowserDomainExecutor};
