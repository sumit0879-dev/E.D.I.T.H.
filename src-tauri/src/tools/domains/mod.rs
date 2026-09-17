pub mod browser;
pub mod computer;
pub mod computer_platform;

#[cfg(test)]
mod browser_tests;
#[cfg(test)]
mod computer_tests;

pub use browser::{get_browser_definitions, BrowserDomainExecutor};
pub use computer::{get_computer_definitions, ComputerDomainExecutor};
