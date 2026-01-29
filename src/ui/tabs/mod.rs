mod configure_tab;
mod results_tab;
mod console_tab;
mod about_tab;

#[derive(PartialEq)]
pub enum Tab {
    Configure,
    Results,
    Console,
    About,
}

pub use configure_tab::{ConfigureTab, ConfigureAction};
pub use results_tab::ResultsTab;
pub use console_tab::ConsoleTab;
pub use about_tab::AboutTab;