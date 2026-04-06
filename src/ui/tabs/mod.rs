mod about_tab;
mod configure_tab;
mod console_tab;
mod results_tab;

#[derive(PartialEq)]
pub enum Tab {
    Configure,
    Results,
    Console,
    About,
}

pub use about_tab::AboutTab;
pub use configure_tab::{ConfigureAction, ConfigureTab};
pub use console_tab::ConsoleTab;
pub use results_tab::ResultsTab;
